//! Detached class-12 `"Flip Over And Die"` task shell.
//!
//! This module retains the exact successful-allocation suffix of
//! `FUN_00404120`, the callback body at `FUN_00404220`, and the generic
//! task-owner lifetime/result policy. Its detached setup plan can publish into
//! an [`ActorTaskOwner`], but the module deliberately does not attach that
//! owner to live entities, bind retail component pointers, run
//! `FUN_00401430` itself, or dispatch the requested owner transition.
//!
//! Constructor effects remain explicit because `FUN_00406030` calls shared
//! component initializer `FUN_00406070` before the class-12-specific tail.
//! Collapsing those calls into a generic "enable components" flag would lose
//! both their order and the two conditional shared-RNG draws. When both
//! components are authored, Sub-G consumes its draw before Sub-A.

use crate::{
    actor_task_owner::{
        ActorTaskMutation, ActorTaskOwner, ActorTaskPrepareError, ActorTaskSlot, PreparedActorTask,
    },
    entity_collision_state::{
        CommonMoverComponentTopology, EntityTypeRuntimeMetadata, RetailRuntimeValue,
    },
};

pub const COMMON_DYING_INITIALIZER_ADDRESS: u32 = 0x0040_C620;
pub const COMMON_DYING_TASK_CONSTRUCTOR_ADDRESS: u32 = 0x0040_4120;
pub const COMMON_DYING_TASK_TICK_ADDRESS: u32 = 0x0040_4220;
pub const COMMON_DYING_SHARED_COMPONENT_SETUP_ADDRESS: u32 = 0x0040_6030;
pub const COMMON_DYING_SHARED_COMPONENT_INITIALIZER_ADDRESS: u32 = 0x0040_6070;
pub const COMMON_DYING_COMMON_MOVER_ADDRESS: u32 = 0x0040_1430;
pub const COMMON_DYING_EFFECT_ADDRESS: u32 = 0x0041_EC70;
pub const COMMON_DYING_ATTACHED_MASS_ADDRESS: u32 = 0x0041_85C0;
pub const COMMON_DYING_TASK_LIFETIME_MS: u32 = 9_000;
pub const COMMON_DYING_INITIAL_VERTICAL_VELOCITY_RAW: i16 = 500;

pub const COMMON_DYING_SCHEDULER_SINGLETON_ADDRESS: u32 = 0x004B_E170;
pub const COMMON_DYING_OWNER_TRANSITION_TAG: u32 = 0x0000_9C01;

pub const COMMON_DYING_EFFECT_KIND: u32 = 0x0100;
pub const COMMON_DYING_EFFECT_ENABLED: u32 = 1;
pub const COMMON_DYING_ATTACHED_MASS_SUPPRESSION_THRESHOLD: i32 = 150;

pub const COMMON_DYING_SUB_H_WRITER_ADDRESS: u32 = 0x0043_8040;
pub const COMMON_DYING_SUB_G_1B970_ADDRESS: u32 = 0x0041_B970;
pub const COMMON_DYING_SUB_G_1B940_ADDRESS: u32 = 0x0041_B940;
pub const COMMON_DYING_SUB_G_1B980_ADDRESS: u32 = 0x0041_B980;
pub const COMMON_DYING_COMPONENT_24380_ADDRESS: u32 = 0x0042_4380;
pub const COMMON_DYING_COMPONENT_24390_ADDRESS: u32 = 0x0042_4390;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommonDyingConstructorError {
    UnresolvedComponentTopology,
    UnresolvedSubADescriptor,
    MissingSubADescriptor,
    UnresolvedSubGDescriptor,
    MissingSubGDescriptor,
}

/// The two Sub-G descriptor fields consumed by shared initializer
/// `FUN_00406070`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommonDyingSubGDescriptorSnapshot {
    /// Descriptor dword `+0x00`, copied to runtime dword `+0x20`.
    pub source_raw_at_0x00: u32,
    /// Signed descriptor word `+0x0C`, added to the random high byte.
    pub randomized_target_base_raw_at_0x0c: i16,
}

/// Constructor-only descriptors not retained by
/// [`EntityTypeRuntimeMetadata`].
///
/// A live adapter must supply Sub-G only when the topology says it is
/// authored. Unresolved or contradictory evidence fails before RNG, external
/// state writes, or task publication.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommonDyingComponentDescriptors {
    pub sub_g: RetailRuntimeValue<Option<CommonDyingSubGDescriptorSnapshot>>,
}

impl Default for CommonDyingComponentDescriptors {
    fn default() -> Self {
        Self {
            sub_g: RetailRuntimeValue::Unresolved,
        }
    }
}

/// One exact external state change in the successful constructor suffix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommonDyingConstructorEffect {
    /// `FUN_00438040`: write runtime dword `+0x08`.
    WriteSubHState08 {
        value: u32,
    },
    /// `FUN_0041B970`: write byte `runtime + 0x3F`.
    WriteSubGMode3f {
        value: u8,
    },
    /// `FUN_0041B940`: one shared RNG draw, then write runtime dword `+0x38`.
    WriteSubGRandomizedTarget38 {
        target_raw: i32,
        random_sample_low16: u16,
        descriptor_base_raw: i16,
    },
    /// `FUN_0041B980(..., 0)`: clear runtime byte `+0x40` and dword
    /// `+0x24`, then copy descriptor dword `+0x00` to runtime `+0x20`.
    ResetSubGModeAndAccumulatorThenWriteSource {
        source_raw: u32,
        accumulator_raw: u32,
        mode_40: u8,
    },
    /// `FUN_00424380`: write byte `runtime + 0x3C`.
    WriteSubGPhase3c {
        value: u8,
    },
    /// `FUN_00424380`: write byte `root runtime + 0x3C`.
    WriteSubFPhase3c {
        value: u8,
    },
    /// `FUN_00424390(..., 0)`: write byte `+0x3D` and dword `+0x24`.
    ResetSubFModeAndAccumulator {
        mode_3d: u8,
        accumulator_raw: u32,
    },
    /// First Sub-A write in `FUN_00406070`: runtime dword `+0x04 = 1`.
    WriteSubADirection {
        direction_multiplier: i32,
    },
    /// Second Sub-A write in `FUN_00406070`: runtime dword `+0x00`.
    WriteSubATargetSpeed {
        target_speed_raw: i32,
        random_sample_low16: u16,
    },
    WriteOwnerVerticalVelocity {
        velocity_raw: i16,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CommonDyingConstructorSuffix {
    topology: CommonMoverComponentTopology,
    sub_a_target_speed_base_raw: Option<i16>,
    sub_g_descriptor: Option<CommonDyingSubGDescriptorSnapshot>,
}

/// Successful allocation paired with every still-unpublished constructor
/// effect.
///
/// `apply_suffix` is the only way to recover the inner
/// [`PreparedActorTask`]. This keeps all component initialization, the vertical
/// velocity write, and the class-12 component tail ahead of slot publication.
#[derive(Debug)]
pub struct PreparedCommonDyingTask<T = CommonDyingTaskState> {
    task: PreparedActorTask<T>,
    owner_entity_id: u32,
    suffix: CommonDyingConstructorSuffix,
}

impl PreparedCommonDyingTask {
    pub fn map_task<T>(
        self,
        map: impl FnOnce(CommonDyingTaskState) -> T,
    ) -> PreparedCommonDyingTask<T> {
        PreparedCommonDyingTask {
            task: self.task.map(map),
            owner_entity_id: self.owner_entity_id,
            suffix: self.suffix,
        }
    }
}

impl<T> PreparedCommonDyingTask<T> {
    pub const fn owner_entity_id(&self) -> u32 {
        self.owner_entity_id
    }

    /// Apply `FUN_00406070` and the remaining `FUN_00404120` tail in exact
    /// order, then release the task for publication.
    ///
    /// Shared RNG is consumed once by authored Sub-G and once by authored
    /// Sub-A. With both present, the Sub-G draw occurs first.
    pub fn apply_suffix(
        self,
        mut next_shared_random: impl FnMut() -> u32,
        mut apply: impl FnMut(CommonDyingConstructorEffect),
    ) -> PreparedActorTask<T> {
        let topology = self.suffix.topology;
        let sub_g_descriptor = self.suffix.sub_g_descriptor;

        if topology.sub_h {
            apply(CommonDyingConstructorEffect::WriteSubHState08 { value: 1 });
        }
        if let Some(descriptor) = sub_g_descriptor {
            apply(CommonDyingConstructorEffect::WriteSubGMode3f { value: 0 });
            let random_sample_low16 = next_shared_random() as u16;
            let random_byte = i32::from(random_sample_low16 >> 8);
            let descriptor_base_raw = descriptor.randomized_target_base_raw_at_0x0c;
            let target_raw = i32::from(descriptor_base_raw).wrapping_add(random_byte);
            apply(CommonDyingConstructorEffect::WriteSubGRandomizedTarget38 {
                target_raw,
                random_sample_low16,
                descriptor_base_raw,
            });
            apply(
                CommonDyingConstructorEffect::ResetSubGModeAndAccumulatorThenWriteSource {
                    source_raw: descriptor.source_raw_at_0x00,
                    accumulator_raw: 0,
                    mode_40: 0,
                },
            );
            apply(CommonDyingConstructorEffect::WriteSubGPhase3c { value: 0 });
        }
        if topology.sub_f {
            apply(CommonDyingConstructorEffect::WriteSubFPhase3c { value: 0 });
            apply(CommonDyingConstructorEffect::ResetSubFModeAndAccumulator {
                mode_3d: 0,
                accumulator_raw: 0xE000,
            });
        }
        if let Some(base) = self.suffix.sub_a_target_speed_base_raw {
            let random_sample_low16 = next_shared_random() as u16;
            let random_byte = i32::from(random_sample_low16 >> 8);
            let base = i32::from(base);
            let target_speed_raw = base.wrapping_add(random_byte.wrapping_mul(base) / 0x0A00);
            apply(CommonDyingConstructorEffect::WriteSubADirection {
                direction_multiplier: 1,
            });
            apply(CommonDyingConstructorEffect::WriteSubATargetSpeed {
                target_speed_raw,
                random_sample_low16,
            });
        }

        apply(CommonDyingConstructorEffect::WriteOwnerVerticalVelocity {
            velocity_raw: COMMON_DYING_INITIAL_VERTICAL_VELOCITY_RAW,
        });

        if topology.sub_a {
            apply(CommonDyingConstructorEffect::WriteSubADirection {
                direction_multiplier: 1,
            });
        }
        if topology.sub_h {
            apply(CommonDyingConstructorEffect::WriteSubHState08 { value: 0 });
        }
        if sub_g_descriptor.is_some() {
            apply(CommonDyingConstructorEffect::WriteSubGMode3f { value: 1 });
        }

        self.task
    }
}

/// Proven input to `FUN_00404120` from outer initializer `FUN_0040C620`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommonDyingTaskSpec {
    owner_entity_id: u32,
    lifetime_ms: u32,
}

impl CommonDyingTaskSpec {
    pub const fn owner_entity_id(self) -> u32 {
        self.owner_entity_id
    }

    pub const fn lifetime_ms(self) -> u32 {
        self.lifetime_ms
    }

    pub const fn constructor_address(self) -> u32 {
        COMMON_DYING_TASK_CONSTRUCTOR_ADDRESS
    }
}

/// One-shot `FUN_0040C620` task transaction.
#[derive(Debug, PartialEq, Eq)]
pub struct CommonDyingSetupPlan {
    ordered_mutations: [ActorTaskMutation<CommonDyingTaskSpec>; 3],
}

impl CommonDyingSetupPlan {
    /// Apply clear-secondary, clear-tertiary, then try-install-primary.
    ///
    /// A failed allocation/preparation preserves the old primary while both
    /// earlier clears remain committed. On success the complete constructor
    /// suffix runs before primary-slot publication.
    pub fn apply<T, E>(
        self,
        owner: &mut ActorTaskOwner<T>,
        mut next_shared_random: impl FnMut() -> u32,
        mut apply_effect: impl FnMut(CommonDyingConstructorEffect),
        mut prepare: impl FnMut(CommonDyingTaskSpec) -> Result<PreparedCommonDyingTask<T>, E>,
    ) -> Result<(), ActorTaskPrepareError<E>> {
        owner.apply_ordered_mutations(self.ordered_mutations, |specification| {
            let prepared = prepare(specification)?;
            Ok(prepared.apply_suffix(&mut next_shared_random, &mut apply_effect))
        })
    }
}

/// Build the exact clear-secondary, clear-tertiary, install-primary sequence
/// emitted by `FUN_0040C620`.
pub fn plan_common_dying_setup(owner_entity_id: u32) -> CommonDyingSetupPlan {
    CommonDyingSetupPlan {
        ordered_mutations: [
            ActorTaskMutation::Clear {
                slot: ActorTaskSlot::Secondary,
            },
            ActorTaskMutation::Clear {
                slot: ActorTaskSlot::Tertiary,
            },
            ActorTaskMutation::TryInstall {
                slot: ActorTaskSlot::Primary,
                specification: CommonDyingTaskSpec {
                    owner_entity_id,
                    lifetime_ms: COMMON_DYING_TASK_LIFETIME_MS,
                },
            },
        ],
    }
}

/// Scheduler-owned state for the 9,000-ms class-12 primary task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommonDyingTaskState {
    elapsed_ms: u32,
}

impl CommonDyingTaskState {
    /// Complete construction after the generic wrapper allocation succeeds.
    ///
    /// No RNG or external effect occurs here. Metadata is first validated into
    /// an all-or-nothing suffix, allowing [`PreparedCommonDyingTask::apply_suffix`]
    /// to retain retail call order without publishing a partial task.
    pub fn prepare_after_allocation(
        owner_entity_id: u32,
        metadata: &EntityTypeRuntimeMetadata,
        descriptors: CommonDyingComponentDescriptors,
    ) -> Result<PreparedCommonDyingTask, CommonDyingConstructorError> {
        let topology = match metadata.common_mover_topology {
            RetailRuntimeValue::Known(topology) => topology,
            RetailRuntimeValue::Unresolved => {
                return Err(CommonDyingConstructorError::UnresolvedComponentTopology);
            }
        };

        let sub_a_target_speed_base_raw = if topology.sub_a {
            match metadata.sub_a_propulsion_descriptor {
                RetailRuntimeValue::Known(Some(descriptor)) => {
                    Some(descriptor.target_speed_base_raw)
                }
                RetailRuntimeValue::Known(None) => {
                    return Err(CommonDyingConstructorError::MissingSubADescriptor);
                }
                RetailRuntimeValue::Unresolved => {
                    return Err(CommonDyingConstructorError::UnresolvedSubADescriptor);
                }
            }
        } else {
            None
        };

        let sub_g_descriptor = if topology.sub_g {
            match descriptors.sub_g {
                RetailRuntimeValue::Known(Some(descriptor)) => Some(descriptor),
                RetailRuntimeValue::Known(None) => {
                    return Err(CommonDyingConstructorError::MissingSubGDescriptor);
                }
                RetailRuntimeValue::Unresolved => {
                    return Err(CommonDyingConstructorError::UnresolvedSubGDescriptor);
                }
            }
        } else {
            None
        };

        Ok(PreparedCommonDyingTask {
            task: PreparedActorTask::new(Self { elapsed_ms: 0 }),
            owner_entity_id,
            suffix: CommonDyingConstructorSuffix {
                topology,
                sub_a_target_speed_base_raw,
                sub_g_descriptor,
            },
        })
    }

    #[cfg(test)]
    const fn from_elapsed_ms(elapsed_ms: u32) -> Self {
        Self { elapsed_ms }
    }

    pub const fn elapsed_ms(self) -> u32 {
        self.elapsed_ms
    }

    /// Exact `FUN_00401120` prefix: truncate each frame independently,
    /// wrapping-add milliseconds, and mark the post-unwind transition due only
    /// when `9000 < elapsed`.
    pub fn before_callback(&mut self, elapsed_micros: u32) -> CommonDyingCallbackPrefix {
        self.elapsed_ms = self.elapsed_ms.wrapping_add(elapsed_micros / 1_000);
        CommonDyingCallbackPrefix {
            elapsed_ms: self.elapsed_ms,
            lifetime_status: if COMMON_DYING_TASK_LIFETIME_MS < self.elapsed_ms {
                CommonDyingLifetimeStatus::OwnerTransitionDue
            } else {
                CommonDyingLifetimeStatus::WithinLifetime
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommonDyingLifetimeStatus {
    WithinLifetime,
    OwnerTransitionDue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommonDyingCallbackPrefix {
    pub elapsed_ms: u32,
    pub lifetime_status: CommonDyingLifetimeStatus,
}

/// Lazy post-scheduler-mode inputs for selecting the signed effect byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommonDyingEffectInputs {
    /// Exact signed Section-12 Sub-C byte at descriptor `+0x0C`.
    pub descriptor_effect_raw: RetailRuntimeValue<Option<i8>>,
    /// Result of `FUN_004185C0(component_runtime + 0x14)`.
    ///
    /// This value is ignored when Sub-C is authoritatively absent.
    pub attached_mass: RetailRuntimeValue<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommonDyingCallbackError {
    UnresolvedEffectDescriptor,
    UnresolvedAttachedMass,
}

/// Resolve the signed Sub-C selector consumed by Common-Dying's direct
/// terrain-attitude call.
///
/// Keeping this separate from backend invocation lets live adapters preflight
/// every fallible descriptor/mass lookup before the task's elapsed prefix is
/// committed.  Authored Sub-C absence selects zero; an attached-mass sum
/// strictly above 150 suppresses a present selector.
pub fn select_common_dying_effect_raw(
    inputs: CommonDyingEffectInputs,
) -> Result<i32, CommonDyingCallbackError> {
    match inputs.descriptor_effect_raw {
        RetailRuntimeValue::Known(None) => Ok(0),
        RetailRuntimeValue::Known(Some(effect_raw)) => {
            let attached_mass = match inputs.attached_mass {
                RetailRuntimeValue::Known(attached_mass) => attached_mass,
                RetailRuntimeValue::Unresolved => {
                    return Err(CommonDyingCallbackError::UnresolvedAttachedMass);
                }
            };
            Ok(
                if COMMON_DYING_ATTACHED_MASS_SUPPRESSION_THRESHOLD < attached_mass {
                    0
                } else {
                    i32::from(effect_raw)
                },
            )
        }
        RetailRuntimeValue::Unresolved => Err(CommonDyingCallbackError::UnresolvedEffectDescriptor),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvokeCommonDyingEffect {
    pub owner_entity_id: u32,
    pub effect_kind: u32,
    pub signed_effect_raw: i32,
    pub enabled: u32,
    pub elapsed_micros: u32,
}

/// Exact seven arguments submitted to shared common mover `FUN_00401430`.
#[derive(Debug)]
pub struct InvokeCommonDyingMover<TaskWrapper, TypeRuntime, ComponentRuntime> {
    pub task_wrapper: TaskWrapper,
    pub owner_entity_id: u32,
    pub type_runtime: TypeRuntime,
    pub component_runtime: ComponentRuntime,
    pub explicit_target_raw: Option<[i16; 3]>,
    pub elapsed_micros: u32,
    pub scheduler_mode: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommonDyingTaggedResult {
    pub singleton_address: u32,
    pub tag: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommonDyingCallbackResult {
    Continue,
    TaggedOwnerTransition(CommonDyingTaggedResult),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommonDyingCallbackResolution {
    pub callback_result: CommonDyingCallbackResult,
    pub linear_velocity_raw: [i16; 3],
    /// `None` proves the scheduler-mode gate returned before effect selection.
    pub selected_effect_raw: Option<i32>,
}

/// Detached inputs that are already available before the scheduler-mode gate.
///
/// A live adapter must keep Sub-C descriptor/mass resolution in the
/// `resolve_effect_inputs` closure: retail does not inspect either value when
/// `scheduler_mode != 0`.
#[derive(Debug)]
pub struct CommonDyingFrameRequest<TaskWrapper, TypeRuntime, ComponentRuntime> {
    pub task_wrapper: TaskWrapper,
    pub owner_entity_id: u32,
    pub type_runtime: TypeRuntime,
    pub component_runtime: ComponentRuntime,
    pub elapsed_micros: u32,
    pub scheduler_mode: u32,
    /// Live value immediately before `FUN_00401430`. The mover closure may
    /// replace it; class-12 damping reads the post-mover X/Z words.
    pub linear_velocity_raw: [i16; 3],
}

/// Execute the exact body of `FUN_00404220`.
///
/// Nonzero scheduler mode returns singleton `0x004BE170` without selecting an
/// effect, invoking either backend, or damping velocity. Normal mode always
/// submits the `0x100` effect first, then calls the common mover and ignores
/// its return. Finally it damps only the mover-produced X/Z velocity words;
/// Y is untouched.
pub fn tick_common_dying<TaskWrapper, TypeRuntime, ComponentRuntime, MoverReturn>(
    request: CommonDyingFrameRequest<TaskWrapper, TypeRuntime, ComponentRuntime>,
    resolve_effect_inputs: impl FnOnce() -> CommonDyingEffectInputs,
    mut invoke_effect: impl FnMut(InvokeCommonDyingEffect),
    invoke_common_mover: impl FnOnce(
        InvokeCommonDyingMover<TaskWrapper, TypeRuntime, ComponentRuntime>,
        &mut [i16; 3],
    ) -> MoverReturn,
) -> Result<CommonDyingCallbackResolution, CommonDyingCallbackError> {
    if request.scheduler_mode != 0 {
        return Ok(CommonDyingCallbackResolution {
            callback_result: CommonDyingCallbackResult::TaggedOwnerTransition(
                CommonDyingTaggedResult {
                    singleton_address: COMMON_DYING_SCHEDULER_SINGLETON_ADDRESS,
                    tag: COMMON_DYING_OWNER_TRANSITION_TAG,
                },
            ),
            linear_velocity_raw: request.linear_velocity_raw,
            selected_effect_raw: None,
        });
    }

    let signed_effect_raw = select_common_dying_effect_raw(resolve_effect_inputs())?;

    invoke_effect(InvokeCommonDyingEffect {
        owner_entity_id: request.owner_entity_id,
        effect_kind: COMMON_DYING_EFFECT_KIND,
        signed_effect_raw,
        enabled: COMMON_DYING_EFFECT_ENABLED,
        elapsed_micros: request.elapsed_micros,
    });

    let mut linear_velocity_raw = request.linear_velocity_raw;
    let mover_request = InvokeCommonDyingMover {
        task_wrapper: request.task_wrapper,
        owner_entity_id: request.owner_entity_id,
        type_runtime: request.type_runtime,
        component_runtime: request.component_runtime,
        explicit_target_raw: None,
        elapsed_micros: request.elapsed_micros,
        scheduler_mode: 0,
    };
    let _ignored_return = invoke_common_mover(mover_request, &mut linear_velocity_raw);
    linear_velocity_raw[0] = damp_common_dying_axis(linear_velocity_raw[0], request.elapsed_micros);
    linear_velocity_raw[2] = damp_common_dying_axis(linear_velocity_raw[2], request.elapsed_micros);

    Ok(CommonDyingCallbackResolution {
        callback_result: CommonDyingCallbackResult::Continue,
        linear_velocity_raw,
        selected_effect_raw: Some(signed_effect_raw),
    })
}

/// Exact signed fixed-point damping expression emitted by the x86 compiler.
///
/// Retail first wraps `elapsed_micros << 11` to 32 bits, treats that result as
/// signed, multiplies by the signed velocity word, then extracts the signed
/// product shifted right 31 before a wrapping 16-bit subtraction.
pub fn damp_common_dying_axis(velocity_raw: i16, elapsed_micros: u32) -> i16 {
    let scaled_elapsed = elapsed_micros.wrapping_shl(11) as i32;
    let product = i64::from(scaled_elapsed) * i64::from(velocity_raw);
    let damping_raw = (product >> 31) as i16;
    velocity_raw.wrapping_sub(damping_raw)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommonDyingTransitionReason {
    TaggedSchedulerMode(CommonDyingTaggedResult),
    LifetimeExpired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommonDyingAfterUnwindOutcome {
    Continue,
    /// A request for the generic actor owner to run its primary transition
    /// callback and state gates. The actual transition is not guaranteed.
    RequestOwnerTransition {
        reason: CommonDyingTransitionReason,
    },
}

/// Preserve `FUN_00401120` precedence after the task wrapper survives.
///
/// A tagged callback result requests the owner transition before the generic
/// timeout is inspected. A null result falls through to the strict 9,000-ms
/// lifetime.
pub const fn common_dying_after_unwind(
    prefix: CommonDyingCallbackPrefix,
    callback_result: CommonDyingCallbackResult,
) -> CommonDyingAfterUnwindOutcome {
    match callback_result {
        CommonDyingCallbackResult::TaggedOwnerTransition(tagged) => {
            CommonDyingAfterUnwindOutcome::RequestOwnerTransition {
                reason: CommonDyingTransitionReason::TaggedSchedulerMode(tagged),
            }
        }
        CommonDyingCallbackResult::Continue => match prefix.lifetime_status {
            CommonDyingLifetimeStatus::WithinLifetime => CommonDyingAfterUnwindOutcome::Continue,
            CommonDyingLifetimeStatus::OwnerTransitionDue => {
                CommonDyingAfterUnwindOutcome::RequestOwnerTransition {
                    reason: CommonDyingTransitionReason::LifetimeExpired,
                }
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use super::*;
    use v2k_formats::collision::SubAPropulsionDescriptor;

    fn metadata(topology: CommonMoverComponentTopology) -> EntityTypeRuntimeMetadata {
        EntityTypeRuntimeMetadata {
            common_mover_topology: RetailRuntimeValue::Known(topology),
            sub_a_propulsion_descriptor: RetailRuntimeValue::Known(None),
            ..EntityTypeRuntimeMetadata::default()
        }
    }

    fn sub_a_descriptor(base: i16) -> SubAPropulsionDescriptor {
        SubAPropulsionDescriptor {
            acceleration_raw: 0,
            overspeed_correction_raw: 0,
            target_speed_base_raw: base,
        }
    }

    const fn sub_g_descriptor(
        source_raw: u32,
        randomized_target_base_raw: i16,
    ) -> CommonDyingSubGDescriptorSnapshot {
        CommonDyingSubGDescriptorSnapshot {
            source_raw_at_0x00: source_raw,
            randomized_target_base_raw_at_0x0c: randomized_target_base_raw,
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum LoggedConstructorEffect {
        H(u32),
        GMode(u8),
        GRandomizedTarget(i32, u16, i16),
        GSource(u32, u32, u8),
        GPhase(u8),
        FPhase(u8),
        FModeAndAccumulator(u8, u32),
        ATarget(i32, u16),
        Velocity(i16),
        ADirection(i32),
        Published,
    }

    fn log_effect(
        log: &RefCell<Vec<LoggedConstructorEffect>>,
        effect: CommonDyingConstructorEffect,
    ) {
        let event = match effect {
            CommonDyingConstructorEffect::WriteSubHState08 { value } => {
                LoggedConstructorEffect::H(value)
            }
            CommonDyingConstructorEffect::WriteSubGMode3f { value } => {
                LoggedConstructorEffect::GMode(value)
            }
            CommonDyingConstructorEffect::WriteSubGRandomizedTarget38 {
                target_raw,
                random_sample_low16,
                descriptor_base_raw,
            } => LoggedConstructorEffect::GRandomizedTarget(
                target_raw,
                random_sample_low16,
                descriptor_base_raw,
            ),
            CommonDyingConstructorEffect::ResetSubGModeAndAccumulatorThenWriteSource {
                source_raw,
                accumulator_raw,
                mode_40,
            } => LoggedConstructorEffect::GSource(source_raw, accumulator_raw, mode_40),
            CommonDyingConstructorEffect::WriteSubGPhase3c { value } => {
                LoggedConstructorEffect::GPhase(value)
            }
            CommonDyingConstructorEffect::WriteSubFPhase3c { value } => {
                LoggedConstructorEffect::FPhase(value)
            }
            CommonDyingConstructorEffect::ResetSubFModeAndAccumulator {
                mode_3d,
                accumulator_raw,
            } => LoggedConstructorEffect::FModeAndAccumulator(mode_3d, accumulator_raw),
            CommonDyingConstructorEffect::WriteSubATargetSpeed {
                target_speed_raw,
                random_sample_low16,
            } => LoggedConstructorEffect::ATarget(target_speed_raw, random_sample_low16),
            CommonDyingConstructorEffect::WriteOwnerVerticalVelocity { velocity_raw } => {
                LoggedConstructorEffect::Velocity(velocity_raw)
            }
            CommonDyingConstructorEffect::WriteSubADirection {
                direction_multiplier,
            } => LoggedConstructorEffect::ADirection(direction_multiplier),
        };
        log.borrow_mut().push(event);
    }

    #[test]
    fn full_constructor_suffix_preserves_h_g_f_a_tail_and_publication_order() {
        let mut topology = CommonMoverComponentTopology::default();
        topology.sub_a = true;
        topology.sub_f = true;
        topology.sub_g = true;
        topology.sub_h = true;
        let mut metadata = metadata(topology);
        metadata.sub_a_propulsion_descriptor =
            RetailRuntimeValue::Known(Some(sub_a_descriptor(2_560)));

        let prepared = CommonDyingTaskState::prepare_after_allocation(
            0x0497_0001,
            &metadata,
            CommonDyingComponentDescriptors {
                sub_g: RetailRuntimeValue::Known(Some(sub_g_descriptor(0xABCD, -5))),
            },
        )
        .unwrap();
        assert_eq!(prepared.owner_entity_id(), 0x0497_0001);

        let draws = Cell::new(0);
        let log = RefCell::new(Vec::new());
        let task = prepared.apply_suffix(
            || {
                let draw = match draws.get() {
                    0 => 0xCAFE_1200,
                    1 => 0xCAFE_FF00,
                    _ => panic!("constructor consumed more than two RNG draws"),
                };
                draws.set(draws.get() + 1);
                draw
            },
            |effect| log_effect(&log, effect),
        );
        assert_eq!(draws.get(), 2);

        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(ActorTaskSlot::Primary, task);
        log.borrow_mut().push(LoggedConstructorEffect::Published);
        assert_eq!(
            *log.borrow(),
            [
                LoggedConstructorEffect::H(1),
                LoggedConstructorEffect::GMode(0),
                LoggedConstructorEffect::GRandomizedTarget(13, 0x1200, -5),
                LoggedConstructorEffect::GSource(0xABCD, 0, 0),
                LoggedConstructorEffect::GPhase(0),
                LoggedConstructorEffect::FPhase(0),
                LoggedConstructorEffect::FModeAndAccumulator(0, 0xE000),
                LoggedConstructorEffect::ADirection(1),
                LoggedConstructorEffect::ATarget(2_815, 0xFF00),
                LoggedConstructorEffect::Velocity(500),
                LoggedConstructorEffect::ADirection(1),
                LoggedConstructorEffect::H(0),
                LoggedConstructorEffect::GMode(1),
                LoggedConstructorEffect::Published,
            ]
        );
        assert_eq!(
            owner
                .state_in_slot(ActorTaskSlot::Primary)
                .unwrap()
                .elapsed_ms(),
            0
        );
    }

    #[test]
    fn outer_initializer_plan_is_clear_secondary_clear_tertiary_then_primary_install() {
        let plan = plan_common_dying_setup(0x0497_0001);
        assert_eq!(
            plan.ordered_mutations,
            [
                ActorTaskMutation::Clear {
                    slot: ActorTaskSlot::Secondary,
                },
                ActorTaskMutation::Clear {
                    slot: ActorTaskSlot::Tertiary,
                },
                ActorTaskMutation::TryInstall {
                    slot: ActorTaskSlot::Primary,
                    specification: CommonDyingTaskSpec {
                        owner_entity_id: 0x0497_0001,
                        lifetime_ms: 9_000,
                    },
                },
            ]
        );
    }

    #[test]
    fn failed_primary_prepare_keeps_primary_after_both_prior_clears() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(
            ActorTaskSlot::Primary,
            PreparedActorTask::new(CommonDyingTaskState::from_elapsed_ms(11)),
        );
        owner.replace_prepared(
            ActorTaskSlot::Secondary,
            PreparedActorTask::new(CommonDyingTaskState::from_elapsed_ms(22)),
        );
        owner.replace_prepared(
            ActorTaskSlot::Tertiary,
            PreparedActorTask::new(CommonDyingTaskState::from_elapsed_ms(33)),
        );
        let draws = Cell::new(0);
        let effects = Cell::new(0);

        let result = plan_common_dying_setup(0x0497_0001).apply(
            &mut owner,
            || {
                draws.set(draws.get() + 1);
                0
            },
            |_| effects.set(effects.get() + 1),
            |_| Err::<PreparedCommonDyingTask, _>("allocation failed"),
        );

        assert_eq!(
            result,
            Err(ActorTaskPrepareError {
                action_index: 2,
                slot: ActorTaskSlot::Primary,
                error: "allocation failed",
            })
        );
        assert_eq!(
            owner
                .state_in_slot(ActorTaskSlot::Primary)
                .unwrap()
                .elapsed_ms(),
            11
        );
        assert!(owner.task_in_slot(ActorTaskSlot::Secondary).is_none());
        assert!(owner.task_in_slot(ActorTaskSlot::Tertiary).is_none());
        assert_eq!(draws.get(), 0);
        assert_eq!(effects.get(), 0);
    }

    #[test]
    fn successful_plan_applies_suffix_before_publishing_new_primary() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(
            ActorTaskSlot::Primary,
            PreparedActorTask::new(CommonDyingTaskState::from_elapsed_ms(11)),
        );
        owner.replace_prepared(
            ActorTaskSlot::Secondary,
            PreparedActorTask::new(CommonDyingTaskState::from_elapsed_ms(22)),
        );
        owner.replace_prepared(
            ActorTaskSlot::Tertiary,
            PreparedActorTask::new(CommonDyingTaskState::from_elapsed_ms(33)),
        );
        let effects = RefCell::new(Vec::new());
        let metadata = metadata(CommonMoverComponentTopology::default());

        plan_common_dying_setup(0x0497_0001)
            .apply(
                &mut owner,
                || unreachable!("no authored RNG component"),
                |effect| log_effect(&effects, effect),
                |specification| {
                    assert_eq!(specification.owner_entity_id(), 0x0497_0001);
                    assert_eq!(specification.lifetime_ms(), 9_000);
                    assert_eq!(specification.constructor_address(), 0x0040_4120);
                    CommonDyingTaskState::prepare_after_allocation(
                        specification.owner_entity_id(),
                        &metadata,
                        CommonDyingComponentDescriptors::default(),
                    )
                },
            )
            .unwrap();
        effects
            .borrow_mut()
            .push(LoggedConstructorEffect::Published);

        assert_eq!(
            *effects.borrow(),
            [
                LoggedConstructorEffect::Velocity(500),
                LoggedConstructorEffect::Published,
            ]
        );
        assert_eq!(
            owner
                .state_in_slot(ActorTaskSlot::Primary)
                .unwrap()
                .elapsed_ms(),
            0
        );
        assert!(owner.task_in_slot(ActorTaskSlot::Secondary).is_none());
        assert!(owner.task_in_slot(ActorTaskSlot::Tertiary).is_none());
    }

    #[test]
    fn absent_optional_components_only_write_vertical_velocity_and_draw_no_rng() {
        let prepared = CommonDyingTaskState::prepare_after_allocation(
            7,
            &metadata(CommonMoverComponentTopology::default()),
            CommonDyingComponentDescriptors {
                sub_g: RetailRuntimeValue::Unresolved,
            },
        )
        .unwrap();
        let draws = Cell::new(0);
        let effects = RefCell::new(Vec::new());
        let _task = prepared.apply_suffix(
            || {
                draws.set(draws.get() + 1);
                0
            },
            |effect| log_effect(&effects, effect),
        );
        assert_eq!(draws.get(), 0);
        assert_eq!(*effects.borrow(), [LoggedConstructorEffect::Velocity(500)]);
    }

    #[test]
    fn negative_sub_a_base_uses_signed_truncation_and_low_sixteen_random_bits() {
        let mut topology = CommonMoverComponentTopology::default();
        topology.sub_a = true;
        let mut metadata = metadata(topology);
        metadata.sub_a_propulsion_descriptor =
            RetailRuntimeValue::Known(Some(sub_a_descriptor(-101)));
        let prepared = CommonDyingTaskState::prepare_after_allocation(
            7,
            &metadata,
            CommonDyingComponentDescriptors::default(),
        )
        .unwrap();
        let effects = RefCell::new(Vec::new());
        let _task = prepared.apply_suffix(|| 0xFF00_0100, |effect| log_effect(&effects, effect));
        assert_eq!(
            *effects.borrow(),
            [
                LoggedConstructorEffect::ADirection(1),
                LoggedConstructorEffect::ATarget(-101, 0x0100),
                LoggedConstructorEffect::Velocity(500),
                LoggedConstructorEffect::ADirection(1),
            ]
        );
    }

    #[test]
    fn every_unresolved_or_inconsistent_consumed_constructor_dependency_fails_closed() {
        let mut topology = CommonMoverComponentTopology::default();
        topology.sub_a = true;
        topology.sub_g = true;

        let unresolved_topology = EntityTypeRuntimeMetadata::default();
        assert_eq!(
            CommonDyingTaskState::prepare_after_allocation(
                1,
                &unresolved_topology,
                CommonDyingComponentDescriptors::default(),
            )
            .unwrap_err(),
            CommonDyingConstructorError::UnresolvedComponentTopology
        );

        let unresolved_a = metadata(topology);
        let mut unresolved_a = unresolved_a;
        unresolved_a.sub_a_propulsion_descriptor = RetailRuntimeValue::Unresolved;
        assert_eq!(
            CommonDyingTaskState::prepare_after_allocation(
                1,
                &unresolved_a,
                CommonDyingComponentDescriptors {
                    sub_g: RetailRuntimeValue::Known(Some(sub_g_descriptor(5, 0))),
                },
            )
            .unwrap_err(),
            CommonDyingConstructorError::UnresolvedSubADescriptor
        );

        let missing_a = metadata(topology);
        assert_eq!(
            CommonDyingTaskState::prepare_after_allocation(
                1,
                &missing_a,
                CommonDyingComponentDescriptors {
                    sub_g: RetailRuntimeValue::Known(Some(sub_g_descriptor(5, 0))),
                },
            )
            .unwrap_err(),
            CommonDyingConstructorError::MissingSubADescriptor
        );

        let mut valid_a = metadata(topology);
        valid_a.sub_a_propulsion_descriptor = RetailRuntimeValue::Known(Some(sub_a_descriptor(10)));
        assert_eq!(
            CommonDyingTaskState::prepare_after_allocation(
                1,
                &valid_a,
                CommonDyingComponentDescriptors::default(),
            )
            .unwrap_err(),
            CommonDyingConstructorError::UnresolvedSubGDescriptor
        );
        assert_eq!(
            CommonDyingTaskState::prepare_after_allocation(
                1,
                &valid_a,
                CommonDyingComponentDescriptors {
                    sub_g: RetailRuntimeValue::Known(None),
                },
            )
            .unwrap_err(),
            CommonDyingConstructorError::MissingSubGDescriptor
        );
    }

    #[test]
    fn failures_occur_before_any_rng_effect_or_publication() {
        let mut topology = CommonMoverComponentTopology::default();
        topology.sub_a = true;
        let metadata = metadata(topology);
        let draws = Cell::new(0);
        let effects = Cell::new(0);
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(
            ActorTaskSlot::Primary,
            PreparedActorTask::new(CommonDyingTaskState::from_elapsed_ms(123)),
        );

        let result = CommonDyingTaskState::prepare_after_allocation(
            1,
            &metadata,
            CommonDyingComponentDescriptors::default(),
        );
        assert_eq!(
            result.unwrap_err(),
            CommonDyingConstructorError::MissingSubADescriptor
        );
        assert_eq!(draws.get(), 0);
        assert_eq!(effects.get(), 0);
        assert_eq!(
            owner
                .state_in_slot(ActorTaskSlot::Primary)
                .unwrap()
                .elapsed_ms(),
            123
        );
    }

    fn frame_request(
        elapsed_micros: u32,
        scheduler_mode: u32,
    ) -> CommonDyingFrameRequest<u32, &'static str, &'static str> {
        CommonDyingFrameRequest {
            task_wrapper: 0x1234,
            owner_entity_id: 0x0497_0001,
            type_runtime: "type",
            component_runtime: "component",
            elapsed_micros,
            scheduler_mode,
            linear_velocity_raw: [1_000, 500, -1_000],
        }
    }

    #[test]
    fn scheduler_mode_returns_tag_without_resolution_effect_mover_or_damping() {
        let resolved = Cell::new(0);
        let effects = Cell::new(0);
        let movers = Cell::new(0);
        let result = tick_common_dying(
            frame_request(20_000, 7),
            || {
                resolved.set(resolved.get() + 1);
                CommonDyingEffectInputs {
                    descriptor_effect_raw: RetailRuntimeValue::Unresolved,
                    attached_mass: RetailRuntimeValue::Unresolved,
                }
            },
            |_| effects.set(effects.get() + 1),
            |_, _| movers.set(movers.get() + 1),
        )
        .unwrap();

        assert_eq!(resolved.get(), 0);
        assert_eq!(effects.get(), 0);
        assert_eq!(movers.get(), 0);
        assert_eq!(result.linear_velocity_raw, [1_000, 500, -1_000]);
        assert_eq!(result.selected_effect_raw, None);
        assert_eq!(
            result.callback_result,
            CommonDyingCallbackResult::TaggedOwnerTransition(CommonDyingTaggedResult {
                singleton_address: 0x004B_E170,
                tag: 0x9C01,
            })
        );
    }

    #[test]
    fn absent_descriptor_selects_zero_without_reading_unresolved_mass() {
        let effect = RefCell::new(None);
        let result = tick_common_dying(
            frame_request(0, 0),
            || CommonDyingEffectInputs {
                descriptor_effect_raw: RetailRuntimeValue::Known(None),
                attached_mass: RetailRuntimeValue::Unresolved,
            },
            |request| {
                effect.replace(Some(request));
            },
            |request, velocity| {
                assert_eq!(request.explicit_target_raw, None);
                assert_eq!(request.scheduler_mode, 0);
                velocity[0] = 77;
                velocity[1] = 88;
                velocity[2] = 99;
                0xDEAD_u32
            },
        )
        .unwrap();
        assert_eq!(result.selected_effect_raw, Some(0));
        assert_eq!(result.linear_velocity_raw, [77, 88, 99]);
        assert_eq!(effect.borrow().unwrap().signed_effect_raw, 0,);
    }

    #[test]
    fn signed_effect_is_retained_through_mass_150_and_suppressed_above_it() {
        for (mass, expected) in [(150, -7), (151, 0)] {
            let selected = Cell::new(i32::MAX);
            let result = tick_common_dying(
                frame_request(0, 0),
                || CommonDyingEffectInputs {
                    descriptor_effect_raw: RetailRuntimeValue::Known(Some(-7)),
                    attached_mass: RetailRuntimeValue::Known(mass),
                },
                |request| selected.set(request.signed_effect_raw),
                |_, _| (),
            )
            .unwrap();
            assert_eq!(selected.get(), expected);
            assert_eq!(result.selected_effect_raw, Some(expected));
        }
    }

    #[test]
    fn unresolved_effect_inputs_fail_before_effect_or_mover() {
        for (inputs, expected) in [
            (
                CommonDyingEffectInputs {
                    descriptor_effect_raw: RetailRuntimeValue::Unresolved,
                    attached_mass: RetailRuntimeValue::Known(0),
                },
                CommonDyingCallbackError::UnresolvedEffectDescriptor,
            ),
            (
                CommonDyingEffectInputs {
                    descriptor_effect_raw: RetailRuntimeValue::Known(Some(1)),
                    attached_mass: RetailRuntimeValue::Unresolved,
                },
                CommonDyingCallbackError::UnresolvedAttachedMass,
            ),
        ] {
            let effects = Cell::new(0);
            let movers = Cell::new(0);
            assert_eq!(
                tick_common_dying(
                    frame_request(20_000, 0),
                    || inputs,
                    |_| effects.set(effects.get() + 1),
                    |_, _| movers.set(movers.get() + 1),
                ),
                Err(expected)
            );
            assert_eq!(effects.get(), 0);
            assert_eq!(movers.get(), 0);
        }
    }

    #[test]
    fn effect_precedes_mover_mover_return_is_ignored_and_only_post_mover_x_z_are_damped() {
        let log = RefCell::new(Vec::new());
        let result = tick_common_dying(
            frame_request(20_000, 0),
            || CommonDyingEffectInputs {
                descriptor_effect_raw: RetailRuntimeValue::Known(Some(3)),
                attached_mass: RetailRuntimeValue::Known(10),
            },
            |request| {
                assert_eq!(
                    request,
                    InvokeCommonDyingEffect {
                        owner_entity_id: 0x0497_0001,
                        effect_kind: 0x100,
                        signed_effect_raw: 3,
                        enabled: 1,
                        elapsed_micros: 20_000,
                    }
                );
                log.borrow_mut().push("effect");
            },
            |request, velocity| {
                log.borrow_mut().push("mover");
                assert_eq!(request.task_wrapper, 0x1234);
                assert_eq!(request.owner_entity_id, 0x0497_0001);
                assert_eq!(request.type_runtime, "type");
                assert_eq!(request.component_runtime, "component");
                assert_eq!(request.explicit_target_raw, None);
                assert_eq!(request.elapsed_micros, 20_000);
                assert_eq!(request.scheduler_mode, 0);
                *velocity = [10_000, -321, -10_000];
                0xFEED_FACE_u32
            },
        )
        .unwrap();

        assert_eq!(*log.borrow(), ["effect", "mover"]);
        assert_eq!(result.callback_result, CommonDyingCallbackResult::Continue);
        assert_eq!(
            result.linear_velocity_raw,
            [
                damp_common_dying_axis(10_000, 20_000),
                -321,
                damp_common_dying_axis(-10_000, 20_000),
            ]
        );
    }

    #[test]
    fn damping_matches_signed_shift_wrap_and_word_subtraction_edges() {
        assert_eq!(damp_common_dying_axis(1_000, 20_000), 981);
        assert_eq!(damp_common_dying_axis(-1_000, 20_000), -980);
        assert_eq!(damp_common_dying_axis(i16::MIN, 0), i16::MIN);

        let elapsed: u32 = 0x0010_0000;
        let scaled = elapsed.wrapping_shl(11) as i32;
        assert_eq!(scaled, i32::MIN);
        assert_eq!(
            damp_common_dying_axis(i16::MIN, elapsed),
            i16::MIN.wrapping_sub(i16::MIN)
        );
    }

    #[test]
    fn lifetime_truncates_each_frame_wraps_and_expires_strictly_after_9000() {
        let mut state = CommonDyingTaskState::from_elapsed_ms(0);
        assert_eq!(
            state.before_callback(8_999_999),
            CommonDyingCallbackPrefix {
                elapsed_ms: 8_999,
                lifetime_status: CommonDyingLifetimeStatus::WithinLifetime,
            }
        );
        assert_eq!(state.before_callback(999).elapsed_ms, 8_999);
        assert_eq!(
            state.before_callback(1_000).lifetime_status,
            CommonDyingLifetimeStatus::WithinLifetime
        );
        assert_eq!(state.elapsed_ms(), 9_000);
        assert_eq!(
            state.before_callback(1_000).lifetime_status,
            CommonDyingLifetimeStatus::OwnerTransitionDue
        );

        let mut wrapping = CommonDyingTaskState::from_elapsed_ms(u32::MAX);
        assert_eq!(
            wrapping.before_callback(1_000),
            CommonDyingCallbackPrefix {
                elapsed_ms: 0,
                lifetime_status: CommonDyingLifetimeStatus::WithinLifetime,
            }
        );
    }

    #[test]
    fn tagged_result_precedes_expired_lifetime_and_timeout_is_only_a_request() {
        let expired = CommonDyingCallbackPrefix {
            elapsed_ms: 9_001,
            lifetime_status: CommonDyingLifetimeStatus::OwnerTransitionDue,
        };
        let tagged = CommonDyingTaggedResult {
            singleton_address: 0x004B_E170,
            tag: 0x9C01,
        };
        assert_eq!(
            common_dying_after_unwind(
                expired,
                CommonDyingCallbackResult::TaggedOwnerTransition(tagged)
            ),
            CommonDyingAfterUnwindOutcome::RequestOwnerTransition {
                reason: CommonDyingTransitionReason::TaggedSchedulerMode(tagged),
            }
        );
        assert_eq!(
            common_dying_after_unwind(expired, CommonDyingCallbackResult::Continue),
            CommonDyingAfterUnwindOutcome::RequestOwnerTransition {
                reason: CommonDyingTransitionReason::LifetimeExpired,
            }
        );
        assert_eq!(
            common_dying_after_unwind(
                CommonDyingCallbackPrefix {
                    elapsed_ms: 9_000,
                    lifetime_status: CommonDyingLifetimeStatus::WithinLifetime,
                },
                CommonDyingCallbackResult::Continue,
            ),
            CommonDyingAfterUnwindOutcome::Continue
        );
    }

    #[test]
    fn prepared_task_can_be_mapped_without_reordering_suffix() {
        let prepared = CommonDyingTaskState::prepare_after_allocation(
            1,
            &metadata(CommonMoverComponentTopology::default()),
            CommonDyingComponentDescriptors {
                sub_g: RetailRuntimeValue::Known(None),
            },
        )
        .unwrap()
        .map_task(|state| ("dying", state.elapsed_ms()));
        let effects = RefCell::new(Vec::new());
        let task = prepared.apply_suffix(
            || unreachable!("absent Sub-A consumes no RNG"),
            |effect| match effect {
                CommonDyingConstructorEffect::WriteOwnerVerticalVelocity { velocity_raw } => {
                    effects.borrow_mut().push(velocity_raw)
                }
                _ => panic!("only the unconditional velocity write is expected"),
            },
        );
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(ActorTaskSlot::Primary, task);
        assert_eq!(
            owner.state_in_slot(ActorTaskSlot::Primary),
            Some(&("dying", 0))
        );
        assert_eq!(*effects.borrow(), [500]);
    }
}
