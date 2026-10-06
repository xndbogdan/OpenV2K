//! Failure-ordered task-owner setup for class-32 `"Guard Location"`.
//!
//! This module implements only the statically closed initializer transaction
//! at `FUN_0040B5A0`:
//!
//! 1. copy the type-authored word at `+0x04` into the behavior's primary
//!    context;
//! 2. clear task slot 2;
//! 3. prepare and install `FUN_00401F80/00401FB0` in slot 1; and
//! 4. only after that succeeds, prepare and install the 5,000-ms
//!    `FUN_00402E20/00402EB0` companion in slot 0.
//!
//! Allocation/private-state preparation remains caller-owned. The detached
//! slot-1 acquisition callback and its first-eligible-candidate selector live
//! in [`acquisition`]. Process-shared RNG ownership, the seven-argument common
//! mover, component ownership, and live entity attachment remain deliberately
//! outside these seams.

pub mod acquisition;

use crate::actor_task_owner::{ActorTaskOwner, ActorTaskSlot, PreparedActorTask};

pub const GUARD_LOCATION_INITIALIZER_ADDRESS: u32 = 0x0040_B5A0;
pub const GUARD_LOCATION_STYLE_ADDRESS: u32 = 0x004C_7BB8;
pub const GUARD_LOCATION_NEXT_STYLE_ADDRESS: u32 = 0x004C_7C00;
pub const GUARD_LOCATION_SEARCH_CONSTRUCTOR_ADDRESS: u32 = 0x0040_1F80;
pub const GUARD_LOCATION_SEARCH_TICK_ADDRESS: u32 = 0x0040_1FB0;
pub const GUARD_LOCATION_SEARCH_FIXED_ARGUMENT: u32 = 0;
pub const GUARD_LOCATION_WANDER_CONSTRUCTOR_ADDRESS: u32 = 0x0040_2E20;
pub const GUARD_LOCATION_WANDER_TICK_ADDRESS: u32 = 0x0040_2EB0;
pub const GUARD_LOCATION_WANDER_LIFETIME_MS: u32 = 5_000;

/// The two distinct task families installed by the initial Guard program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardLocationTaskRole {
    AcquireCandidate,
    WanderAroundAnchor,
}

/// Proven callback identity and destination for one setup phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuardLocationTaskSpec {
    pub role: GuardLocationTaskRole,
    pub slot: ActorTaskSlot,
    pub constructor_address: u32,
    pub tick_address: u32,
}

const SEARCH_TASK: GuardLocationTaskSpec = GuardLocationTaskSpec {
    role: GuardLocationTaskRole::AcquireCandidate,
    slot: ActorTaskSlot::Secondary,
    constructor_address: GUARD_LOCATION_SEARCH_CONSTRUCTOR_ADDRESS,
    tick_address: GUARD_LOCATION_SEARCH_TICK_ADDRESS,
};

const WANDER_TASK: GuardLocationTaskSpec = GuardLocationTaskSpec {
    role: GuardLocationTaskRole::WanderAroundAnchor,
    slot: ActorTaskSlot::Primary,
    constructor_address: GUARD_LOCATION_WANDER_CONSTRUCTOR_ADDRESS,
    tick_address: GUARD_LOCATION_WANDER_TICK_ADDRESS,
};

const SETUP_PHASES: [GuardLocationTaskSpec; 2] = [SEARCH_TASK, WANDER_TASK];

/// The consumed behavior-context field written before any task mutation.
///
/// This intentionally models only the proven primary word rather than
/// inventing the layout of the remaining behavior-owned state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuardLocationPrimaryContext {
    word: u32,
}

impl GuardLocationPrimaryContext {
    pub const fn new(word: u32) -> Self {
        Self { word }
    }

    pub const fn word(self) -> u32 {
        self.word
    }
}

/// Inputs consumed by `FUN_0040B5A0`.
///
/// The second word is initializer argument 5. Retail forwards it through
/// `FUN_00401F80` into the search task's private state at `+0x28`; its deeper
/// semantics remain unresolved, so it must not be replaced by a zero sentinel
/// or assigned a guessed behavior meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuardLocationSetupRequest {
    pub authored_primary_context_word: u32,
    pub search_task_context_word: u32,
}

/// Exact phase-specific values passed after entity and destination slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardLocationTaskConstructorInputs {
    AcquireCandidate {
        fixed_argument: u32,
        search_task_context_word: u32,
    },
    WanderAroundAnchor {
        lifetime_ms: u32,
    },
}

/// One task whose allocation and private initialization is about to run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuardLocationTaskPreparation {
    pub phase_index: usize,
    pub task: GuardLocationTaskSpec,
    pub constructor_inputs: GuardLocationTaskConstructorInputs,
}

/// Preparation failure after every earlier retail-ordered mutation committed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardLocationSetupError<E> {
    pub phase_index: usize,
    pub slot: ActorTaskSlot,
    pub role: GuardLocationTaskRole,
    pub error: E,
}

/// Apply the closed task-table and primary-context slice of `FUN_0040B5A0`.
///
/// `prepare` models allocation plus task-family private initialization. A
/// failure preserves that phase's destination task, retains the context copy
/// and every earlier mutation, and prevents all later phases from running.
pub fn apply_guard_location_setup<T, E>(
    owner: &mut ActorTaskOwner<T>,
    primary_context: &mut GuardLocationPrimaryContext,
    request: GuardLocationSetupRequest,
    mut prepare: impl FnMut(GuardLocationTaskPreparation) -> Result<PreparedActorTask<T>, E>,
) -> Result<(), GuardLocationSetupError<E>> {
    primary_context.word = request.authored_primary_context_word;
    owner.clear_slot(ActorTaskSlot::Tertiary);

    for (phase_index, task) in SETUP_PHASES.into_iter().enumerate() {
        let preparation = GuardLocationTaskPreparation {
            phase_index,
            task,
            constructor_inputs: match task.role {
                GuardLocationTaskRole::AcquireCandidate => {
                    GuardLocationTaskConstructorInputs::AcquireCandidate {
                        fixed_argument: GUARD_LOCATION_SEARCH_FIXED_ARGUMENT,
                        search_task_context_word: request.search_task_context_word,
                    }
                }
                GuardLocationTaskRole::WanderAroundAnchor => {
                    GuardLocationTaskConstructorInputs::WanderAroundAnchor {
                        lifetime_ms: GUARD_LOCATION_WANDER_LIFETIME_MS,
                    }
                }
            },
        };
        let prepared = prepare(preparation).map_err(|error| GuardLocationSetupError {
            phase_index,
            slot: task.slot,
            role: task.role,
            error,
        })?;
        owner.replace_prepared(task.slot, prepared);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum InstalledTask {
        Old(ActorTaskSlot),
        Guard(GuardLocationTaskRole),
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
        preparation: GuardLocationTaskPreparation,
    ) -> Result<PreparedActorTask<InstalledTask>, &'static str> {
        Ok(PreparedActorTask::new(InstalledTask::Guard(
            preparation.task.role,
        )))
    }

    #[test]
    fn successful_setup_copies_context_clears_tertiary_then_installs_both_tasks() {
        let mut owner = seeded_owner();
        let mut context = GuardLocationPrimaryContext::new(0xAAAA_AAAA);
        let request = GuardLocationSetupRequest {
            authored_primary_context_word: 0x1234_5678,
            search_task_context_word: 0xCAFE_BABE,
        };
        let mut preparations = Vec::new();

        apply_guard_location_setup(&mut owner, &mut context, request, |preparation| {
            preparations.push(preparation);
            prepared(preparation)
        })
        .unwrap();

        assert_eq!(context.word(), 0x1234_5678);
        assert_eq!(
            preparations,
            [
                GuardLocationTaskPreparation {
                    phase_index: 0,
                    task: SEARCH_TASK,
                    constructor_inputs: GuardLocationTaskConstructorInputs::AcquireCandidate {
                        fixed_argument: 0,
                        search_task_context_word: 0xCAFE_BABE,
                    },
                },
                GuardLocationTaskPreparation {
                    phase_index: 1,
                    task: WANDER_TASK,
                    constructor_inputs: GuardLocationTaskConstructorInputs::WanderAroundAnchor {
                        lifetime_ms: 5_000,
                    },
                },
            ]
        );
        assert_eq!(
            installed(&owner, ActorTaskSlot::Primary),
            Some(InstalledTask::Guard(
                GuardLocationTaskRole::WanderAroundAnchor
            ))
        );
        assert_eq!(
            installed(&owner, ActorTaskSlot::Secondary),
            Some(InstalledTask::Guard(
                GuardLocationTaskRole::AcquireCandidate
            ))
        );
        assert_eq!(installed(&owner, ActorTaskSlot::Tertiary), None);
    }

    #[test]
    fn search_failure_commits_context_and_tertiary_clear_but_preserves_other_slots() {
        let mut owner = seeded_owner();
        let mut context = GuardLocationPrimaryContext::new(7);
        let request = GuardLocationSetupRequest {
            authored_primary_context_word: 11,
            search_task_context_word: 13,
        };
        let attempts = std::cell::Cell::new(0);

        let error = apply_guard_location_setup(&mut owner, &mut context, request, |_preparation| {
            attempts.set(attempts.get() + 1);
            Err::<PreparedActorTask<InstalledTask>, _>("search allocation")
        })
        .unwrap_err();

        assert_eq!(
            error,
            GuardLocationSetupError {
                phase_index: 0,
                slot: ActorTaskSlot::Secondary,
                role: GuardLocationTaskRole::AcquireCandidate,
                error: "search allocation",
            }
        );
        assert_eq!(attempts.get(), 1);
        assert_eq!(context.word(), 11);
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
    fn wander_failure_keeps_new_search_and_old_primary() {
        let mut owner = seeded_owner();
        let mut context = GuardLocationPrimaryContext::new(7);
        let request = GuardLocationSetupRequest {
            authored_primary_context_word: 11,
            search_task_context_word: 13,
        };
        let mut attempts = Vec::new();

        let error = apply_guard_location_setup(&mut owner, &mut context, request, |preparation| {
            attempts.push(preparation.task.role);
            if preparation.task.role == GuardLocationTaskRole::WanderAroundAnchor {
                Err("wander allocation")
            } else {
                prepared(preparation)
            }
        })
        .unwrap_err();

        assert_eq!(attempts.len(), 2);
        assert_eq!(
            error,
            GuardLocationSetupError {
                phase_index: 1,
                slot: ActorTaskSlot::Primary,
                role: GuardLocationTaskRole::WanderAroundAnchor,
                error: "wander allocation",
            }
        );
        assert_eq!(context.word(), 11);
        assert_eq!(
            installed(&owner, ActorTaskSlot::Primary),
            Some(InstalledTask::Old(ActorTaskSlot::Primary))
        );
        assert_eq!(
            installed(&owner, ActorTaskSlot::Secondary),
            Some(InstalledTask::Guard(
                GuardLocationTaskRole::AcquireCandidate
            ))
        );
        assert_eq!(installed(&owner, ActorTaskSlot::Tertiary), None);
    }
}
