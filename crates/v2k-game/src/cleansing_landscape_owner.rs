//! Failure-ordered task-owner setup for `"Cleansing Landscape"`.
//!
//! Static recovery from the repository retail executable and the accepted
//! passive lifecycle agree on a topology which is similar to, but deliberately
//! not interchangeable with, `"Defecate Virus"`. In `FUN_0040AD50`, retail
//! instructions `0x0040AD6E..0x0040AD7F` push mode 6, payload 0, the type's
//! `+0xA2` lifetime, slot 2, and the owner before calling `FUN_00402820`.
//! `0x0040AD87..0x0040ADA1` return immediately on failure; otherwise they clear
//! slot 1 and then call `FUN_00402FB0(owner, 0, 0)`. `FUN_00402FB0` itself
//! installs tick `0x00403040` and the callback tuple retained below, publishing
//! slot 0 through `FUN_0040A7A0` only after `FUN_00406030` succeeds. Therefore:
//!
//! 1. prepare and publish the mode-6 terrain-contact task in slot 2;
//! 2. clear slot 1;
//! 3. prepare the zero-lifetime `FUN_00403040` movement task for slot 0; and
//! 4. publish that movement task only when its preparation succeeds.
//!
//! Earlier mutations remain committed when a later preparation fails. This
//! module retains only that closed transaction; selector/materializer
//! ownership and live player attachment remain outside the boundary.

use crate::actor_task_owner::{ActorTaskOwner, ActorTaskSlot, PreparedActorTask};
use crate::defecate_virus::TerrainContactMode;

pub const CLEANSING_LANDSCAPE_NAME_ADDRESS: u32 = 0x004C_8FC0;
pub const CLEANSING_LANDSCAPE_BEHAVIOR_DESCRIPTOR_ADDRESS: u32 = 0x004C_8988;
pub const CLEANSING_LANDSCAPE_PROTOTYPE_ADDRESS: u32 = 0x004C_85D8;
pub const CLEANSING_LANDSCAPE_INITIALIZER_ADDRESS: u32 = 0x0040_AD50;

pub const CLEANSING_LANDSCAPE_TERRAIN_PREPARE_ADDRESS: u32 = 0x0040_2820;
pub const CLEANSING_LANDSCAPE_TERRAIN_TICK_ADDRESS: u32 = 0x0040_2850;
pub const CLEANSING_LANDSCAPE_TERRAIN_PAYLOAD: u16 = 0;
pub const CLEANSING_LANDSCAPE_TERRAIN_MODE: u16 = TerrainContactMode::Cleanse as u16;
pub const CLEANSING_LANDSCAPE_TERRAIN_PACKED_MODE: u32 = TerrainContactMode::Cleanse.packed();

pub const CLEANSING_LANDSCAPE_MOVEMENT_PREPARE_ADDRESS: u32 = 0x0040_2FB0;
pub const CLEANSING_LANDSCAPE_MOVEMENT_TICK_ADDRESS: u32 = 0x0040_3040;
pub const CLEANSING_LANDSCAPE_MOVEMENT_LIFETIME_MS: u32 = 0;
pub const CLEANSING_LANDSCAPE_MOVEMENT_INITIALIZER_ADDRESS: u32 = 0x0040_1350;
pub const CLEANSING_LANDSCAPE_MOVEMENT_DESTRUCTOR_ADDRESS: u32 = 0x0040_7120;
pub const CLEANSING_LANDSCAPE_MOVEMENT_PAIR_ADDRESS: u32 = 0x0040_2DA0;
pub const CLEANSING_LANDSCAPE_MOVEMENT_AUX_ADDRESS: u32 = 0x0040_2CA0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleansingLandscapeTaskRole {
    TerrainCleansing,
    TerrainSeekingMovement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CleansingLandscapeTaskSpec {
    pub role: CleansingLandscapeTaskRole,
    pub slot: ActorTaskSlot,
    pub prepare_address: u32,
    pub tick_address: u32,
}

pub const CLEANSING_LANDSCAPE_TERRAIN_TASK: CleansingLandscapeTaskSpec =
    CleansingLandscapeTaskSpec {
        role: CleansingLandscapeTaskRole::TerrainCleansing,
        slot: ActorTaskSlot::Tertiary,
        prepare_address: CLEANSING_LANDSCAPE_TERRAIN_PREPARE_ADDRESS,
        tick_address: CLEANSING_LANDSCAPE_TERRAIN_TICK_ADDRESS,
    };

pub const CLEANSING_LANDSCAPE_MOVEMENT_TASK: CleansingLandscapeTaskSpec =
    CleansingLandscapeTaskSpec {
        role: CleansingLandscapeTaskRole::TerrainSeekingMovement,
        slot: ActorTaskSlot::Primary,
        prepare_address: CLEANSING_LANDSCAPE_MOVEMENT_PREPARE_ADDRESS,
        tick_address: CLEANSING_LANDSCAPE_MOVEMENT_TICK_ADDRESS,
    };

/// Exact type-owned value consumed from the live record at `+0xA2`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CleansingLandscapeSetupRequest {
    pub terrain_task_lifetime_ms: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleansingLandscapeTaskConstructorInputs {
    TerrainCleansing {
        lifetime_ms: u16,
        payload: u16,
        mode: u16,
        packed_mode: u32,
    },
    TerrainSeekingMovement {
        lifetime_ms: u32,
        initializer_address: u32,
        destructor_address: u32,
        pair_address: u32,
        auxiliary_address: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CleansingLandscapeTaskPreparation {
    pub phase_index: usize,
    pub task: CleansingLandscapeTaskSpec,
    pub constructor_inputs: CleansingLandscapeTaskConstructorInputs,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleansingLandscapeSetupError<E> {
    pub phase_index: usize,
    pub slot: ActorTaskSlot,
    pub role: CleansingLandscapeTaskRole,
    pub error: E,
}

/// Apply the closed task-table slice of `FUN_0040AD50`.
///
/// Terrain preparation is the first fallible operation, so its failure leaves
/// all three old slots intact. Once it succeeds, slot 2 is visible before slot
/// 1 is cleared. A later movement-preparation failure therefore retains the
/// new cleansing task and cleared slot 1 while preserving the old slot 0.
pub fn apply_cleansing_landscape_setup<T, E>(
    owner: &mut ActorTaskOwner<T>,
    request: CleansingLandscapeSetupRequest,
    mut prepare: impl FnMut(CleansingLandscapeTaskPreparation) -> Result<PreparedActorTask<T>, E>,
) -> Result<(), CleansingLandscapeSetupError<E>> {
    let terrain_preparation = CleansingLandscapeTaskPreparation {
        phase_index: 0,
        task: CLEANSING_LANDSCAPE_TERRAIN_TASK,
        constructor_inputs: CleansingLandscapeTaskConstructorInputs::TerrainCleansing {
            lifetime_ms: request.terrain_task_lifetime_ms,
            payload: CLEANSING_LANDSCAPE_TERRAIN_PAYLOAD,
            mode: CLEANSING_LANDSCAPE_TERRAIN_MODE,
            packed_mode: CLEANSING_LANDSCAPE_TERRAIN_PACKED_MODE,
        },
    };
    let terrain_task =
        prepare(terrain_preparation).map_err(|error| CleansingLandscapeSetupError {
            phase_index: 0,
            slot: ActorTaskSlot::Tertiary,
            role: CleansingLandscapeTaskRole::TerrainCleansing,
            error,
        })?;
    owner.replace_prepared(ActorTaskSlot::Tertiary, terrain_task);

    owner.clear_slot(ActorTaskSlot::Secondary);

    let movement_preparation = CleansingLandscapeTaskPreparation {
        phase_index: 1,
        task: CLEANSING_LANDSCAPE_MOVEMENT_TASK,
        constructor_inputs: CleansingLandscapeTaskConstructorInputs::TerrainSeekingMovement {
            lifetime_ms: CLEANSING_LANDSCAPE_MOVEMENT_LIFETIME_MS,
            initializer_address: CLEANSING_LANDSCAPE_MOVEMENT_INITIALIZER_ADDRESS,
            destructor_address: CLEANSING_LANDSCAPE_MOVEMENT_DESTRUCTOR_ADDRESS,
            pair_address: CLEANSING_LANDSCAPE_MOVEMENT_PAIR_ADDRESS,
            auxiliary_address: CLEANSING_LANDSCAPE_MOVEMENT_AUX_ADDRESS,
        },
    };
    let movement_task =
        prepare(movement_preparation).map_err(|error| CleansingLandscapeSetupError {
            phase_index: 1,
            slot: ActorTaskSlot::Primary,
            role: CleansingLandscapeTaskRole::TerrainSeekingMovement,
            error,
        })?;
    owner.replace_prepared(ActorTaskSlot::Primary, movement_task);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum InstalledTask {
        Old(ActorTaskSlot),
        Cleansing(CleansingLandscapeTaskRole),
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
        preparation: CleansingLandscapeTaskPreparation,
    ) -> Result<PreparedActorTask<InstalledTask>, &'static str> {
        Ok(PreparedActorTask::new(InstalledTask::Cleansing(
            preparation.task.role,
        )))
    }

    #[test]
    fn success_uses_exact_distinct_task_topology_and_arguments() {
        let mut owner = seeded_owner();
        let mut attempts = Vec::new();

        apply_cleansing_landscape_setup(
            &mut owner,
            CleansingLandscapeSetupRequest {
                terrain_task_lifetime_ms: 73,
            },
            |preparation| {
                attempts.push(preparation);
                prepared(preparation)
            },
        )
        .unwrap();

        assert_eq!(
            attempts,
            [
                CleansingLandscapeTaskPreparation {
                    phase_index: 0,
                    task: CLEANSING_LANDSCAPE_TERRAIN_TASK,
                    constructor_inputs: CleansingLandscapeTaskConstructorInputs::TerrainCleansing {
                        lifetime_ms: 73,
                        payload: 0,
                        mode: 6,
                        packed_mode: 0x0006_0000,
                    },
                },
                CleansingLandscapeTaskPreparation {
                    phase_index: 1,
                    task: CLEANSING_LANDSCAPE_MOVEMENT_TASK,
                    constructor_inputs:
                        CleansingLandscapeTaskConstructorInputs::TerrainSeekingMovement {
                            lifetime_ms: 0,
                            initializer_address: 0x0040_1350,
                            destructor_address: 0x0040_7120,
                            pair_address: 0x0040_2DA0,
                            auxiliary_address: 0x0040_2CA0,
                        },
                },
            ]
        );
        assert_eq!(
            installed(&owner, ActorTaskSlot::Primary),
            Some(InstalledTask::Cleansing(
                CleansingLandscapeTaskRole::TerrainSeekingMovement
            ))
        );
        assert_eq!(installed(&owner, ActorTaskSlot::Secondary), None);
        assert_eq!(
            installed(&owner, ActorTaskSlot::Tertiary),
            Some(InstalledTask::Cleansing(
                CleansingLandscapeTaskRole::TerrainCleansing
            ))
        );
    }

    #[test]
    fn terrain_failure_preserves_all_old_slots() {
        let mut owner = seeded_owner();
        let error = apply_cleansing_landscape_setup(
            &mut owner,
            CleansingLandscapeSetupRequest {
                terrain_task_lifetime_ms: 73,
            },
            |_preparation| Err::<PreparedActorTask<InstalledTask>, _>("terrain allocation"),
        )
        .unwrap_err();

        assert_eq!(
            error,
            CleansingLandscapeSetupError {
                phase_index: 0,
                slot: ActorTaskSlot::Tertiary,
                role: CleansingLandscapeTaskRole::TerrainCleansing,
                error: "terrain allocation",
            }
        );
        for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
            assert_eq!(installed(&owner, slot), Some(InstalledTask::Old(slot)));
        }
    }

    #[test]
    fn movement_failure_keeps_new_terrain_and_secondary_clear() {
        let mut owner = seeded_owner();
        let mut attempts = Vec::new();
        let error = apply_cleansing_landscape_setup(
            &mut owner,
            CleansingLandscapeSetupRequest {
                terrain_task_lifetime_ms: 73,
            },
            |preparation| {
                attempts.push(preparation.task.role);
                if preparation.task.role == CleansingLandscapeTaskRole::TerrainSeekingMovement {
                    Err("movement allocation")
                } else {
                    prepared(preparation)
                }
            },
        )
        .unwrap_err();

        assert_eq!(
            attempts,
            [
                CleansingLandscapeTaskRole::TerrainCleansing,
                CleansingLandscapeTaskRole::TerrainSeekingMovement,
            ]
        );
        assert_eq!(
            error,
            CleansingLandscapeSetupError {
                phase_index: 1,
                slot: ActorTaskSlot::Primary,
                role: CleansingLandscapeTaskRole::TerrainSeekingMovement,
                error: "movement allocation",
            }
        );
        assert_eq!(
            installed(&owner, ActorTaskSlot::Primary),
            Some(InstalledTask::Old(ActorTaskSlot::Primary))
        );
        assert_eq!(installed(&owner, ActorTaskSlot::Secondary), None);
        assert_eq!(
            installed(&owner, ActorTaskSlot::Tertiary),
            Some(InstalledTask::Cleansing(
                CleansingLandscapeTaskRole::TerrainCleansing
            ))
        );
    }
}
