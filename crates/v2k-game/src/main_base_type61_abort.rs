//! Type-61 Main Base-abort contract.
//!
//! Authored Level-1 pickups still use the captured spawn table. Factory
//! products authenticate the producing Type66 allocation in any ordinary world
//! that constructed them; overlay 13 spawn 23 is not the admission key.
//!
//! Retail `FUN_00410C10 -> FUN_0040DB80 -> FUN_0040BD20` selects class 49,
//! emits the class-authored burst and radial damage synchronously, clears the
//! dying actor's three task slots, attempts one zero-filled Type-60 allocation,
//! and reaches the shared idempotent deferred-destroy helper. An earlier
//! Power-Up owner makes only that final helper a no-op; allocation failure is
//! deliberately non-fatal.
//!
//! The Type-60 tail is based on retail `FUN_004104B0` and `FUN_0040B290`
//! (with the matched demo chain).  Component preparation precedes the
//! singleton weighted selector, which still consumes exactly one process RNG
//! word.  The Type-60 initializer then clears T and S and fallibly constructs
//! its Primary ring task.  Success publishes bound output 1 as `0xffff`;
//! task-allocation failure leaves that zero-filled output at zero and causes
//! the behavior installer to publish its unnamed fallback.  The outer entity
//! constructor still links that fallback actor.  Either linked tail is
//! eligible for the same mutation-sensitive Main Base sweep and subsequently
//! takes its authored class-2 alternate death route.
//!
//! The retail construction chain is `004104B0 -> 0040D4A0 -> 004381F0 ->
//! 0040AC60 -> 00425680 -> 00438340 -> 0040ABE0/0040ABB0 -> 0040C6B0 ->
//! 0040B290 -> 00406D20`. Demo preserves the order at `00410440 -> 0040D4A0
//! -> 00437C30 -> 0040AC70 -> 00425550 -> 00437D80 -> 0040ABF0/0040ABC0 ->
//! 0040C6C0 -> 0040B2A0 -> 00406D90`. This is why the contract distinguishes
//! rejection before selection, rejection after the selector draw, and the
//! post-draw fallback actor which is nevertheless published.

use crate::damage::{DamagePacket, DamageProfile};
use crate::entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue};
use crate::main_base_abort::{
    MainBaseAbortActorLease, MainBaseAbortActorObservation, MainBaseAbortActorRoute,
    MainBaseDeferredDestroyCommit, MainBaseDeferredDestroyCustodyBlock,
    MainBaseExternalDeferredDestroyOwner,
};
use crate::radial_damage::RadialDamageTemplate;
use crate::type60_exploding_ring::{
    Type60ConstructionOutcome, Type60ExplodingRingTaskLease, Type60InitializerDisposition,
    TYPE60_COMPONENT_TOPOLOGY,
};
use crate::world_fx::WorldFx;
use v2k_formats::collision::{BehaviorChoice, CommonAxisDescriptor};

pub const LEVEL_ONE_TYPE61_ENTITY_TYPE: u32 = 61;
pub const LEVEL_ONE_TYPE61_SPAWN_INDICES: [usize; 3] = [32, 33, 34];
pub const LEVEL_ONE_TYPE61_AUTHORED_MODEL_ID: usize = 82;
/// Section-8 model header `+0x08` for the canonical Type-61 model. Factory
/// products cannot carry a Section-13 model override, so this is also their
/// exact class-49 scatter extent.
pub const LEVEL_ONE_TYPE61_DEFAULT_MODEL_EXTENT_RAW: u16 = 128;
pub const LEVEL_ONE_TYPE61_INITIAL_BEHAVIOR_CLASS: u8 = 23;
pub const LEVEL_ONE_TYPE61_DEATH_BEHAVIOR_CLASS: u8 = 49;
pub const LEVEL_ONE_TYPE61_DEATH_STYLE_ADDRESS: u32 = 0x004C_71E0;
pub const LEVEL_ONE_TYPE61_MASS_RAW: u16 = 1;
pub const LEVEL_ONE_TYPE61_CAPABILITY_FLAGS: u32 = 0x40;
pub const LEVEL_ONE_TYPE61_INITIAL_HEALTH_RAW: i32 = 900_000;
pub const LEVEL_ONE_TYPE61_INITIALIZER_STATE_RAW: u32 = 0x0002_3205;
pub const LEVEL_ONE_TYPE61_CONSTRUCTOR_SOUND_ATTACHMENT_ID: u16 = 44;
/// Exact post-constructor state for a factory request built without terrain.
/// `FUN_004104B0` applies the fully-above-surface bit before Type 61's policy
/// and the infallible class-23 initializer publish.
pub const FACTORY_TYPE61_NEWBORN_STATE_RAW: u32 = 0x0E40_8805;

/// Class 49 is installed directly from the alternate slot; it does not run a
/// weighted selector. The shared explosion tail consumes one randomized sound
/// word. Dynamic Type-60 construction separately owns its selector accounting.
pub const TYPE61_ALTERNATE_SELECTOR_RNG_DRAWS: u8 = 0;
pub const TYPE61_BURST_SOUND_RNG_DRAWS: u8 = 1;

pub const TYPE61_DAMAGE_PROFILE: DamageProfile = DamageProfile {
    thresholds_raw: [0, 10_000, 10_000, 10_000, 4_000, 4_000, 0],
    multipliers_q8: [0, 256, 256, 256, 256, 0, 0],
};

/// Exact Section-13 facts shared by retail presentation tiers 0--3 and demo
/// tiers 0--1.  Model slots are effective runtime ids after zero overrides
/// have fallen back to the Type-61 Section-12 row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MainBaseType61CapturedSpawn {
    pub spawn_index: usize,
    pub position_raw: [i16; 3],
    pub power_up_payload_packed: u32,
    pub rotation_raw: [u16; 3],
    pub initial_damage_buffer_raw: i32,
    pub model_slots: [usize; 4],
    pub active_model_id: usize,
    /// Section-8 model header `+0x08`, consumed by the class-49 scatter.
    pub active_model_extent_raw: u16,
    /// Accepted pre-abort live state at world-track sample 5560.
    pub preabort_state_raw: u32,
}

impl MainBaseType61CapturedSpawn {
    pub const fn for_spawn(spawn_index: usize) -> Option<Self> {
        match spawn_index {
            32 => Some(LEVEL_ONE_TYPE61_CAPTURED_SPAWNS[0]),
            33 => Some(LEVEL_ONE_TYPE61_CAPTURED_SPAWNS[1]),
            34 => Some(LEVEL_ONE_TYPE61_CAPTURED_SPAWNS[2]),
            _ => None,
        }
    }
}

pub const LEVEL_ONE_TYPE61_CAPTURED_SPAWNS: [MainBaseType61CapturedSpawn; 3] = [
    MainBaseType61CapturedSpawn {
        spawn_index: 32,
        position_raw: [-29_440, -512, 22_528],
        power_up_payload_packed: 0x0000_003C,
        rotation_raw: [0; 3],
        initial_damage_buffer_raw: 0,
        model_slots: [LEVEL_ONE_TYPE61_AUTHORED_MODEL_ID; 4],
        active_model_id: LEVEL_ONE_TYPE61_AUTHORED_MODEL_ID,
        active_model_extent_raw: LEVEL_ONE_TYPE61_DEFAULT_MODEL_EXTENT_RAW,
        preabort_state_raw: 0x0840_8805,
    },
    MainBaseType61CapturedSpawn {
        spawn_index: 33,
        position_raw: [7_424, -3_072, -25_344],
        power_up_payload_packed: 0x0000_003F,
        rotation_raw: [0; 3],
        initial_damage_buffer_raw: 0,
        model_slots: [138; 4],
        active_model_id: 138,
        active_model_extent_raw: 75,
        preabort_state_raw: 0x0820_8805,
    },
    MainBaseType61CapturedSpawn {
        spawn_index: 34,
        position_raw: [22_272, -1_536, 21_248],
        power_up_payload_packed: 0x0000_C802,
        rotation_raw: [0; 3],
        initial_damage_buffer_raw: 0,
        model_slots: [LEVEL_ONE_TYPE61_AUTHORED_MODEL_ID; 4],
        active_model_id: LEVEL_ONE_TYPE61_AUTHORED_MODEL_ID,
        active_model_extent_raw: LEVEL_ONE_TYPE61_DEFAULT_MODEL_EXTENT_RAW,
        preabort_state_raw: 0x0820_8805,
    },
];

pub(crate) fn exact_level_one_type61_metadata(metadata: &EntityTypeRuntimeMetadata) -> bool {
    exact_optional_component_absence(metadata)
        && metadata.model_slots == [LEVEL_ONE_TYPE61_AUTHORED_MODEL_ID as u16; 4]
        && metadata.mass_raw == LEVEL_ONE_TYPE61_MASS_RAW
        && metadata.capability_flags == LEVEL_ONE_TYPE61_CAPABILITY_FLAGS
        && metadata.initial_health_raw == Some(LEVEL_ONE_TYPE61_INITIAL_HEALTH_RAW)
        && metadata.damage_profile == Some(TYPE61_DAMAGE_PROFILE)
        && metadata.accepted_hit_presentation_sound_id == RetailRuntimeValue::Known(None)
        && metadata.death_sound_id == RetailRuntimeValue::Known(None)
        && metadata.constructor_sound_attachment_id
            == RetailRuntimeValue::Known(Some(LEVEL_ONE_TYPE61_CONSTRUCTOR_SOUND_ATTACHMENT_ID))
        && metadata.generic_hit_sound_id == RetailRuntimeValue::Known(None)
        && metadata.common_mover_topology == RetailRuntimeValue::Known(TYPE60_COMPONENT_TOPOLOGY)
        && metadata.initializer.as_ref().is_some_and(|initializer| {
            initializer.initializer_state_flags_raw == LEVEL_ONE_TYPE61_INITIALIZER_STATE_RAW
                && initializer.common_axis_descriptor == CommonAxisDescriptor::default()
                && initializer.behavior_choices.as_ref()
                    == [BehaviorChoice {
                        weight_rule_id: 1,
                        weight_multiplier: 1,
                        behavior_class_id: u32::from(LEVEL_ONE_TYPE61_INITIAL_BEHAVIOR_CLASS),
                    }]
                && initializer.behavior_rule_ref == 1
                && initializer.alternate_behavior_class_ref
                    == u32::from(LEVEL_ONE_TYPE61_DEATH_BEHAVIOR_CLASS)
        })
}

fn exact_optional_component_absence(metadata: &EntityTypeRuntimeMetadata) -> bool {
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MainBaseType61LogicalOwner {
    pub entity_id: u32,
    pub entity_type: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseType61NetworkSession {
    SoloNetworkingDisabled,
    NetworkingEnabled,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseType61NetworkDisposition {
    Kind2SuppressedBySoloSession,
}

/// Live inputs that cannot be recovered safely from the Type-61 type row.
/// In particular, the extent belongs to the selected Section-8 model (82 or
/// spawn 33's override 138), so the caller supplies it and the live adapter
/// authenticates it against the exact captured active-model row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MainBaseType61DeathRequest {
    pub active_model_extent_raw: u16,
    pub sea_level_raw: Option<i16>,
    pub logical_owner: MainBaseType61LogicalOwner,
    pub network_session: MainBaseType61NetworkSession,
}

pub const TYPE61_RADIAL_DAMAGE_BASE: RadialDamageTemplate = RadialDamageTemplate {
    inner_radius_raw: 512,
    outer_radius_raw: 1_024,
    impulse_raw: 2_000,
    packet: DamagePacket {
        channels: [1, 3],
        amounts_raw: [4_000, 4_000],
    },
    trailing_raw: [0; 2],
};

pub const fn type61_radial_damage_template(
    logical_owner: MainBaseType61LogicalOwner,
) -> RadialDamageTemplate {
    RadialDamageTemplate {
        trailing_raw: [
            logical_owner.entity_type as i32,
            logical_owner.entity_id as i32,
        ],
        ..TYPE61_RADIAL_DAMAGE_BASE
    }
}

/// External owner of `FUN_004566E0`'s static-object half.
///
/// The bounded entity adapter can preflight the complete dynamic live-list
/// walk itself, but the terrain/static-object scheduler belongs to the world
/// runtime.  Splitting this authority into a read-only admission and an
/// infallible commit prevents an unresolved static target from leaving the
/// earlier generic-death prefix or particle burst half-applied.  Implementors
/// may retain an opaque prepared plan between these calls.
pub trait MainBaseType61StaticRadialEffects {
    /// Opaque, one-shot authority binding the exact origin/template admitted
    /// by preflight. It need not expose static-object internals to the entity
    /// transaction.
    type Prepared;

    fn preflight_static_radial(
        &mut self,
        origin_raw: [i16; 3],
        template: RadialDamageTemplate,
    ) -> Option<Self::Prepared>;

    fn commit_static_radial(&mut self, world_fx: &mut WorldFx, prepared: Self::Prepared);

    /// Section-8 header `+08` for a Hive67 dying model, used by `FUN_00440950`.
    fn hive_dying_burst_model_extent_raw(&self, model_id: usize) -> Option<u16> {
        let _ = model_id;
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseType61DeathOutcome {
    RemoteOwnedNoOp {
        entity_id: u32,
        entity_type: u32,
    },
    AlreadyDyingNoOp {
        entity_id: u32,
        entity_type: u32,
    },
    ExplodeWithRingCommitted {
        entity_id: u32,
        released_constructor_sound_attachment_id: u16,
        request: MainBaseType61DeathRequest,
        network: MainBaseType61NetworkDisposition,
        radial: RadialDamageTemplate,
        dynamic_radial_accepted_targets: usize,
        ring: Type60ConstructionOutcome,
        deferred_destroy: MainBaseDeferredDestroyCommit,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseType61DeathAdvance {
    Advanced {
        outcome: MainBaseType61DeathOutcome,
        next_actor: Option<MainBaseAbortActorObservation>,
    },
    SuccessorUnavailableAfterCommit {
        outcome: MainBaseType61DeathOutcome,
    },
}

/// Missing evidence or custody before generic death's first mutation.  Ring
/// allocation rejection is intentionally absent: retail has already committed
/// the explosion/radial/task-clear prefix and continues to source destruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseType61DeathBlock {
    EntityMissing,
    ActorLeaseMismatch {
        expected: MainBaseAbortActorLease,
        actual: MainBaseAbortActorLease,
    },
    NotOrdinaryRoute(MainBaseAbortActorRoute),
    NotFreshNewGameFirstWorld,
    UnsupportedEntityType {
        actual: u32,
    },
    UnauthenticatedSpawn {
        actual: Option<usize>,
    },
    CapturedSpawnStateMismatch,
    FactoryBirthProvenanceMismatch,
    TypeMetadataUnavailable,
    TypeMetadataMismatch,
    RingTypeMetadataUnavailable,
    RingTypeMetadataMismatch,
    ActiveModelMismatch,
    InitialBehaviorMismatch,
    CurrentBehaviorContextAbsent,
    CurrentBehaviorContextMismatch,
    UnexpectedPublishedTask,
    ConstructorSoundAttachmentStateMismatch,
    LogicalOwnerUnresolved,
    LogicalOwnerUnavailable,
    LogicalOwnerMismatch {
        expected: MainBaseType61LogicalOwner,
        actual: MainBaseType61LogicalOwner,
    },
    SeaLevelMismatch {
        expected: i16,
        actual: Option<i16>,
    },
    RemoteOwnershipUnresolved,
    DyingStateUnresolved,
    NetworkSessionUnsupported(MainBaseType61NetworkSession),
    StaticRadialDamagePreflightUnresolved,
    DynamicRadialDamagePreflightUnresolved,
    DeferredDestroyCustody(MainBaseDeferredDestroyCustodyBlock),
    DeferredDestroyOwnerUnsupported(MainBaseExternalDeferredDestroyOwner),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseType60RingDeathOutcome {
    RemoteOwnedNoOp {
        entity_id: u32,
        entity_type: u32,
    },
    AlreadyDyingNoOp {
        entity_id: u32,
        entity_type: u32,
    },
    DeferredDestroyStaged {
        entity_id: u32,
        initializer: Type60InitializerDisposition,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseType60RingDeathAdvance {
    Advanced {
        outcome: MainBaseType60RingDeathOutcome,
        next_actor: Option<MainBaseAbortActorObservation>,
    },
    SuccessorUnavailableAfterCommit {
        outcome: MainBaseType60RingDeathOutcome,
    },
}

/// Missing custody before a linked Type-60 actor can take its authored class-2
/// alternate. Both the named class-48/Primary form and the unnamed
/// fallback/no-Primary form are admitted. A same-sweep class-49 tail must be
/// fresh and unscheduled, or already registered by the shared native terminal.
/// Every supplied class49/hard-water owner must match the exact task and
/// callback sequence removed by the atomic Main Base transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseType60RingDeathBlock {
    EntityMissing,
    ActorLeaseMismatch {
        expected: MainBaseAbortActorLease,
        actual: MainBaseAbortActorLease,
    },
    NotOrdinaryRoute(MainBaseAbortActorRoute),
    UnsupportedEntityType {
        actual: u32,
    },
    ConstructionProvenanceMismatch,
    TypeMetadataUnavailable,
    TypeMetadataMismatch,
    LiveModelMismatch,
    ConstructorStateMismatch,
    CurrentBehaviorContextMismatch,
    InitializerTaskStateMismatch,
    SchedulerOwnerUnexpected,
    SchedulerOwnerMissing,
    SchedulerOwnerTaskLeaseMismatch {
        expected: Type60ExplodingRingTaskLease,
        actual: Type60ExplodingRingTaskLease,
    },
    SchedulerOwnerSequenceMismatch {
        expected: u64,
        actual: u64,
    },
    PresentationStateMismatch,
    RemoteOwnershipUnresolved,
    DyingStateUnresolved,
    DeferredDestroyStateUnresolved,
    DeferredDestroyAlreadyPending,
    DeferredDestroyAlreadyQueued,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity_collision_state::EntityInitializerSpec;
    use crate::type60_exploding_ring::{
        exact_type60_ring_metadata, TYPE60_RING_ALTERNATE_BEHAVIOR_CLASS,
        TYPE60_RING_COMMON_AXIS_DESCRIPTOR, TYPE60_RING_DAMAGE_PROFILE, TYPE60_RING_ENTITY_TYPE,
        TYPE60_RING_INITIALIZER_STATE_RAW, TYPE60_RING_INITIAL_BEHAVIOR_CLASS,
        TYPE60_RING_INITIAL_HEALTH_RAW, TYPE60_RING_MODEL_ID,
    };

    fn exact_metadata(entity_type: u32) -> EntityTypeRuntimeMetadata {
        let (model, health, profile, attachment, state, axis, initial, alternate) =
            if entity_type == LEVEL_ONE_TYPE61_ENTITY_TYPE {
                (
                    LEVEL_ONE_TYPE61_AUTHORED_MODEL_ID,
                    LEVEL_ONE_TYPE61_INITIAL_HEALTH_RAW,
                    TYPE61_DAMAGE_PROFILE,
                    Some(LEVEL_ONE_TYPE61_CONSTRUCTOR_SOUND_ATTACHMENT_ID),
                    LEVEL_ONE_TYPE61_INITIALIZER_STATE_RAW,
                    CommonAxisDescriptor::default(),
                    LEVEL_ONE_TYPE61_INITIAL_BEHAVIOR_CLASS,
                    LEVEL_ONE_TYPE61_DEATH_BEHAVIOR_CLASS,
                )
            } else {
                (
                    TYPE60_RING_MODEL_ID,
                    TYPE60_RING_INITIAL_HEALTH_RAW,
                    TYPE60_RING_DAMAGE_PROFILE,
                    None,
                    TYPE60_RING_INITIALIZER_STATE_RAW,
                    TYPE60_RING_COMMON_AXIS_DESCRIPTOR,
                    TYPE60_RING_INITIAL_BEHAVIOR_CLASS,
                    TYPE60_RING_ALTERNATE_BEHAVIOR_CLASS,
                )
            };
        EntityTypeRuntimeMetadata {
            cured_model_presentation_sound_id: RetailRuntimeValue::Unresolved,
            common_world_effects: RetailRuntimeValue::Unresolved,
            detailed_sound_policy: RetailRuntimeValue::Unresolved,
            model_slots: [model as u16; 4],
            mass_raw: 1,
            capability_flags: 0x40,
            initial_health_raw: Some(health),
            damage_profile: Some(profile),
            accepted_hit_presentation_sound_id: RetailRuntimeValue::Known(None),
            infected_model_presentation_sound_id: RetailRuntimeValue::Known(None),
            death_sound_id: RetailRuntimeValue::Known(None),
            target_warning_sound_id: RetailRuntimeValue::Unresolved,
            constructor_sound_attachment_id: RetailRuntimeValue::Known(attachment),
            generic_hit_sound_id: RetailRuntimeValue::Known(None),
            search_attack_optional_prelude_sound_id: RetailRuntimeValue::Unresolved,
            search_attack_aim_sound_id: RetailRuntimeValue::Unresolved,
            search_attack_aim_sound_period_raw: RetailRuntimeValue::Unresolved,
            run_away_optional_sound_id: RetailRuntimeValue::Unresolved,
            run_away_sound_period_raw: RetailRuntimeValue::Unresolved,
            terrain_contact_task_lifetime_ms: RetailRuntimeValue::Unresolved,
            sub_a_propulsion_descriptor: RetailRuntimeValue::Known(None),
            sub_b_lateral_descriptor: RetailRuntimeValue::Known(None),
            sub_c_lift_descriptor: RetailRuntimeValue::Known(None),
            sub_d_steering_descriptor: RetailRuntimeValue::Known(None),
            sub_f_swimming_descriptor: RetailRuntimeValue::Known(None),
            model_variable_count_raw: RetailRuntimeValue::Unresolved,
            projectile_emitter_descriptor: RetailRuntimeValue::Known(None),
            sub_n_payload: None,
            status_component_descriptor: RetailRuntimeValue::Known(None),
            actor_animation_descriptor: RetailRuntimeValue::Known(None),
            sub_h_external_frame_descriptor: RetailRuntimeValue::Known(None),
            sub_j_attachment_descriptor: RetailRuntimeValue::Known(None),
            common_mover_topology: RetailRuntimeValue::Known(TYPE60_COMPONENT_TOPOLOGY),
            initializer: Some(EntityInitializerSpec {
                initializer_state_flags_raw: state,
                common_axis_descriptor: axis,
                behavior_choices: vec![BehaviorChoice {
                    weight_rule_id: 1,
                    weight_multiplier: 1,
                    behavior_class_id: u32::from(initial),
                }]
                .into_boxed_slice(),
                behavior_rule_ref: 1,
                alternate_behavior_class_ref: u32::from(alternate),
            }),
            common_mover_gkl_payloads: RetailRuntimeValue::Unresolved,
        }
    }

    #[test]
    fn exact_spawns_retain_positions_payloads_and_override_identity() {
        assert_eq!(MainBaseType61CapturedSpawn::for_spawn(31), None);
        assert_eq!(MainBaseType61CapturedSpawn::for_spawn(35), None);
        assert_eq!(
            MainBaseType61CapturedSpawn::for_spawn(32),
            Some(LEVEL_ONE_TYPE61_CAPTURED_SPAWNS[0])
        );
        assert_eq!(LEVEL_ONE_TYPE61_CAPTURED_SPAWNS[1].model_slots, [138; 4]);
        assert_eq!(
            LEVEL_ONE_TYPE61_CAPTURED_SPAWNS.map(|spawn| spawn.power_up_payload_packed),
            [0x3c, 0x3f, 0xc802]
        );
    }

    #[test]
    fn cumulative_type61_and_type60_rows_are_not_interchangeable() {
        let type61 = exact_metadata(LEVEL_ONE_TYPE61_ENTITY_TYPE);
        let type60 = exact_metadata(TYPE60_RING_ENTITY_TYPE);
        assert!(exact_level_one_type61_metadata(&type61));
        assert!(!exact_type60_ring_metadata(&type61));
        assert!(exact_type60_ring_metadata(&type60));
        assert!(!exact_level_one_type61_metadata(&type60));

        let mut wrong_type60 = type60;
        wrong_type60
            .initializer
            .as_mut()
            .unwrap()
            .common_axis_descriptor
            .raw_word_at_0x04 = 0;
        assert!(!exact_type60_ring_metadata(&wrong_type60));
    }

    #[test]
    fn radial_template_binds_logical_owner_in_retail_word_order() {
        let owner = MainBaseType61LogicalOwner {
            entity_id: 0xF123_4567,
            entity_type: 61,
        };
        let template = type61_radial_damage_template(owner);
        assert_eq!(template.inner_radius_raw, 512);
        assert_eq!(template.outer_radius_raw, 1_024);
        assert_eq!(template.impulse_raw, 2_000);
        assert_eq!(template.packet.channels, [1, 3]);
        assert_eq!(template.packet.amounts_raw, [4_000, 4_000]);
        assert_eq!(template.trailing_raw, [61, 0xF123_4567_u32 as i32]);
    }

    #[test]
    fn captured_model_extents_and_preabort_states_are_spawn_specific() {
        assert_eq!(
            LEVEL_ONE_TYPE61_CAPTURED_SPAWNS.map(|spawn| spawn.active_model_extent_raw),
            [128, 75, 128]
        );
        assert_eq!(
            LEVEL_ONE_TYPE61_CAPTURED_SPAWNS.map(|spawn| spawn.preabort_state_raw),
            [0x0840_8805, 0x0820_8805, 0x0820_8805]
        );
    }
}
