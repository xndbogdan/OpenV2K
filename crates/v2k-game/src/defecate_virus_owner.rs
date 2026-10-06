//! Failure-ordered task-owner setup for class-4 `"Defecate Virus"`.
//!
//! `FUN_0040B9E0` owns a deliberately partial transaction:
//!
//! 1. clear slot 1;
//! 2. prepare and install the mode-5 terrain-contact task in slot 2;
//! 3. prepare the 2,000-ms wander task for slot 0;
//! 4. after successful wander preparation, reset authored Sub-A target speed
//!    to literal one; and
//! 5. publish the prepared wander task.
//!
//! Earlier mutations remain committed when a later preparation fails. This
//! detached owner seam preserves that ordering without claiming the unresolved
//! common-mover integration or live behavior attachment.

use crate::actor_task_owner::{ActorTaskOwner, ActorTaskSlot, PreparedActorTask};
use crate::common_mover::SubAPropulsionRuntime;
use crate::defecate_virus::TerrainContactMode;
pub use crate::defecate_virus::{
    DEFECATE_VIRUS_INITIALIZER_ADDRESS, DEFECATE_VIRUS_PROTOTYPE_ADDRESS,
    DEFECATE_VIRUS_WANDER_LIFETIME_MS, DEFECATE_VIRUS_WANDER_TICK_ADDRESS,
};

pub const DEFECATE_VIRUS_NAME_ADDRESS: u32 = 0x004C_9258;
pub const DEFECATE_VIRUS_TERRAIN_PREPARE_ADDRESS: u32 = 0x0040_2820;
pub const DEFECATE_VIRUS_TERRAIN_TICK_ADDRESS: u32 = 0x0040_2850;
pub const DEFECATE_VIRUS_WANDER_PREPARE_ADDRESS: u32 = 0x0040_32A0;
pub const DEFECATE_VIRUS_WANDER_INITIALIZER_ADDRESS: u32 = 0x0040_1350;
pub const DEFECATE_VIRUS_WANDER_DESTRUCTOR_ADDRESS: u32 = 0x0040_7120;
pub const DEFECATE_VIRUS_WANDER_PAIR_ADDRESS: u32 = 0x0040_2DA0;
pub const DEFECATE_VIRUS_WANDER_AUX_ADDRESS: u32 = 0x0040_2CA0;
pub const DEFECATE_VIRUS_TERRAIN_PAYLOAD: u16 = 0;
pub const DEFECATE_VIRUS_TERRAIN_MODE: u16 = TerrainContactMode::Infect as u16;
pub const DEFECATE_VIRUS_TERRAIN_PACKED_MODE: u32 = TerrainContactMode::Infect.packed();

/// The two distinct task families installed by the behavior initializer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefecateVirusTaskRole {
    TerrainInfection,
    Wander,
}

/// Proven callback identity and destination for one setup phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefecateVirusTaskSpec {
    pub role: DefecateVirusTaskRole,
    pub slot: ActorTaskSlot,
    pub prepare_address: u32,
    pub tick_address: u32,
}

pub const DEFECATE_VIRUS_TERRAIN_TASK: DefecateVirusTaskSpec = DefecateVirusTaskSpec {
    role: DefecateVirusTaskRole::TerrainInfection,
    slot: ActorTaskSlot::Tertiary,
    prepare_address: DEFECATE_VIRUS_TERRAIN_PREPARE_ADDRESS,
    tick_address: DEFECATE_VIRUS_TERRAIN_TICK_ADDRESS,
};

pub const DEFECATE_VIRUS_WANDER_TASK: DefecateVirusTaskSpec = DefecateVirusTaskSpec {
    role: DefecateVirusTaskRole::Wander,
    slot: ActorTaskSlot::Primary,
    prepare_address: DEFECATE_VIRUS_WANDER_PREPARE_ADDRESS,
    tick_address: DEFECATE_VIRUS_WANDER_TICK_ADDRESS,
};

/// Exact type-owned value consumed by `FUN_0040B9E0`.
///
/// The word comes from the live type record at `+0xA2`. The generic task
/// scheduler mechanically uses it as slot 2's strict elapsed-ms threshold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefecateVirusSetupRequest {
    pub terrain_task_lifetime_ms: u16,
}

/// Exact phase-specific values passed after entity and destination slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefecateVirusTaskConstructorInputs {
    TerrainInfection {
        lifetime_ms: u16,
        payload: u16,
        mode: u16,
        packed_mode: u32,
    },
    Wander {
        lifetime_ms: u32,
        initializer_address: u32,
        destructor_address: u32,
        pair_address: u32,
        auxiliary_address: u32,
    },
}

/// One task whose allocation and private initialization is about to run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefecateVirusTaskPreparation {
    pub phase_index: usize,
    pub task: DefecateVirusTaskSpec,
    pub constructor_inputs: DefecateVirusTaskConstructorInputs,
}

/// Proven Section-12 Sub-A topology at the point the wander task publishes.
///
/// Absence is an authored, successful configuration. `RequiredUnresolved`
/// represents a port-side loader/integration failure and prevents publishing
/// the prepared wander task rather than silently skipping a retail-required
/// write.
pub enum DefecateVirusSubATopology<'a> {
    NotAuthored,
    Authored(&'a mut SubAPropulsionRuntime),
    RequiredUnresolved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefecateVirusSetupFailure<E> {
    Preparation(E),
    RequiredSubATopologyUnresolved,
}

/// Setup failure after every earlier retail-ordered mutation committed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefecateVirusSetupError<E> {
    pub phase_index: usize,
    pub slot: ActorTaskSlot,
    pub role: DefecateVirusTaskRole,
    pub failure: DefecateVirusSetupFailure<E>,
}

/// Apply the statically closed task-table slice of `FUN_0040B9E0`.
///
/// `prepare` models allocation plus task-family private initialization. A
/// failed terrain preparation preserves the old slots 2 and 0 after slot 1 was
/// cleared. A failed wander preparation preserves the newly installed terrain
/// task and old slot 0. The authored Sub-A write occurs only after wander
/// preparation succeeds and before that task becomes visible in slot 0.
pub fn apply_defecate_virus_setup<T, E>(
    owner: &mut ActorTaskOwner<T>,
    request: DefecateVirusSetupRequest,
    sub_a: DefecateVirusSubATopology<'_>,
    mut prepare: impl FnMut(DefecateVirusTaskPreparation) -> Result<PreparedActorTask<T>, E>,
) -> Result<(), DefecateVirusSetupError<E>> {
    // Existing Type16/26 preparers retain their bounded family-state policy:
    // clear1 -> prepare/publish2 -> prepare0 -> A literal1 -> publish0.
    // They do not supply406070's H/RNG/A suffix. That remains their explicit
    // integration boundary; delegating here does not grant native parity.
    apply_defecate_virus_setup_with_component_suffix(owner, request, sub_a, |phase, _| {
        prepare(phase)
    })
}

/// Same failure/publication matrix, with the real component topology borrowed
/// during each successful preparation. Native callers apply406070's ordered
/// H -> RNG -> A writes before the prepared wrapper becomes visible. The
/// final032A0 A literal1 write still belongs to this setup after phase1.
pub fn apply_defecate_virus_setup_with_component_suffix<'a, T, E>(
    owner: &mut ActorTaskOwner<T>,
    request: DefecateVirusSetupRequest,
    mut sub_a: DefecateVirusSubATopology<'a>,
    mut prepare: impl FnMut(
        DefecateVirusTaskPreparation,
        &mut DefecateVirusSubATopology<'a>,
    ) -> Result<PreparedActorTask<T>, E>,
) -> Result<(), DefecateVirusSetupError<E>> {
    owner.clear_slot(ActorTaskSlot::Secondary);

    let terrain_preparation = DefecateVirusTaskPreparation {
        phase_index: 0,
        task: DEFECATE_VIRUS_TERRAIN_TASK,
        constructor_inputs: DefecateVirusTaskConstructorInputs::TerrainInfection {
            lifetime_ms: request.terrain_task_lifetime_ms,
            payload: DEFECATE_VIRUS_TERRAIN_PAYLOAD,
            mode: DEFECATE_VIRUS_TERRAIN_MODE,
            packed_mode: DEFECATE_VIRUS_TERRAIN_PACKED_MODE,
        },
    };
    let terrain_task =
        prepare(terrain_preparation, &mut sub_a).map_err(|error| DefecateVirusSetupError {
            phase_index: 0,
            slot: ActorTaskSlot::Tertiary,
            role: DefecateVirusTaskRole::TerrainInfection,
            failure: DefecateVirusSetupFailure::Preparation(error),
        })?;
    owner.replace_prepared(ActorTaskSlot::Tertiary, terrain_task);

    let wander_preparation = DefecateVirusTaskPreparation {
        phase_index: 1,
        task: DEFECATE_VIRUS_WANDER_TASK,
        constructor_inputs: DefecateVirusTaskConstructorInputs::Wander {
            lifetime_ms: DEFECATE_VIRUS_WANDER_LIFETIME_MS,
            initializer_address: DEFECATE_VIRUS_WANDER_INITIALIZER_ADDRESS,
            destructor_address: DEFECATE_VIRUS_WANDER_DESTRUCTOR_ADDRESS,
            pair_address: DEFECATE_VIRUS_WANDER_PAIR_ADDRESS,
            auxiliary_address: DEFECATE_VIRUS_WANDER_AUX_ADDRESS,
        },
    };
    let wander_task =
        prepare(wander_preparation, &mut sub_a).map_err(|error| DefecateVirusSetupError {
            phase_index: 1,
            slot: ActorTaskSlot::Primary,
            role: DefecateVirusTaskRole::Wander,
            failure: DefecateVirusSetupFailure::Preparation(error),
        })?;

    match sub_a {
        DefecateVirusSubATopology::NotAuthored => {}
        DefecateVirusSubATopology::Authored(runtime) => {
            runtime.apply_defecate_virus_reset();
        }
        DefecateVirusSubATopology::RequiredUnresolved => {
            return Err(DefecateVirusSetupError {
                phase_index: 1,
                slot: ActorTaskSlot::Primary,
                role: DefecateVirusTaskRole::Wander,
                failure: DefecateVirusSetupFailure::RequiredSubATopologyUnresolved,
            });
        }
    }
    owner.replace_prepared(ActorTaskSlot::Primary, wander_task);
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::entity_collision_state::RetailRuntimeValue;

    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum InstalledTask {
        Old(ActorTaskSlot),
        Virus(DefecateVirusTaskRole),
    }

    fn seeded_owner() -> ActorTaskOwner<InstalledTask> {
        let mut owner = ActorTaskOwner::new();
        for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
            owner.replace_prepared(slot, PreparedActorTask::new(InstalledTask::Old(slot)));
        }
        owner
    }

    #[test]
    fn native_suffix_commits_before_each_publication_and_keeps_failure_prefix() {
        for fail_at in [Some(0), Some(1), None] {
            let mut owner = seeded_owner();
            let mut a =
                SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Known(99), -1, 73);
            let mut suffixes = Vec::new();
            let result = apply_defecate_virus_setup_with_component_suffix(
                &mut owner,
                DefecateVirusSetupRequest {
                    terrain_task_lifetime_ms: 2000,
                },
                DefecateVirusSubATopology::Authored(&mut a),
                |phase, topology| {
                    if fail_at == Some(phase.phase_index) {
                        return Err("allocation");
                    }
                    // A real406070 suffix finishes H/RNG/A before returning
                    // its prepared wrapper; the host never queues it later.
                    suffixes.push(phase.phase_index);
                    let DefecateVirusSubATopology::Authored(a) = topology else {
                        unreachable!();
                    };
                    a.set_direction_multiplier(1);
                    a.apply_shared_initializer_target_speed_write(100 + phase.phase_index as i32);
                    prepared(phase)
                },
            );
            assert_eq!(installed(&owner, ActorTaskSlot::Secondary), None);
            match fail_at {
                Some(0) => {
                    assert!(result.is_err());
                    assert!(suffixes.is_empty());
                    assert_eq!(
                        installed(&owner, ActorTaskSlot::Tertiary),
                        Some(InstalledTask::Old(ActorTaskSlot::Tertiary))
                    );
                    assert_eq!(a.target_speed_raw(), RetailRuntimeValue::Known(99));
                    assert_eq!(a.direction_multiplier(), -1);
                }
                Some(1) => {
                    assert!(result.is_err());
                    assert_eq!(suffixes, [0]);
                    assert_eq!(
                        installed(&owner, ActorTaskSlot::Tertiary),
                        Some(InstalledTask::Virus(
                            DefecateVirusTaskRole::TerrainInfection
                        ))
                    );
                    assert_eq!(
                        installed(&owner, ActorTaskSlot::Primary),
                        Some(InstalledTask::Old(ActorTaskSlot::Primary))
                    );
                    assert_eq!(a.target_speed_raw(), RetailRuntimeValue::Known(100));
                    assert_eq!(a.direction_multiplier(), 1);
                }
                None => {
                    assert!(result.is_ok());
                    assert_eq!(suffixes, [0, 1]);
                    assert_eq!(a.target_speed_raw(), RetailRuntimeValue::Known(1));
                    assert_eq!(
                        installed(&owner, ActorTaskSlot::Primary),
                        Some(InstalledTask::Virus(DefecateVirusTaskRole::Wander))
                    );
                }
                _ => unreachable!(),
            }
        }
    }

    fn installed(
        owner: &ActorTaskOwner<InstalledTask>,
        slot: ActorTaskSlot,
    ) -> Option<InstalledTask> {
        owner.state_in_slot(slot).copied()
    }

    fn prepared(
        preparation: DefecateVirusTaskPreparation,
    ) -> Result<PreparedActorTask<InstalledTask>, &'static str> {
        Ok(PreparedActorTask::new(InstalledTask::Virus(
            preparation.task.role,
        )))
    }

    #[test]
    fn success_uses_exact_order_arguments_and_sub_a_write() {
        let mut owner = seeded_owner();
        let mut sub_a =
            SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Known(99), -1, 73);
        let mut attempts = Vec::new();

        apply_defecate_virus_setup(
            &mut owner,
            DefecateVirusSetupRequest {
                terrain_task_lifetime_ms: 67,
            },
            DefecateVirusSubATopology::Authored(&mut sub_a),
            |preparation| {
                attempts.push(preparation);
                prepared(preparation)
            },
        )
        .unwrap();

        assert_eq!(
            attempts,
            [
                DefecateVirusTaskPreparation {
                    phase_index: 0,
                    task: DEFECATE_VIRUS_TERRAIN_TASK,
                    constructor_inputs: DefecateVirusTaskConstructorInputs::TerrainInfection {
                        lifetime_ms: 67,
                        payload: 0,
                        mode: 5,
                        packed_mode: 0x0005_0000,
                    },
                },
                DefecateVirusTaskPreparation {
                    phase_index: 1,
                    task: DEFECATE_VIRUS_WANDER_TASK,
                    constructor_inputs: DefecateVirusTaskConstructorInputs::Wander {
                        lifetime_ms: 2_000,
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
            Some(InstalledTask::Virus(DefecateVirusTaskRole::Wander))
        );
        assert_eq!(installed(&owner, ActorTaskSlot::Secondary), None);
        assert_eq!(
            installed(&owner, ActorTaskSlot::Tertiary),
            Some(InstalledTask::Virus(
                DefecateVirusTaskRole::TerrainInfection
            ))
        );
        assert_eq!(sub_a.target_speed_raw(), RetailRuntimeValue::Known(1));
        assert_eq!(sub_a.direction_multiplier(), -1);
        assert_eq!(sub_a.drive_scale_percent(), 73);
    }

    #[test]
    fn authored_sub_a_absence_is_a_successful_configuration() {
        let mut owner = seeded_owner();
        apply_defecate_virus_setup(
            &mut owner,
            DefecateVirusSetupRequest {
                terrain_task_lifetime_ms: 67,
            },
            DefecateVirusSubATopology::NotAuthored,
            prepared,
        )
        .unwrap();

        assert_eq!(
            installed(&owner, ActorTaskSlot::Primary),
            Some(InstalledTask::Virus(DefecateVirusTaskRole::Wander))
        );
        assert_eq!(
            installed(&owner, ActorTaskSlot::Tertiary),
            Some(InstalledTask::Virus(
                DefecateVirusTaskRole::TerrainInfection
            ))
        );
    }

    #[test]
    fn terrain_failure_commits_only_secondary_clear() {
        let mut owner = seeded_owner();
        let error = apply_defecate_virus_setup(
            &mut owner,
            DefecateVirusSetupRequest {
                terrain_task_lifetime_ms: 67,
            },
            DefecateVirusSubATopology::NotAuthored,
            |_preparation| Err::<PreparedActorTask<InstalledTask>, _>("terrain allocation"),
        )
        .unwrap_err();

        assert_eq!(
            error,
            DefecateVirusSetupError {
                phase_index: 0,
                slot: ActorTaskSlot::Tertiary,
                role: DefecateVirusTaskRole::TerrainInfection,
                failure: DefecateVirusSetupFailure::Preparation("terrain allocation"),
            }
        );
        assert_eq!(
            installed(&owner, ActorTaskSlot::Primary),
            Some(InstalledTask::Old(ActorTaskSlot::Primary))
        );
        assert_eq!(installed(&owner, ActorTaskSlot::Secondary), None);
        assert_eq!(
            installed(&owner, ActorTaskSlot::Tertiary),
            Some(InstalledTask::Old(ActorTaskSlot::Tertiary))
        );
    }

    #[test]
    fn wander_failure_keeps_new_terrain_and_old_primary_without_sub_a_write() {
        let mut owner = seeded_owner();
        let mut sub_a =
            SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Known(99), -1, 73);
        let mut attempts = Vec::new();
        let error = apply_defecate_virus_setup(
            &mut owner,
            DefecateVirusSetupRequest {
                terrain_task_lifetime_ms: 67,
            },
            DefecateVirusSubATopology::Authored(&mut sub_a),
            |preparation| {
                attempts.push(preparation.task.role);
                if preparation.task.role == DefecateVirusTaskRole::Wander {
                    Err("wander allocation")
                } else {
                    prepared(preparation)
                }
            },
        )
        .unwrap_err();

        assert_eq!(
            attempts,
            [
                DefecateVirusTaskRole::TerrainInfection,
                DefecateVirusTaskRole::Wander
            ]
        );
        assert_eq!(
            error,
            DefecateVirusSetupError {
                phase_index: 1,
                slot: ActorTaskSlot::Primary,
                role: DefecateVirusTaskRole::Wander,
                failure: DefecateVirusSetupFailure::Preparation("wander allocation"),
            }
        );
        assert_eq!(
            installed(&owner, ActorTaskSlot::Primary),
            Some(InstalledTask::Old(ActorTaskSlot::Primary))
        );
        assert_eq!(
            installed(&owner, ActorTaskSlot::Tertiary),
            Some(InstalledTask::Virus(
                DefecateVirusTaskRole::TerrainInfection
            ))
        );
        assert_eq!(sub_a.target_speed_raw(), RetailRuntimeValue::Known(99));
    }

    #[test]
    fn unresolved_required_sub_a_fails_closed_before_primary_publish() {
        let mut owner = seeded_owner();
        let error = apply_defecate_virus_setup(
            &mut owner,
            DefecateVirusSetupRequest {
                terrain_task_lifetime_ms: 67,
            },
            DefecateVirusSubATopology::RequiredUnresolved,
            prepared,
        )
        .unwrap_err();

        assert_eq!(
            error,
            DefecateVirusSetupError {
                phase_index: 1,
                slot: ActorTaskSlot::Primary,
                role: DefecateVirusTaskRole::Wander,
                failure: DefecateVirusSetupFailure::RequiredSubATopologyUnresolved,
            }
        );
        assert_eq!(
            installed(&owner, ActorTaskSlot::Primary),
            Some(InstalledTask::Old(ActorTaskSlot::Primary))
        );
        assert_eq!(
            installed(&owner, ActorTaskSlot::Tertiary),
            Some(InstalledTask::Virus(
                DefecateVirusTaskRole::TerrainInfection
            ))
        );
    }
}
