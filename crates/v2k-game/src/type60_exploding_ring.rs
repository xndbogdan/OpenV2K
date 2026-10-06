//! Shared construction contract for live Type-60 `Exploding Ring` actors.
//!
//! Retail `FUN_004104B0` and demo `FUN_00410440` route both class-49
//! explosion tails and hard whole-body water impacts through the same generic
//! entity constructor. Component preparation precedes the singleton weighted
//! selector, which still consumes one process RNG word. Class 48 then
//! fallibly publishes its Primary task: task-allocation failure is consumed by
//! the behavior installer and still links a fallback actor with bound output
//! zero, while success publishes output `0xffff`.
//!
//! The callers differ only in request provenance and model policy. Class 49
//! leaves all four model overrides zero and receives Type 60's default model
//! 243. Hard-water construction writes all four overrides to model 132 for a
//! moderate entry or model 130 for a severe entry. Keeping that distinction
//! explicit prevents a runtime allocation from being authenticated later by
//! a coincidental active-model value.

use crate::actor_task_owner::{ActorTaskId, PreparedActorTask};
use crate::damage::DamageProfile;
use crate::entity_collision_state::{
    CommonMoverComponentTopology, EntityTypeRuntimeMetadata, RetailRuntimeValue,
};
use crate::main_base_abort::MainBaseAbortActorLease;
use crate::world_fx::{HARD_WATER_ENTRY_SPLASH_MID_MODEL_ID, HARD_WATER_ENTRY_SPLASH_MODEL_ID};
use v2k_formats::collision::{BehaviorChoice, CommonAxisDescriptor};

pub const TYPE60_RING_ENTITY_TYPE: u32 = 60;
pub const TYPE60_RING_MODEL_ID: usize = 243;
pub const TYPE60_RING_INITIAL_BEHAVIOR_CLASS: u8 = 48;
pub const TYPE60_RING_ALTERNATE_BEHAVIOR_CLASS: u8 = 2;
pub const TYPE60_RING_MASS_RAW: u16 = 1;
pub const TYPE60_RING_CAPABILITY_FLAGS: u32 = 0x40;
pub const TYPE60_RING_INITIAL_HEALTH_RAW: i32 = 1;
pub const TYPE60_RING_INITIALIZER_STATE_RAW: u32 = 0x0002_1084;
pub const TYPE60_RING_INITIAL_CONTROL_OUTPUT_1_RAW: u16 = u16::MAX;
pub const TYPE60_RING_FAILED_TASK_CONTROL_OUTPUT_1_RAW: u16 = 0;
pub const TYPE60_SELECTOR_RNG_DRAWS_AFTER_COMPONENT_PREFLIGHT: u8 = 1;

pub const TYPE60_RING_DAMAGE_PROFILE: DamageProfile = DamageProfile {
    thresholds_raw: [0, 2_000, 200, 0, 200, 0, 0],
    multipliers_q8: [0, 256, 256, 512, 128, 0, 0],
};

pub const TYPE60_RING_COMMON_AXIS_DESCRIPTOR: CommonAxisDescriptor = CommonAxisDescriptor {
    strict_axis_limit_raw: 512,
    raw_word_at_0x04: 8,
};

pub const TYPE60_COMPONENT_TOPOLOGY: CommonMoverComponentTopology = CommonMoverComponentTopology {
    sub_a: false,
    sub_b: false,
    sub_c: false,
    sub_d: false,
    sub_e: false,
    sub_f: false,
    sub_g: false,
    sub_h: false,
    sub_i: false,
    sub_j: false,
    sub_k: true,
    sub_l: false,
    sub_m: false,
    sub_n: false,
    sub_o: false,
};

pub(crate) fn exact_type60_ring_metadata(metadata: &EntityTypeRuntimeMetadata) -> bool {
    metadata.sub_a_propulsion_descriptor == RetailRuntimeValue::Known(None)
        && metadata.sub_b_lateral_descriptor == RetailRuntimeValue::Known(None)
        && metadata.sub_c_lift_descriptor == RetailRuntimeValue::Known(None)
        && metadata.sub_d_steering_descriptor == RetailRuntimeValue::Known(None)
        && metadata.projectile_emitter_descriptor == RetailRuntimeValue::Known(None)
        && metadata.sub_n_payload.is_none()
        && metadata.status_component_descriptor == RetailRuntimeValue::Known(None)
        && metadata.actor_animation_descriptor == RetailRuntimeValue::Known(None)
        && metadata.sub_h_external_frame_descriptor == RetailRuntimeValue::Known(None)
        && metadata.sub_j_attachment_descriptor == RetailRuntimeValue::Known(None)
        && metadata.model_slots == [TYPE60_RING_MODEL_ID as u16; 4]
        && metadata.mass_raw == TYPE60_RING_MASS_RAW
        && metadata.capability_flags == TYPE60_RING_CAPABILITY_FLAGS
        && metadata.initial_health_raw == Some(TYPE60_RING_INITIAL_HEALTH_RAW)
        && metadata.damage_profile == Some(TYPE60_RING_DAMAGE_PROFILE)
        && metadata.accepted_hit_presentation_sound_id == RetailRuntimeValue::Known(None)
        && metadata.death_sound_id == RetailRuntimeValue::Known(None)
        && metadata.constructor_sound_attachment_id == RetailRuntimeValue::Known(None)
        && metadata.generic_hit_sound_id == RetailRuntimeValue::Known(None)
        && metadata.common_mover_topology == RetailRuntimeValue::Known(TYPE60_COMPONENT_TOPOLOGY)
        && metadata.initializer.as_ref().is_some_and(|initializer| {
            initializer.initializer_state_flags_raw == TYPE60_RING_INITIALIZER_STATE_RAW
                && initializer.common_axis_descriptor == TYPE60_RING_COMMON_AXIS_DESCRIPTOR
                && initializer.behavior_choices.as_ref()
                    == [BehaviorChoice {
                        weight_rule_id: 1,
                        weight_multiplier: 1,
                        behavior_class_id: u32::from(TYPE60_RING_INITIAL_BEHAVIOR_CLASS),
                    }]
                && initializer.behavior_rule_ref == 1
                && initializer.alternate_behavior_class_ref
                    == u32::from(TYPE60_RING_ALTERNATE_BEHAVIOR_CLASS)
        })
}

/// Constructor origin retained by each runtime-only Type-60 allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Type60ConstructionProvenance {
    /// Zero model overrides emitted by class 49's explosion tail.
    Class49ExplosionTail,
    /// Hard whole-body water entry using `splashmid`, model 132.
    HardWaterModerate,
    /// Hard whole-body water entry using `splash`, model 130.
    HardWaterSevere,
}

impl Type60ConstructionProvenance {
    pub const fn model_overrides(self) -> [Option<usize>; 4] {
        match self {
            Self::Class49ExplosionTail => [None; 4],
            Self::HardWaterModerate => [Some(HARD_WATER_ENTRY_SPLASH_MID_MODEL_ID); 4],
            Self::HardWaterSevere => [Some(HARD_WATER_ENTRY_SPLASH_MODEL_ID); 4],
        }
    }

    pub const fn presentation_model_id(self) -> usize {
        match self {
            Self::Class49ExplosionTail => TYPE60_RING_MODEL_ID,
            Self::HardWaterModerate => HARD_WATER_ENTRY_SPLASH_MID_MODEL_ID,
            Self::HardWaterSevere => HARD_WATER_ENTRY_SPLASH_MODEL_ID,
        }
    }
}

/// Explicit hard-water policy accepted by the public construction seam.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardWaterType60Severity {
    Moderate,
    Severe,
}

impl HardWaterType60Severity {
    const fn provenance(self) -> Type60ConstructionProvenance {
        match self {
            Self::Moderate => Type60ConstructionProvenance::HardWaterModerate,
            Self::Severe => Type60ConstructionProvenance::HardWaterSevere,
        }
    }
}

/// Narrow hard-water request. It cannot accidentally select class-49 policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HardWaterType60ConstructionRequest {
    position_raw: [i16; 3],
    severity: HardWaterType60Severity,
}

impl HardWaterType60ConstructionRequest {
    pub const fn new(position_raw: [i16; 3], severity: HardWaterType60Severity) -> Self {
        Self {
            position_raw,
            severity,
        }
    }

    pub const fn position_raw(self) -> [i16; 3] {
        self.position_raw
    }

    pub const fn severity(self) -> HardWaterType60Severity {
        self.severity
    }

    pub const fn provenance(self) -> Type60ConstructionProvenance {
        self.severity.provenance()
    }

    pub(crate) const fn into_generic(self) -> Type60ConstructionRequest {
        Type60ConstructionRequest {
            position_raw: self.position_raw,
            provenance: self.provenance(),
        }
    }
}

/// Exact zero-filled 0x4c Type-60 request represented without the
/// allocator-owned world-root handle at offset zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Type60ConstructionRequest {
    position_raw: [i16; 3],
    provenance: Type60ConstructionProvenance,
}

impl Type60ConstructionRequest {
    pub(crate) const fn class49_at(position_raw: [i16; 3]) -> Self {
        Self {
            position_raw,
            provenance: Type60ConstructionProvenance::Class49ExplosionTail,
        }
    }

    pub const fn entity_type(self) -> u32 {
        TYPE60_RING_ENTITY_TYPE
    }

    pub const fn position_raw(self) -> [i16; 3] {
        self.position_raw
    }

    pub const fn rotation_raw(self) -> [u16; 3] {
        [0; 3]
    }

    pub const fn payload_raw(self) -> u32 {
        0
    }

    pub const fn initial_damage_buffer_raw(self) -> i32 {
        0
    }

    pub const fn model_overrides(self) -> [Option<usize>; 4] {
        self.provenance.model_overrides()
    }

    pub const fn provenance(self) -> Type60ConstructionProvenance {
        self.provenance
    }

    pub const fn presentation_model_id(self) -> usize {
        self.provenance.presentation_model_id()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type60ConstructorRngDisposition {
    NoDraw,
    SelectorWord(u16),
}

/// Class-48 Primary state. The callback sequence remains private linear
/// scheduler custody; only the owning production adapter may inspect/advance
/// it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type60ExplodingRingTaskState {
    control_output_1_raw: u16,
    next_callback_sequence: u64,
}

impl Type60ExplodingRingTaskState {
    pub(crate) const fn new() -> Self {
        Self {
            control_output_1_raw: TYPE60_RING_INITIAL_CONTROL_OUTPUT_1_RAW,
            next_callback_sequence: 1,
        }
    }

    pub const fn control_output_1_raw(self) -> u16 {
        self.control_output_1_raw
    }

    pub(crate) const fn next_callback_sequence(self) -> u64 {
        self.next_callback_sequence
    }

    /// Atomically publish one callback's bound output and consume its receipt
    /// sequence. Exhausted receipt space fails closed without mutating either
    /// field, so the production owner can retain custody and report a block.
    pub(crate) fn commit_callback_control_output_1_raw(
        &mut self,
        control_output_1_raw: u16,
    ) -> bool {
        let Some(next_callback_sequence) = self.next_callback_sequence.checked_add(1) else {
            return false;
        };
        self.control_output_1_raw = control_output_1_raw;
        self.next_callback_sequence = next_callback_sequence;
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Type60ExplodingRingTaskLease {
    actor: MainBaseAbortActorLease,
    task_id: ActorTaskId,
}

impl Type60ExplodingRingTaskLease {
    pub const fn actor(self) -> MainBaseAbortActorLease {
        self.actor
    }

    pub const fn task_id(self) -> ActorTaskId {
        self.task_id
    }

    pub(crate) const fn issue(actor: MainBaseAbortActorLease, task_id: ActorTaskId) -> Self {
        Self { actor, task_id }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type60InitializerDisposition {
    PrimaryPublished {
        task_lease: Type60ExplodingRingTaskLease,
        control_output_1_raw: u16,
    },
    /// The selected initializer propagates the nested task-allocation error;
    /// the shared behavior installer consumes it, publishes the unnamed
    /// fallback, and leaves the zero-filled bound output at zero.
    FallbackPublishedAfterPrimaryAllocationFailure { control_output_1_raw: u16 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type60ConstructionReceipt {
    actor: MainBaseAbortActorLease,
    provenance: Type60ConstructionProvenance,
    constructor_rng: Type60ConstructorRngDisposition,
    initializer: Type60InitializerDisposition,
}

impl Type60ConstructionReceipt {
    pub const fn actor(self) -> MainBaseAbortActorLease {
        self.actor
    }

    pub const fn provenance(self) -> Type60ConstructionProvenance {
        self.provenance
    }

    pub const fn constructor_rng(self) -> Type60ConstructorRngDisposition {
        self.constructor_rng
    }

    pub const fn initializer(self) -> Type60InitializerDisposition {
        self.initializer
    }

    pub const fn primary_task_lease(self) -> Option<Type60ExplodingRingTaskLease> {
        match self.initializer {
            Type60InitializerDisposition::PrimaryPublished { task_lease, .. } => Some(task_lease),
            Type60InitializerDisposition::FallbackPublishedAfterPrimaryAllocationFailure {
                ..
            } => None,
        }
    }

    pub(crate) const fn linked_with_primary(
        actor: MainBaseAbortActorLease,
        provenance: Type60ConstructionProvenance,
        task_id: ActorTaskId,
        selector_rng_word: u16,
    ) -> Self {
        Self {
            actor,
            provenance,
            constructor_rng: Type60ConstructorRngDisposition::SelectorWord(selector_rng_word),
            initializer: Type60InitializerDisposition::PrimaryPublished {
                task_lease: Type60ExplodingRingTaskLease::issue(actor, task_id),
                control_output_1_raw: TYPE60_RING_INITIAL_CONTROL_OUTPUT_1_RAW,
            },
        }
    }

    pub(crate) const fn linked_after_primary_allocation_failure(
        actor: MainBaseAbortActorLease,
        provenance: Type60ConstructionProvenance,
        selector_rng_word: u16,
    ) -> Self {
        Self {
            actor,
            provenance,
            constructor_rng: Type60ConstructorRngDisposition::SelectorWord(selector_rng_word),
            initializer:
                Type60InitializerDisposition::FallbackPublishedAfterPrimaryAllocationFailure {
                    control_output_1_raw: TYPE60_RING_FAILED_TASK_CONTROL_OUTPUT_1_RAW,
                },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type60ConstructionOutcome {
    /// Raw entity or component preparation failed before weighted selection.
    RejectedBeforeSelector,
    /// Behavior-context preparation failed after the singleton selector draw.
    RejectedAfterSelector { selector_rng_word: u16 },
    /// A named class-48 actor or its task-allocation fallback is linked.
    ActorLinked(Type60ConstructionReceipt),
}

impl Type60ConstructionOutcome {
    pub const fn constructor_rng(self) -> Type60ConstructorRngDisposition {
        match self {
            Self::RejectedBeforeSelector => Type60ConstructorRngDisposition::NoDraw,
            Self::RejectedAfterSelector { selector_rng_word } => {
                Type60ConstructorRngDisposition::SelectorWord(selector_rng_word)
            }
            Self::ActorLinked(receipt) => receipt.constructor_rng(),
        }
    }

    pub const fn primary_task_lease(self) -> Option<Type60ExplodingRingTaskLease> {
        match self {
            Self::ActorLinked(receipt) => receipt.primary_task_lease(),
            Self::RejectedBeforeSelector | Self::RejectedAfterSelector { .. } => None,
        }
    }
}

/// Injectable versions of Type 60's three fallible construction stages.
pub(crate) trait Type60Allocator {
    fn prepare_components(&mut self, request: Type60ConstructionRequest) -> bool;
    fn prepare_behavior_context(&mut self) -> bool;
    fn prepare_primary_task(
        &mut self,
        state: Type60ExplodingRingTaskState,
    ) -> Option<PreparedActorTask<Type60ExplodingRingTaskState>>;
}

pub(crate) struct HostType60Allocator;

impl Type60Allocator for HostType60Allocator {
    fn prepare_components(&mut self, _request: Type60ConstructionRequest) -> bool {
        true
    }

    fn prepare_behavior_context(&mut self) -> bool {
        true
    }

    fn prepare_primary_task(
        &mut self,
        state: Type60ExplodingRingTaskState,
    ) -> Option<PreparedActorTask<Type60ExplodingRingTaskState>> {
        Some(PreparedActorTask::new(state))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor_task_owner::{ActorTaskOwner, ActorTaskSlot};

    #[test]
    fn provenance_owns_all_four_model_overrides() {
        let position_raw = [-10, 20, -30];
        let class49 = Type60ConstructionRequest::class49_at(position_raw);
        assert_eq!(class49.entity_type(), TYPE60_RING_ENTITY_TYPE);
        assert_eq!(class49.position_raw(), position_raw);
        assert_eq!(class49.rotation_raw(), [0; 3]);
        assert_eq!(class49.payload_raw(), 0);
        assert_eq!(class49.initial_damage_buffer_raw(), 0);
        assert_eq!(class49.model_overrides(), [None; 4]);
        assert_eq!(class49.presentation_model_id(), TYPE60_RING_MODEL_ID);

        let moderate = HardWaterType60ConstructionRequest::new(
            position_raw,
            HardWaterType60Severity::Moderate,
        )
        .into_generic();
        assert_eq!(
            moderate.model_overrides(),
            [Some(HARD_WATER_ENTRY_SPLASH_MID_MODEL_ID); 4]
        );
        assert_eq!(
            moderate.provenance(),
            Type60ConstructionProvenance::HardWaterModerate
        );

        let severe =
            HardWaterType60ConstructionRequest::new(position_raw, HardWaterType60Severity::Severe)
                .into_generic();
        assert_eq!(
            severe.model_overrides(),
            [Some(HARD_WATER_ENTRY_SPLASH_MODEL_ID); 4]
        );
        assert_eq!(
            severe.provenance(),
            Type60ConstructionProvenance::HardWaterSevere
        );
    }

    #[test]
    fn successful_receipt_publishes_linear_task_lease_at_sequence_one() {
        let actor = MainBaseAbortActorLease {
            entity_id: 100,
            allocation_identity: 701,
        };
        let mut tasks = ActorTaskOwner::new();
        let task_id = tasks.replace_prepared(ActorTaskSlot::Primary, PreparedActorTask::new(()));
        let receipt = Type60ConstructionReceipt::linked_with_primary(
            actor,
            Type60ConstructionProvenance::HardWaterModerate,
            task_id,
            0xabcd,
        );
        let Type60InitializerDisposition::PrimaryPublished { task_lease, .. } =
            receipt.initializer()
        else {
            panic!("successful construction publishes one Primary lease")
        };
        assert_eq!(task_lease.actor(), actor);
        assert_eq!(task_lease.task_id(), task_id);
        assert_eq!(
            receipt.constructor_rng(),
            Type60ConstructorRngDisposition::SelectorWord(0xabcd)
        );
        assert_eq!(
            receipt.provenance(),
            Type60ConstructionProvenance::HardWaterModerate
        );

        let mut state = Type60ExplodingRingTaskState::new();
        assert_eq!(state.next_callback_sequence(), 1);
        assert!(state.commit_callback_control_output_1_raw(0xf97f));
        assert_eq!(state.next_callback_sequence(), 2);
        assert_eq!(state.control_output_1_raw(), 0xf97f);
    }

    #[test]
    fn construction_outcomes_retain_pre_and_post_selector_boundaries() {
        assert_eq!(
            Type60ConstructionOutcome::RejectedBeforeSelector.constructor_rng(),
            Type60ConstructorRngDisposition::NoDraw
        );
        assert_eq!(
            Type60ConstructionOutcome::RejectedAfterSelector {
                selector_rng_word: 77,
            }
            .constructor_rng(),
            Type60ConstructorRngDisposition::SelectorWord(77)
        );
        assert_eq!(TYPE60_SELECTOR_RNG_DRAWS_AFTER_COMPONENT_PREFLIGHT, 1);
    }

    #[test]
    fn exhausted_callback_receipt_fails_without_partial_control_commit() {
        let mut state = Type60ExplodingRingTaskState {
            control_output_1_raw: 0x4567,
            next_callback_sequence: u64::MAX,
        };

        assert!(!state.commit_callback_control_output_1_raw(0x1234));
        assert_eq!(state.control_output_1_raw(), 0x4567);
        assert_eq!(state.next_callback_sequence(), u64::MAX);
    }
}
