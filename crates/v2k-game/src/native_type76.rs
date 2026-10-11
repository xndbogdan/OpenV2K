//! Native ordinary Type76/Type77 construction on the shared ground host.
//!
//! Six Type76 rows (worlds 26 and 31) and eight Type77 rows (worlds 26, 37,
//! 39 and 40) own A/B/C/D/E/H with no J. Both roots weigh Follow Beacons 33,
//! Trash Furniture 26 (FurnitureNearby) and Search 7 (UnderAttack); Type77
//! adds Defecate Virus 4 and Move About Aimlessly 5. Each class runs its
//! existing shared program. Both take class12 (including E370 drowning).

pub mod aim;
mod construction;
pub mod contact;
pub(crate) mod impact;
pub(crate) mod profile;
#[cfg(test)]
mod tests;
pub(crate) use construction::{publish_native_type76, NativeType76ConstructionRequest};

use crate::{
    actor_task_owner::ActorTaskSlot,
    common_mover::sub_d::{Type9SubDFrameOwner, Type9SubDRuntime, INTRO2_TYPE58_SUB_D},
    entity::{Entity, EntityManager},
    entity_behavior::{
        select_initial_behavior, BehaviorContextRuntime, BehaviorSelection, BehaviorWeightRule,
    },
    entity_collision_state::{
        CommonMoverComponentTopology, EntityTypeRuntimeMetadata, RetailRuntimeValue,
    },
    main_base_abort::MainBaseAbortActorLease,
    native_ground_actor as shared,
    resource_cache::ResourceCache,
    sub_h_external_frame::SubHRuntimeState,
    world_fx::WorldFx,
};
use v2k_formats::{
    anim_frames::TerrainObjectTable,
    collision::{
        BehaviorChoice, CommonAxisDescriptor, ProjectileEmitterDescriptor,
        SubAPropulsionDescriptor, SubBLateralDescriptor, SubCLiftDescriptor,
    },
    terrain::TerrainGrid,
};

/// A/B/C/D/E/H: Type122's topology without J.
pub(crate) const TOPOLOGY: CommonMoverComponentTopology = CommonMoverComponentTopology {
    sub_j: false,
    ..crate::native_type122::TOPOLOGY
};
const SUB_H_RECORDS: usize = 6;
/// Both rows pair their legs the same way.
const SUB_H_DEPENDENCIES: [[u8; 4]; 6] = [
    [1, 2, 3, 4],
    [0, 2, 3, 5],
    [0, 1, 4, 5],
    [0, 1, 4, 5],
    [0, 2, 3, 5],
    [1, 2, 3, 4],
];
const TYPE76_CHOICES: [BehaviorChoice; 3] = [
    BehaviorChoice {
        weight_rule_id: 1,
        weight_multiplier: 1,
        behavior_class_id: 33,
    },
    BehaviorChoice {
        weight_rule_id: 8,
        weight_multiplier: 5,
        behavior_class_id: 26,
    },
    BehaviorChoice {
        weight_rule_id: 2,
        weight_multiplier: 10,
        behavior_class_id: 7,
    },
];
const TYPE77_CHOICES: [BehaviorChoice; 5] = [
    BehaviorChoice {
        weight_rule_id: 1,
        weight_multiplier: 10,
        behavior_class_id: 33,
    },
    BehaviorChoice {
        weight_rule_id: 8,
        weight_multiplier: 5,
        behavior_class_id: 26,
    },
    BehaviorChoice {
        weight_rule_id: 2,
        weight_multiplier: 20,
        behavior_class_id: 7,
    },
    BehaviorChoice {
        weight_rule_id: 1,
        weight_multiplier: 1,
        behavior_class_id: 4,
    },
    BehaviorChoice {
        weight_rule_id: 1,
        weight_multiplier: 2,
        behavior_class_id: 5,
    },
];

/// Section-12 rows sharing this owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type76Row {
    /// Worlds 26/31: method24, `+C0` 0x39.
    Type76,
    /// Worlds 26/37/39/40: method30, `+C0` 0x439, Defecate and Aimless too.
    Type77,
}

impl Type76Row {
    pub const fn from_entity_type(entity_type: u32) -> Option<Self> {
        match entity_type {
            76 => Some(Self::Type76),
            77 => Some(Self::Type77),
            _ => None,
        }
    }
    pub const fn entity_type(self) -> u32 {
        match self {
            Self::Type76 => 76,
            Self::Type77 => 77,
        }
    }
    pub const fn model(self) -> usize {
        match self {
            Self::Type76 => 269,
            Self::Type77 => 271,
        }
    }
    pub const fn health(self) -> i32 {
        match self {
            Self::Type76 => 7000,
            Self::Type77 => 10_000,
        }
    }
    /// `+C0`: Type76 has no generic crush (0x400); both drag.
    pub const fn default_flags(self) -> u32 {
        match self {
            Self::Type76 => 0x39,
            Self::Type77 => 0x439,
        }
    }
    /// Candidate masks: Type76 7 (player, 0x02, 0x04); Type77 0x105.
    pub const fn axis(self) -> CommonAxisDescriptor {
        CommonAxisDescriptor {
            strict_axis_limit_raw: 3584,
            raw_word_at_0x04: match self {
                Self::Type76 => 7,
                Self::Type77 => 0x105,
            },
        }
    }
    pub const fn choices(self) -> &'static [BehaviorChoice] {
        match self {
            Self::Type76 => &TYPE76_CHOICES,
            Self::Type77 => &TYPE77_CHOICES,
        }
    }
    pub const fn living_classes(self) -> &'static [u8] {
        match self {
            Self::Type76 => &[7, 26, 33],
            Self::Type77 => &[4, 5, 7, 26, 33],
        }
    }
    pub const fn emitter(self) -> ProjectileEmitterDescriptor {
        let (method, interval, tolerance, raw) = match self {
            Self::Type76 => (24, 150_000, 5120, 114),
            Self::Type77 => (30, 300_000, 2560, 168),
        };
        ProjectileEmitterDescriptor {
            projectile_method: method,
            random_interval_us: interval,
            spread_raw: 256,
            aim_threshold_raw: 8000,
            speed_override_raw: 0,
            target_axis_tolerance_raw: tolerance,
            sound_id: 70,
            raw_word_at_0x12: raw,
            alternate_emitter_raw: 0,
            stochastic_gate_mode: 0,
            auxiliary_command: 0,
            variable_bindings: [0; 4],
        }
    }
    const fn sub_h_phase_rate(self) -> i32 {
        match self {
            Self::Type76 => 0x3000_0000,
            Self::Type77 => 0x2000_0000,
        }
    }
    const fn sub_h_refs(self) -> [[u16; 3]; 6] {
        match self {
            Self::Type76 => [
                [52, 60, 66],
                [53, 61, 67],
                [50, 58, 64],
                [51, 59, 65],
                [48, 56, 62],
                [49, 57, 63],
            ],
            Self::Type77 => [
                [114, 58, 64],
                [115, 59, 65],
                [112, 56, 62],
                [113, 57, 63],
                [66, 120, 118],
                [67, 121, 119],
            ],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type76Runtime {
    pub(crate) row: Type76Row,
    entity_id: u32,
    pub(crate) spawn_index: usize,
    anchor_raw: [i16; 3],
    allocation: MainBaseAbortActorLease,
    pub(crate) sub_d_runtime: Type9SubDRuntime,
    pub(crate) sub_d_owner: Type9SubDFrameOwner,
    pub(crate) sub_e_runtime: crate::generic_projectile_emitter::GenericEmitterRuntime,
}

pub type Type76Owner = shared::NativeGroundActorOwner<profile::Type76Profile>;
pub type Type77Owner = shared::NativeGroundActorOwner<profile::Type77Profile>;
pub type Type76Outcome = shared::NativeGroundActorOutcome;
pub use shared::NativeGroundActorBlock as Type76Block;

/// One scheduler entry for both rows; each keeps its own framework profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type76FamilyOwner {
    Type76(Type76Owner),
    Type77(Type77Owner),
}

impl Type76FamilyOwner {
    pub fn adopt(manager: &EntityManager, id: u32) -> Result<Self, Type76Block> {
        match manager
            .iter_all()
            .find(|entity| entity.id == id)
            .and_then(type76_row)
        {
            Some(Type76Row::Type76) => Type76Owner::adopt(manager, id).map(Self::Type76),
            Some(Type76Row::Type77) => Type77Owner::adopt(manager, id).map(Self::Type77),
            None => Err(Type76Block::Allocation),
        }
    }
    pub(crate) fn adopt_blocked_prefix(
        manager: &EntityManager,
        id: u32,
    ) -> Result<Self, Type76Block> {
        match manager
            .iter_all()
            .find(|entity| entity.id == id)
            .and_then(type76_row)
        {
            Some(Type76Row::Type76) => {
                Type76Owner::adopt_blocked_prefix(manager, id).map(Self::Type76)
            }
            Some(Type76Row::Type77) => {
                Type77Owner::adopt_blocked_prefix(manager, id).map(Self::Type77)
            }
            None => Err(Type76Block::Allocation),
        }
    }
    pub const fn entity_id(self) -> u32 {
        match self {
            Self::Type76(owner) => owner.entity_id(),
            Self::Type77(owner) => owner.entity_id(),
        }
    }
    pub(crate) const fn actor_lease(self) -> MainBaseAbortActorLease {
        match self {
            Self::Type76(owner) => owner.actor_lease(),
            Self::Type77(owner) => owner.actor_lease(),
        }
    }
    pub(crate) const fn has_pending_prefix(self) -> bool {
        match self {
            Self::Type76(owner) => owner.has_pending_prefix(),
            Self::Type77(owner) => owner.has_pending_prefix(),
        }
    }
    pub(crate) fn park_external_prefix(&mut self) {
        match self {
            Self::Type76(owner) => owner.park_external_prefix(),
            Self::Type77(owner) => owner.park_external_prefix(),
        }
    }
    pub(crate) const fn fork_for_main_base_abort_transaction(self) -> Self {
        self
    }
    pub(crate) fn completed_mutation_boundary(self, manager: &EntityManager) -> bool {
        match self {
            Self::Type76(owner) => owner.completed_mutation_boundary(manager),
            Self::Type77(owner) => owner.completed_mutation_boundary(manager),
        }
    }
}

pub struct Type76FamilyTick {
    pub outcome: Type76Outcome,
    pub retained_owner: Option<Type76FamilyOwner>,
    pub replacement_terminal: Option<shared::NativeGroundTerminalPublication>,
}

pub struct Type76Frame<'a> {
    pub resources: &'a mut ResourceCache,
    pub world_fx: &'a mut WorldFx,
    pub elapsed_micros: u32,
    pub retail_tick: u32,
}

/// Type77's class4 terrain task writes infection, so both rows lend the
/// mutable world; neither captures.
pub fn tick_type76_family(
    manager: &mut EntityManager,
    owner: Type76FamilyOwner,
    frame: Type76Frame<'_>,
) -> Type76FamilyTick {
    let shared_frame = shared::NativeGroundActorFrame {
        resources: shared::NativeGroundResources::Mutable(frame.resources),
        world_fx: frame.world_fx,
        elapsed_micros: frame.elapsed_micros,
        retail_tick: frame.retail_tick,
        capture: shared::NativeCaptureDispatch::NoCapture,
    };
    match owner {
        Type76FamilyOwner::Type76(owner) => {
            let tick = shared::tick_native_ground_actor(manager, owner, shared_frame);
            Type76FamilyTick {
                outcome: tick.outcome,
                retained_owner: tick.retained_owner.map(Type76FamilyOwner::Type76),
                replacement_terminal: tick.replacement_terminal,
            }
        }
        Type76FamilyOwner::Type77(owner) => {
            let tick = shared::tick_native_ground_actor(manager, owner, shared_frame);
            Type76FamilyTick {
                outcome: tick.outcome,
                retained_owner: tick.retained_owner.map(Type76FamilyOwner::Type77),
                replacement_terminal: tick.replacement_terminal,
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type76Publication {
    pub selection: BehaviorSelection,
    pub selector_word: u32,
    pub under_attack: bool,
    pub furniture_nearby: bool,
    pub initializer_fallback: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type76Error {
    Identity,
    Metadata,
    AlreadyPublished,
    ComponentStorage,
    Prefix,
    Selection,
    Runtime(&'static str),
}

pub(crate) fn allocation_authenticates(entity: &Entity) -> bool {
    entity.native_type76_runtime.is_some_and(|runtime| {
        entity.active
            && entity.id == runtime.entity_id
            && entity.entity_type == runtime.row.entity_type()
            && entity.authored_spawn_index == Some(runtime.spawn_index)
            && entity.model_slots == [Some(runtime.row.model()); 4]
            && entity.capability_flags == 8
            && runtime.allocation.entity_id == entity.id
    })
}

/// A native receipt belongs to its issuing manager generation.
pub(crate) fn manager_allocation_authenticates(manager: &EntityManager, id: u32) -> bool {
    let Some(entity) = manager.iter_all().find(|entity| entity.id == id) else {
        return false;
    };
    allocation_authenticates(entity)
        && manager
            .main_base_abort_actor_observation(id)
            .is_some_and(|observation| {
                observation.lease == entity.native_type76_runtime.unwrap().allocation
            })
}

/// The authenticated row of a Type76-family allocation.
pub(crate) fn type76_row(entity: &Entity) -> Option<Type76Row> {
    allocation_authenticates(entity)
        .then(|| entity.native_type76_runtime.map(|runtime| runtime.row))
        .flatten()
}

pub(crate) fn authenticate_metadata(
    row: Type76Row,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), Type76Error> {
    use crate::actor_detailed_sound::ActorDetailedSoundPolicy;
    use crate::damage::DamageProfile;
    use crate::entity_collision_state::{CommonMoverGklPayloads, CommonWorldEffectProfile};
    use RetailRuntimeValue::Known;
    let Some(initializer) = &metadata.initializer else {
        return Err(Type76Error::Metadata);
    };
    let Known(Some(h)) = &metadata.sub_h_external_frame_descriptor else {
        return Err(Type76Error::Metadata);
    };
    let (damage, healthy_sound, period, lifetime, target_speed) = match row {
        Type76Row::Type76 => (
            DamageProfile {
                thresholds_raw: [0, 4000, 5000, 0, 200, 0, 0],
                multipliers_q8: [0, 256, 256, 1024, 0, 0, 0],
            },
            0,
            0,
            0,
            450,
        ),
        Type76Row::Type77 => (
            DamageProfile {
                thresholds_raw: [0, 4000, 4000, 0, 200, 0, 0],
                multipliers_q8: [0, 256, 256, 128, 128, 0, 0],
            },
            80,
            2_000_000,
            67,
            300,
        ),
    };
    let refs = row.sub_h_refs();
    if metadata.model_slots != [row.model() as u16; 4]
        || metadata.mass_raw != 100
        || metadata.capability_flags != 8
        || metadata.initial_health_raw != Some(row.health())
        || metadata.damage_profile != Some(damage)
        || metadata.common_mover_topology != Known(TOPOLOGY)
        || metadata.common_mover_gkl_payloads
            != Known(CommonMoverGklPayloads {
                sub_g: None,
                sub_k: None,
                sub_l: None,
            })
        || metadata.sub_f_swimming_descriptor != Known(None)
        || metadata.sub_j_attachment_descriptor != Known(None)
        || metadata.projectile_emitter_descriptor != Known(Some(row.emitter()))
        || metadata.actor_animation_descriptor != Known(None)
        || metadata.status_component_descriptor != Known(None)
        || metadata.model_variable_count_raw != Known(0)
        || metadata.sub_n_payload.is_some()
        || metadata.constructor_sound_attachment_id != Known(None)
        || metadata.accepted_hit_presentation_sound_id != Known(Some(84))
        || metadata.infected_model_presentation_sound_id != Known(None)
        || metadata.generic_hit_sound_id != Known(None)
        || metadata.death_sound_id != Known(Some(75))
        || metadata.target_warning_sound_id != Known(None)
        || metadata.search_attack_optional_prelude_sound_id != Known(None)
        || metadata.search_attack_aim_sound_id != Known(None)
        || metadata.search_attack_aim_sound_period_raw != Known(0)
        || metadata.run_away_optional_sound_id != Known(None)
        || metadata.run_away_sound_period_raw != Known(0)
        || metadata.terrain_contact_task_lifetime_ms != Known(lifetime)
        || metadata.detailed_sound_policy
            != Known(ActorDetailedSoundPolicy {
                full_health_raw: row.health(),
                healthy_sounds: [healthy_sound, 0],
                low_health_sound: 0,
                periods_raw: [period, 0],
            })
        || metadata.common_world_effects
            != Known(CommonWorldEffectProfile {
                surface_selectors: [1, 0],
                surface_lifetime_ms: 2000,
                low_health_effect_words: [healthy_sound, 0, 0],
            })
        || metadata.sub_a_propulsion_descriptor
            != Known(Some(SubAPropulsionDescriptor {
                acceleration_raw: 1500,
                overspeed_correction_raw: -3000,
                target_speed_base_raw: target_speed,
            }))
        || metadata.sub_b_lateral_descriptor
            != Known(Some(SubBLateralDescriptor {
                projection_threshold_rate_raw: 10_000,
                correction_rate_raw: 1000,
            }))
        || metadata.sub_c_lift_descriptor
            != Known(Some(SubCLiftDescriptor {
                base_clearance_raw: 50,
                lift_range_raw: 75,
                strength_raw: 0x30_0000,
                near_boost_range_raw: 100,
                damping_range_raw: 200,
                surface_mode_raw: 0,
                offset_sample_raw: 0,
                reserved_at_0x0e: [0; 2],
            }))
        || metadata.sub_d_steering_descriptor != Known(Some(INTRO2_TYPE58_SUB_D))
        || initializer.initializer_state_flags_raw != row.default_flags()
        || initializer.common_axis_descriptor != row.axis()
        || initializer.behavior_choices.as_ref() != row.choices()
        || initializer.behavior_rule_ref != 1
        || initializer.alternate_behavior_class_ref != 12
        || h.completion_sound_id.is_some()
        || h.records.len() != SUB_H_RECORDS
        || h.records.iter().enumerate().any(|(index, record)| {
            record.resolver_flags_raw != 0x2000_0000
                || record.phase_rate_raw != row.sub_h_phase_rate()
                || record.vertex_refs != refs[index]
                || record.axis_mode_raw != 0
                || record.dependencies != SUB_H_DEPENDENCIES[index]
        })
    {
        return Err(Type76Error::Metadata);
    }
    Ok(())
}

/// 24E30 clears the emitter then copies method and sound; bindings are absent.
fn emitter_constructor(row: Type76Row) -> crate::generic_projectile_emitter::GenericEmitterRuntime {
    crate::generic_projectile_emitter::GenericEmitterRuntime {
        joint_bindings: [None; 2],
        projectile_method: row.emitter().projectile_method,
        emitter_selector: 0,
        sound_id: u32::from(row.emitter().sound_id),
        direct_mode: 0,
        remaining_time_raw: 0,
        manual_step_raw: 0,
        remaining_bursts_raw: 0,
        cadence_raw: 0,
        basis_adjustment_identity: None,
    }
}

/// The authored terrain, as resident when 104B0 runs.
pub(crate) struct Type76BirthWorld<'a> {
    pub terrain: &'a TerrainGrid,
    pub objects: Option<&'a TerrainObjectTable>,
    pub retail_tick: u32,
}

/// Execute the source-ordered component, selector and task suffixes.
fn publish_birth(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    receipt: Type76Runtime,
    world: Type76BirthWorld<'_>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Type76Publication, Type76Error> {
    let row = receipt.row;
    let sub_h = SubHRuntimeState::new(SUB_H_RECORDS).map_err(|_| Type76Error::ComponentStorage)?;
    let RetailRuntimeValue::Known(Some(sub_a_descriptor)) = metadata.sub_a_propulsion_descriptor
    else {
        unreachable!("authenticated Type76-family Sub-A");
    };
    // 09A80 constructs H and D before A. 20450 consumes its own word even
    // though each later 06070 suffix overwrites the randomized target speed.
    entity.sub_h_external_frame_runtime = RetailRuntimeValue::Known(Some(sub_h));
    entity.native_type76_runtime = Some(receipt);
    entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(
        crate::common_mover::SubAPropulsionRuntime::from_20450_constructor(
            sub_a_descriptor,
            next_random() as u16,
        ),
    ));
    // D4A0's terrain snap and immutable +90 copy precede every 425680 rule.
    entity.set_position_raw(receipt.anchor_raw);
    // Fresh +34=0 makes 16490 false for every current tick: tick<250 and
    // signed tick>249 cannot both hold. Keep the actual clock at the reader.
    let under_attack = world.retail_tick < 250 && (world.retail_tick as i32) > 249;
    // Rule8 scans objects within a quarter of the authored strict axis.
    let furniture_nearby = crate::trash_furniture::find_furniture(
        world.terrain,
        world.objects,
        entity.position_raw(),
        row.axis().strict_axis_limit_raw / 4,
        -1,
    )
    .is_some();
    let mut selector_word = 0;
    let selection = select_initial_behavior(
        row.choices(),
        |rule| match rule {
            BehaviorWeightRule::Always => 1,
            BehaviorWeightRule::UnderAttack => i32::from(under_attack),
            BehaviorWeightRule::FurnitureNearby => i32::from(furniture_nearby),
            _ => unreachable!("authenticated Type76-family choices"),
        },
        || {
            selector_word = next_random();
            selector_word
        },
    )
    .map_err(|_| Type76Error::Selection)?
    .ok_or(Type76Error::Selection)?;
    let context = BehaviorContextRuntime::from_fresh_weighted_selection(selection)
        .ok_or(Type76Error::Selection)?;

    // Explicit native policy for the allocator's unwritten transient +B2.
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
    entity.initial_behavior = RetailRuntimeValue::Known(Some(selection));
    let succeeded = publish_acquiring(entity, metadata, selection, context, next_random);
    // The successful 104B0 wrapper constructs the body matrix only after the
    // initializer, preserving every authored Euler word.
    let [heading, pitch, roll] = entity.rotation_heading_pitch_roll_raw();
    entity.physical_body_basis_q31 = RetailRuntimeValue::Known(
        crate::common_mover::type9_attitude::Type9BodyBasis::from_angle_words(heading, pitch, roll),
    );
    entity.collision.state_flags_at_0x08.overwrite(
        crate::entity_collision_state::BODY_BASIS_REBUILT_STATE_BIT,
        crate::entity_collision_state::BODY_BASIS_REBUILT_STATE_BIT,
    );
    Ok(Type76Publication {
        selection,
        selector_word,
        under_attack,
        furniture_nearby,
        initializer_fallback: !succeeded,
    })
}

/// AC60's selected initializer, on the ground host's shared programs.
pub(crate) fn publish_acquiring(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    selection: BehaviorSelection,
    context: BehaviorContextRuntime,
    next_random: &mut impl FnMut() -> u32,
) -> bool {
    let Some(row) = entity.native_type76_runtime.map(|runtime| runtime.row) else {
        return false;
    };
    row.living_classes().contains(&selection.program.class_id)
        && shared::publication::publish_shared_root_class(
            entity,
            metadata,
            selection,
            context,
            row.axis(),
            next_random,
        )
}
