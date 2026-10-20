//! Native aquatic actors: shared B/D/F construction and authored task ownership.
//!
//! Types22/23/24/62/124 differ through Section12 descriptors and weighted choices.
//! Their native allocation retains process Sub-D history and Sub-F RNG before
//! selection. Authored coordinates and model selectors remain instance data.
//! Type25 sharks share the B/D/F allocation without the 2000 environment
//! bit, so their coarse visits run tasks; they add class69 Shark Attack.

#[cfg(test)]
mod auto_pilot_tests;
pub(crate) mod death;
mod dying;
pub mod impact;
mod live;
mod mover;
pub(crate) mod shark_contact;
#[cfg(test)]
mod shark_tests;
mod surface;
#[cfg(test)]
mod tests;
pub use live::{
    tick_shared_fish, SharedFishBlock, SharedFishFrame, SharedFishOutcome, SharedFishOwner,
    SharedFishTick,
};

use crate::{
    actor_task_owner::ActorTaskSlot,
    common_mover::{
        sub_d::{NativeSubDConstruction, Type9SubDFrameOwner, Type9SubDRuntime},
        sub_f::SubFSwimmingRuntime,
        type9_attitude::Type9BodyBasis,
    },
    entity::{Entity, EntityConstructionResources, EntityManager},
    entity_behavior::{translate_state_policy, BehaviorContextRuntime},
    entity_collision_state::{
        active_model_slot_from_state_flags, CommonMoverComponentTopology,
        EntityTypeRuntimeMetadata, RetailRuntimeValue, RetailStateWord,
        ACTIVE_MODEL_SLOT_HIGH_STATE_BIT, BODY_BASIS_REBUILT_STATE_BIT, SURFACE_STATE_MASK,
    },
    main_base_abort::MainBaseAbortActorLease,
};

pub(crate) const TOPOLOGY: CommonMoverComponentTopology = CommonMoverComponentTopology {
    sub_b: true,
    sub_d: true,
    sub_f: true,
    sub_a: false,
    sub_c: false,
    sub_e: false,
    sub_g: false,
    sub_h: false,
    sub_i: false,
    sub_j: false,
    sub_k: false,
    sub_l: false,
    sub_m: false,
    sub_n: false,
    sub_o: false,
};

/// B/D/F swimmers whose authored task graphs are owned here: the neutral
/// fish and the Type25 shark. Other hostile swimmers are not admitted.
pub(crate) const fn is_shared_fish_type(entity_type: u32) -> bool {
    matches!(entity_type, 22 | 23 | 24 | 62 | 124 | 25)
}

/// Type25: C8 `0xC`, Wander x1, PlayerNearby x40 Shark Attack, Retarget x6,
/// alternate class12.
pub(crate) const fn is_shark(entity_type: u32) -> bool {
    entity_type == 25
}

const SHARK_CHOICES: [(u32, u32, u32); 3] = [(1, 1, 6), (6, 40, 69), (1, 6, 5)];

fn is_shark_row(init: &crate::entity_collision_state::EntityInitializerSpec) -> bool {
    init.initializer_state_flags_raw == 0xc
        && init.behavior_rule_ref == 1
        && init.alternate_behavior_class_ref == 12
        && init
            .behavior_choices
            .iter()
            .map(|c| (c.weight_rule_id, c.weight_multiplier, c.behavior_class_id))
            .eq(SHARK_CHOICES)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SharedFishRuntime {
    pub(crate) allocation: MainBaseAbortActorLease,
    pub(crate) entity_type: u32,
    /// None for a Hive birth's zeroed record.
    pub(crate) spawn_index: Option<usize>,
    pub(crate) model_slots: [Option<usize>; 4],
    pub(crate) immutable_anchor_raw: [i16; 3],
    pub(crate) sub_d_runtime: Type9SubDRuntime,
    pub(crate) sub_d_owner: Type9SubDFrameOwner,
    pub(crate) sub_f: SubFSwimmingRuntime,
    /// Indexed by the one-based callback selector, matching AnimVars::dynamic.
    /// Index0 is unused; the source pointer is bank+(selector-1)*2.
    pub(crate) variables: [u16; 64],
    /// Published only after the complete native class2 callback has staged
    /// this allocation. Retains repeated-hit custody until the later sweep.
    pub(crate) quiet_death_context: Option<BehaviorContextRuntime>,
    /// An external hit committed before reaching an unsupported suffix.
    /// This remains allocation-owned after class2 retires the task owner.
    pub(crate) impact_prefix_pending: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SharedFishError {
    Allocation,
    Metadata,
    AlreadyPublished,
    Graph,
    Runtime(&'static str),
}

pub(crate) fn authenticate_metadata(
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), SharedFishError> {
    let Some(init) = &metadata.initializer else {
        return Err(SharedFishError::Metadata);
    };
    let RetailRuntimeValue::Known(Some(f)) = metadata.sub_f_swimming_descriptor else {
        return Err(SharedFishError::Metadata);
    };
    let RetailRuntimeValue::Known(variable_count) = metadata.model_variable_count_raw else {
        return Err(SharedFishError::Metadata);
    };
    if metadata.common_mover_topology != RetailRuntimeValue::Known(TOPOLOGY)
        || !matches!(
            metadata.sub_b_lateral_descriptor,
            RetailRuntimeValue::Known(Some(_))
        )
        || !matches!(
            metadata.sub_d_steering_descriptor,
            RetailRuntimeValue::Known(Some(_))
        )
        || !(is_shark_row(init)
            || matches!(init.initializer_state_flags_raw, 0x2004 | 0x200c)
                && !init.behavior_choices.is_empty()
                && init
                    .behavior_choices
                    .iter()
                    .all(|c| c.weight_rule_id == 1 && matches!(c.behavior_class_id, 5 | 6 | 13)))
        || variable_count >= 64
        || f.variable_selectors
            .iter()
            .any(|&s| s < 0 || s as u32 > variable_count)
        || f.animation_mode_raw > 2
    {
        return Err(SharedFishError::Metadata);
    }
    Ok(())
}

/// The owner a shark's class12 publication leaves to tick.
pub(crate) fn dying_shark_owner(manager: &EntityManager, id: u32) -> Option<SharedFishOwner> {
    manager
        .iter_all()
        .find(|entity| entity.id == id)
        .is_some_and(dying::dying_graph)
        .then(|| SharedFishOwner::adopt(manager, id).ok())
        .flatten()
}

pub(crate) fn allocation_authenticates(manager: &EntityManager, id: u32) -> bool {
    let Some(entity) = manager.iter_all().find(|e| e.id == id) else {
        return false;
    };
    let Some(runtime) = &entity.shared_fish_runtime else {
        return false;
    };
    entity.active
        && is_shared_fish_type(entity.entity_type)
        && entity.entity_type == runtime.entity_type
        && entity.authored_spawn_index == runtime.spawn_index
        && entity.model_slots == runtime.model_slots
        && manager
            .main_base_abort_actor_observation(id)
            .is_some_and(|o| o.lease == runtime.allocation)
}

pub(crate) struct FishAuthoredConstruction<'a> {
    pub entity: &'a mut Entity,
    pub allocation: MainBaseAbortActorLease,
    pub metadata: &'a EntityTypeRuntimeMetadata,
    /// An authored spawn, or a Hive birth's zeroed record.
    pub record: crate::entity::NativeCreationRecord,
    /// Earlier live allocations: PlayerNearby's birth candidates.
    pub preceding: &'a [Entity],
    pub resources: EntityConstructionResources<'a>,
    pub constructor_surface_bits: u32,
    pub sub_d: NativeSubDConstruction,
}

pub(crate) fn publish_authored_fish(
    request: FishAuthoredConstruction<'_>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<(), SharedFishError> {
    let FishAuthoredConstruction {
        entity,
        allocation,
        metadata,
        record: spawn,
        preceding,
        resources,
        constructor_surface_bits,
        sub_d,
    } = request;
    authenticate_metadata(metadata)?;
    let model_slots = std::array::from_fn(|slot| {
        let model = if spawn.model_overrides[slot] == 0 {
            u32::from(metadata.model_slots[slot])
        } else {
            spawn.model_overrides[slot]
        };
        (model != 0).then_some(model as usize)
    });
    if !entity.active
        || !is_shared_fish_type(entity.entity_type)
        || entity.entity_type != spawn.entity_type
        || entity.id != allocation.entity_id
        || entity.authored_spawn_index != spawn.authored_spawn_index
        || entity.model_slots != model_slots
        || entity.capability_flags != metadata.capability_flags
        || entity.rotation_heading_pitch_roll_raw() != spawn.rotation.map(|w| w as i16)
        || !spawn.plain
    {
        return Err(SharedFishError::Allocation);
    }
    if entity.shared_fish_runtime.is_some()
        || entity.current_behavior_context != RetailRuntimeValue::Unresolved
        || ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .any(|slot| entity.actor_task_state(slot).is_some())
    {
        return Err(SharedFishError::AlreadyPublished);
    }
    // The detached initializer can already name a singleton choice (Type62).
    // That is not task publication: AC60 still draws after Sub-F construction.
    if metadata.sub_d_steering_descriptor != RetailRuntimeValue::Known(Some(sub_d.descriptor))
        || constructor_surface_bits & !SURFACE_STATE_MASK != 0
    {
        return Err(SharedFishError::Metadata);
    }
    let terrain = resources
        .terrain
        .ok_or(SharedFishError::Runtime("constructor terrain"))?;
    let [x, _, z] = spawn.position_raw;
    let cell = terrain
        .cell(usize::from(x as u16 >> 8), usize::from(z as u16 >> 8))
        .ok_or(SharedFishError::Runtime("constructor cell"))?;
    let initializer = metadata.initializer.as_ref().unwrap();
    let mut state =
        RetailStateWord::exact(0x0607_8801 | if spawn.param != 0 { 0x0100_0000 } else { 0 });
    state.overwrite(SURFACE_STATE_MASK, constructor_surface_bits);
    state.overwrite(
        ACTIVE_MODEL_SLOT_HIGH_STATE_BIT,
        if cell.terrain_type & 0x10 != 0 {
            ACTIVE_MODEL_SLOT_HIGH_STATE_BIT
        } else {
            0
        },
    );
    let policy = translate_state_policy(initializer.initializer_state_flags_raw);
    state.overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);
    entity.collision.state_flags_at_0x08 = state;
    let RetailRuntimeValue::Known(flags) = state.masked(u32::MAX) else {
        unreachable!()
    };
    entity.model_index = entity.model_slots[active_model_slot_from_state_flags(flags)];
    entity.collision.pair_callbacks.damage_modifier_address = RetailRuntimeValue::Known(None);
    entity
        .collision
        .pair_callbacks
        .damage_modifier_identity_context_empty = RetailRuntimeValue::Known(true);
    entity.collision.pair_callbacks.type_hit_callback_address = RetailRuntimeValue::Known(None);
    // Same explicit native allocator policy as the other shared constructors.
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
    let RetailRuntimeValue::Known(Some(f)) = metadata.sub_f_swimming_descriptor else {
        unreachable!()
    };
    let sub_f =
        SubFSwimmingRuntime::construct(f, next_random, &mut |selector| Some(selector as usize));
    entity.set_position_raw(spawn.position_raw);
    entity.shared_fish_runtime = Some(SharedFishRuntime {
        allocation,
        entity_type: entity.entity_type,
        spawn_index: spawn.authored_spawn_index,
        model_slots: entity.model_slots,
        immutable_anchor_raw: spawn.position_raw,
        sub_d_runtime: sub_d.runtime,
        sub_d_owner: sub_d.frame_owner,
        sub_f,
        variables: [0; 64],
        quiet_death_context: None,
        impact_prefix_pending: false,
    });
    // 425680 reads PlayerNearby over the earlier allocations; no word.
    let rules = crate::shared_fish_tasks::FishRules::evaluate(entity, preceding.iter())
        .map_err(|_| SharedFishError::Graph)?;
    let selection = crate::shared_fish_tasks::select_behavior(metadata, rules, next_random)
        .map_err(|_| SharedFishError::Graph)?;
    let context = BehaviorContextRuntime::from_fresh_weighted_selection(selection)
        .ok_or(SharedFishError::Graph)?;
    crate::shared_fish_tasks::publish_selection(entity, metadata, selection, context, next_random)
        .map_err(|_| SharedFishError::Graph)?;
    entity.initial_behavior = RetailRuntimeValue::Known(Some(selection));
    let [heading, pitch, roll] = entity.rotation_heading_pitch_roll_raw();
    entity.physical_body_basis_q31 =
        RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(heading, pitch, roll));
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(BODY_BASIS_REBUILT_STATE_BIT, BODY_BASIS_REBUILT_STATE_BIT);
    Ok(())
}
