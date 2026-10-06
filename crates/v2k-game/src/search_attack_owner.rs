//! Mutation-safe task-owner bridge for class-7 `"Search And Attack Target"`.
//!
//! The behavior's three variants have different, failure-sensitive task setup
//! orders. This module applies only that closed owner transaction. Allocation,
//! task-private initialization, movement, targeting callbacks, firing, audio,
//! and live entity attachment remain caller-owned.

use crate::{
    actor_task_owner::{ActorTaskOwner, ActorTaskSlot, PreparedActorTask},
    search_attack::{
        search_attack_variant_setup, SearchAttackTaskRole, SearchAttackTaskSpec,
        SearchAttackVariant,
    },
};

/// Variant-specific input required before the task-owner transaction begins.
///
/// A pursuing setup always carries its target explicitly. This prevents a
/// missing target from being represented by a numeric sentinel or discovered
/// only after earlier task mutations have committed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchAttackTaskSetupRequest {
    Acquiring,
    Pursuing { target_id: u32 },
    ExternalEvent,
}

impl SearchAttackTaskSetupRequest {
    pub const fn variant(self) -> SearchAttackVariant {
        match self {
            Self::Acquiring => SearchAttackVariant::Acquiring,
            Self::Pursuing { .. } => SearchAttackVariant::Pursuing,
            Self::ExternalEvent => SearchAttackVariant::ExternalEvent,
        }
    }
}

/// One task whose allocation/private initialization is about to be attempted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchAttackTaskPreparation {
    pub request: SearchAttackTaskSetupRequest,
    pub phase_index: usize,
    pub task: SearchAttackTaskSpec,
}

/// Preparation failure after every earlier retail-ordered mutation committed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchAttackTaskSetupError<E> {
    pub phase_index: usize,
    pub slot: ActorTaskSlot,
    pub role: SearchAttackTaskRole,
    pub error: E,
}

/// Apply the task-table slice of the selected class-7 initializer.
///
/// Each `prepare` call models allocation plus task-private initialization. It
/// runs before the destination task is destroyed. On failure, that destination
/// remains intact, mutations from earlier phases remain committed, and no
/// later clear or install is attempted.
///
/// The caller must perform any non-task prelude (including the pursuing
/// variant's optional authored sound) before entering this function. No live
/// callback is invoked here.
pub fn apply_search_attack_task_setup<T, E>(
    owner: &mut ActorTaskOwner<T>,
    request: SearchAttackTaskSetupRequest,
    mut prepare: impl FnMut(SearchAttackTaskPreparation) -> Result<PreparedActorTask<T>, E>,
) -> Result<(), SearchAttackTaskSetupError<E>> {
    let setup = search_attack_variant_setup(request.variant());

    for &slot in setup.initial_clear_slots {
        owner.clear_slot(actor_task_slot(slot));
    }

    for (phase_index, phase) in setup.ordered_phases.iter().enumerate() {
        for &slot in phase.clear_slots_before {
            owner.clear_slot(actor_task_slot(slot));
        }

        let slot = actor_task_slot(phase.install.slot);
        let preparation = SearchAttackTaskPreparation {
            request,
            phase_index,
            task: phase.install,
        };
        let prepared = prepare(preparation).map_err(|error| SearchAttackTaskSetupError {
            phase_index,
            slot,
            role: phase.install.role,
            error,
        })?;
        owner.replace_prepared(slot, prepared);
    }

    Ok(())
}

fn actor_task_slot(slot: u8) -> ActorTaskSlot {
    match slot {
        0 => ActorTaskSlot::Primary,
        1 => ActorTaskSlot::Secondary,
        2 => ActorTaskSlot::Tertiary,
        _ => panic!("authenticated Search And Attack task uses invalid slot {slot}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum InstalledTask {
        Old(ActorTaskSlot),
        SearchAttack {
            request: SearchAttackTaskSetupRequest,
            role: SearchAttackTaskRole,
        },
    }

    fn seeded_owner() -> ActorTaskOwner<InstalledTask> {
        let mut owner = ActorTaskOwner::new();
        for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
            owner.replace_prepared(slot, PreparedActorTask::new(InstalledTask::Old(slot)));
        }
        owner
    }

    fn installed(
        owner: &ActorTaskOwner<InstalledTask>,
        slot: ActorTaskSlot,
    ) -> Option<InstalledTask> {
        owner.state_in_slot(slot).copied()
    }

    fn prepared(
        preparation: SearchAttackTaskPreparation,
    ) -> Result<PreparedActorTask<InstalledTask>, &'static str> {
        Ok(PreparedActorTask::new(InstalledTask::SearchAttack {
            request: preparation.request,
            role: preparation.task.role,
        }))
    }

    #[test]
    fn acquiring_success_clears_tertiary_then_installs_search_and_wander() {
        let mut owner = seeded_owner();
        let mut preparations = Vec::new();

        apply_search_attack_task_setup(
            &mut owner,
            SearchAttackTaskSetupRequest::Acquiring,
            |preparation| {
                preparations.push(preparation);
                prepared(preparation)
            },
        )
        .unwrap();

        assert_eq!(
            preparations
                .iter()
                .map(|preparation| preparation.task.role)
                .collect::<Vec<_>>(),
            [
                SearchAttackTaskRole::AcquireTarget,
                SearchAttackTaskRole::Wander
            ]
        );
        assert_eq!(
            installed(&owner, ActorTaskSlot::Primary),
            Some(InstalledTask::SearchAttack {
                request: SearchAttackTaskSetupRequest::Acquiring,
                role: SearchAttackTaskRole::Wander,
            })
        );
        assert_eq!(
            installed(&owner, ActorTaskSlot::Secondary),
            Some(InstalledTask::SearchAttack {
                request: SearchAttackTaskSetupRequest::Acquiring,
                role: SearchAttackTaskRole::AcquireTarget,
            })
        );
        assert_eq!(installed(&owner, ActorTaskSlot::Tertiary), None);
    }

    #[test]
    fn acquiring_search_failure_commits_only_the_initial_tertiary_clear() {
        let mut owner = seeded_owner();

        let error = apply_search_attack_task_setup(
            &mut owner,
            SearchAttackTaskSetupRequest::Acquiring,
            |_preparation| Err::<PreparedActorTask<InstalledTask>, _>("search allocation"),
        )
        .unwrap_err();

        assert_eq!(
            error,
            SearchAttackTaskSetupError {
                phase_index: 0,
                slot: ActorTaskSlot::Secondary,
                role: SearchAttackTaskRole::AcquireTarget,
                error: "search allocation",
            }
        );
        assert_eq!(
            installed(&owner, ActorTaskSlot::Primary),
            Some(InstalledTask::Old(ActorTaskSlot::Primary))
        );
        assert_eq!(
            installed(&owner, ActorTaskSlot::Secondary),
            Some(InstalledTask::Old(ActorTaskSlot::Secondary))
        );
        assert_eq!(installed(&owner, ActorTaskSlot::Tertiary), None);
    }

    #[test]
    fn acquiring_wander_failure_keeps_old_primary_after_search_install() {
        let mut owner = seeded_owner();

        let error = apply_search_attack_task_setup(
            &mut owner,
            SearchAttackTaskSetupRequest::Acquiring,
            |preparation| {
                if preparation.task.role == SearchAttackTaskRole::Wander {
                    Err("wander allocation")
                } else {
                    prepared(preparation)
                }
            },
        )
        .unwrap_err();

        assert_eq!(error.phase_index, 1);
        assert_eq!(error.slot, ActorTaskSlot::Primary);
        assert_eq!(error.role, SearchAttackTaskRole::Wander);
        assert_eq!(
            installed(&owner, ActorTaskSlot::Primary),
            Some(InstalledTask::Old(ActorTaskSlot::Primary))
        );
        assert_eq!(
            installed(&owner, ActorTaskSlot::Secondary),
            Some(InstalledTask::SearchAttack {
                request: SearchAttackTaskSetupRequest::Acquiring,
                role: SearchAttackTaskRole::AcquireTarget,
            })
        );
        assert_eq!(installed(&owner, ActorTaskSlot::Tertiary), None);
    }

    #[test]
    fn pursuing_success_installs_fire_then_clears_search_then_installs_chase() {
        let mut owner = seeded_owner();
        let request = SearchAttackTaskSetupRequest::Pursuing {
            target_id: 0x047f_0001,
        };

        apply_search_attack_task_setup(&mut owner, request, prepared).unwrap();

        assert_eq!(
            installed(&owner, ActorTaskSlot::Primary),
            Some(InstalledTask::SearchAttack {
                request,
                role: SearchAttackTaskRole::PursueTarget,
            })
        );
        assert_eq!(installed(&owner, ActorTaskSlot::Secondary), None);
        assert_eq!(
            installed(&owner, ActorTaskSlot::Tertiary),
            Some(InstalledTask::SearchAttack {
                request,
                role: SearchAttackTaskRole::AimAndFire,
            })
        );
    }

    #[test]
    fn pursuing_fire_failure_preserves_all_old_slots() {
        let mut owner = seeded_owner();

        let error = apply_search_attack_task_setup(
            &mut owner,
            SearchAttackTaskSetupRequest::Pursuing { target_id: 9 },
            |_preparation| Err::<PreparedActorTask<InstalledTask>, _>("fire allocation"),
        )
        .unwrap_err();

        assert_eq!(error.phase_index, 0);
        assert_eq!(error.slot, ActorTaskSlot::Tertiary);
        for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
            assert_eq!(installed(&owner, slot), Some(InstalledTask::Old(slot)));
        }
    }

    #[test]
    fn pursuing_chase_failure_keeps_fire_and_old_primary_but_commits_search_clear() {
        let mut owner = seeded_owner();
        let request = SearchAttackTaskSetupRequest::Pursuing { target_id: 9 };

        let error = apply_search_attack_task_setup(&mut owner, request, |preparation| {
            if preparation.task.role == SearchAttackTaskRole::PursueTarget {
                Err("chase allocation")
            } else {
                prepared(preparation)
            }
        })
        .unwrap_err();

        assert_eq!(error.phase_index, 1);
        assert_eq!(error.slot, ActorTaskSlot::Primary);
        assert_eq!(
            installed(&owner, ActorTaskSlot::Primary),
            Some(InstalledTask::Old(ActorTaskSlot::Primary))
        );
        assert_eq!(installed(&owner, ActorTaskSlot::Secondary), None);
        assert_eq!(
            installed(&owner, ActorTaskSlot::Tertiary),
            Some(InstalledTask::SearchAttack {
                request,
                role: SearchAttackTaskRole::AimAndFire,
            })
        );
    }

    #[test]
    fn external_event_failure_commits_both_clears_and_preserves_primary() {
        let mut owner = seeded_owner();

        let error = apply_search_attack_task_setup(
            &mut owner,
            SearchAttackTaskSetupRequest::ExternalEvent,
            |_preparation| Err::<PreparedActorTask<InstalledTask>, _>("event allocation"),
        )
        .unwrap_err();

        assert_eq!(error.phase_index, 0);
        assert_eq!(error.slot, ActorTaskSlot::Primary);
        assert_eq!(error.role, SearchAttackTaskRole::ExternalEvent);
        assert_eq!(
            installed(&owner, ActorTaskSlot::Primary),
            Some(InstalledTask::Old(ActorTaskSlot::Primary))
        );
        assert_eq!(installed(&owner, ActorTaskSlot::Secondary), None);
        assert_eq!(installed(&owner, ActorTaskSlot::Tertiary), None);
    }

    #[test]
    fn external_event_success_replaces_primary_after_both_clears() {
        let mut owner = seeded_owner();

        apply_search_attack_task_setup(
            &mut owner,
            SearchAttackTaskSetupRequest::ExternalEvent,
            prepared,
        )
        .unwrap();

        assert_eq!(
            installed(&owner, ActorTaskSlot::Primary),
            Some(InstalledTask::SearchAttack {
                request: SearchAttackTaskSetupRequest::ExternalEvent,
                role: SearchAttackTaskRole::ExternalEvent,
            })
        );
        assert_eq!(installed(&owner, ActorTaskSlot::Secondary), None);
        assert_eq!(installed(&owner, ActorTaskSlot::Tertiary), None);
    }
}
