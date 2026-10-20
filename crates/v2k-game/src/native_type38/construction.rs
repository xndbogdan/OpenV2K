//! Shared 104B0/09A80/D4A0 entry with actual process Sub-D custody, for
//! authored spawns and zeroed (Hive) creation records.

use super::tasks;
use super::*;
use crate::sub_h_external_frame::SubHRuntimeState;
use crate::{
    actor_task_owner::ActorTaskSlot,
    common_mover::{sub_d::NativeSubDConstruction, SubAPropulsionRuntime},
    entity::{EntityConstructionResources, NativeCreationRecord},
    entity_behavior::translate_state_policy,
    entity_collision_state::{
        RetailStateWord, ACTIVE_MODEL_SLOT_HIGH_STATE_BIT, SURFACE_STATE_MASK,
    },
};

pub(crate) struct Type38AuthoredConstructionRequest<'a> {
    pub entity: &'a mut Entity,
    pub allocation: MainBaseAbortActorLease,
    pub metadata: &'a EntityTypeRuntimeMetadata,
    pub record: NativeCreationRecord,
    pub preceding: &'a [Entity],
    pub resources: EntityConstructionResources<'a>,
    pub constructor_surface_bits: u32,
    pub sub_d: NativeSubDConstruction,
}

pub(crate) fn publish_authored_type38(
    request: Type38AuthoredConstructionRequest<'_>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Type38Publication, Type38Block> {
    let Type38AuthoredConstructionRequest {
        entity,
        allocation,
        metadata,
        record,
        preceding,
        resources,
        constructor_surface_bits,
        sub_d,
    } = request;
    let row = Type38Row::from_entity_type(record.entity_type).ok_or(Type38Block::Identity)?;
    authenticate_metadata(row, metadata)?;
    let slots = std::array::from_fn(|slot| {
        let model = if record.model_overrides[slot] == 0 {
            u32::from(metadata.model_slots[slot])
        } else {
            record.model_overrides[slot]
        };
        (model != 0).then_some(model as usize)
    });
    if !entity.active
        || entity.entity_type != row.entity_type()
        || entity.id != allocation.entity_id
        || entity.authored_spawn_index != record.authored_spawn_index
        || entity.capability_flags != metadata.capability_flags
        || entity.model_slots != slots
        || slots != [Some(row.model()); 4]
        || entity.rotation_heading_pitch_roll_raw() != record.rotation.map(|word| word as i16)
        || !record.plain
    {
        return Err(Type38Block::Identity);
    }
    if entity.native_type38_runtime.is_some()
        || entity.initial_behavior != RetailRuntimeValue::Unresolved
        || entity.current_behavior_context != RetailRuntimeValue::Unresolved
        || ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .iter()
            .any(|&slot| entity.actor_task_state(slot).is_some())
    {
        return Err(Type38Block::AlreadyPublished);
    }
    // A persistent arriving player has no authored index and participates in
    // PlayerNearby before the current world's first authored allocation.
    if preceding.iter().any(|candidate| {
        candidate.id == entity.id
            || candidate
                .authored_spawn_index
                .zip(record.authored_spawn_index)
                .is_some_and(|(index, own)| index >= own)
    }) || preceding.windows(2).any(|pair| {
        matches!((pair[0].authored_spawn_index, pair[1].authored_spawn_index),
            (Some(a), Some(b)) if a >= b)
    }) {
        return Err(Type38Block::Prefix);
    }
    let RetailRuntimeValue::Known(Some(a)) = metadata.sub_a_propulsion_descriptor else {
        unreachable!()
    };
    if entity.actor_common_axis_descriptor != RetailRuntimeValue::Known(row.axis())
        || entity.sub_a_propulsion_runtime
            != RetailRuntimeValue::Known(Some(SubAPropulsionRuntime::pending_constructor_rng(a)))
        || !tasks::initializer_storage_authenticates(entity)
        || entity.sub_j_attachment_runtime != RetailRuntimeValue::Known(None)
        || entity.collision.health_raw != RetailRuntimeValue::Known(row.health())
        || entity.collision.pre_health_damage_buffer_raw
            != RetailRuntimeValue::Known(record.initial_damage_buffer_raw)
        || entity.collision.last_hit_presentation_tick_at_0x34 != RetailRuntimeValue::Known(0)
        || entity.collision.recent_relation_id_at_0x60 != RetailRuntimeValue::Known(None)
        || metadata.sub_d_steering_descriptor != RetailRuntimeValue::Known(Some(sub_d.descriptor))
        || sub_d.runtime != Type9SubDRuntime::from_constructor()
        || sub_d.frame_owner.classifier_cache().stagger_counter() != sub_d.seed
    {
        return Err(Type38Block::ComponentStorage);
    }
    if constructor_surface_bits & !SURFACE_STATE_MASK != 0 {
        return Err(Type38Block::Runtime("constructor surface bits"));
    }
    let terrain = resources
        .terrain
        .ok_or(Type38Block::Runtime("constructor terrain"))?;
    let [x, _, z] = record.position_raw;
    let cell = terrain
        .cell(usize::from((x as u16) >> 8), usize::from((z as u16) >> 8))
        .ok_or(Type38Block::Runtime("constructor terrain cell"))?;
    let RetailRuntimeValue::Known(anchor_raw) = crate::entity_initializer::constructor_position_raw(
        metadata,
        record.position_raw,
        Some(terrain),
        None,
    ) else {
        return Err(Type38Block::Metadata);
    };
    let mut state =
        RetailStateWord::exact(0x0607_8801 | if record.param != 0 { 0x0100_0000 } else { 0 });
    state.overwrite(SURFACE_STATE_MASK, constructor_surface_bits);
    state.overwrite(
        ACTIVE_MODEL_SLOT_HIGH_STATE_BIT,
        if cell.terrain_type & 0x10 != 0 {
            ACTIVE_MODEL_SLOT_HIGH_STATE_BIT
        } else {
            0
        },
    );
    let policy = translate_state_policy(row.default_flags());
    state.overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);
    entity.collision.state_flags_at_0x08 = state;
    entity.model_index = Some(row.model());
    // Common 104B0 +44 and the type-vtable +30 are both null.
    entity.collision.pair_callbacks.damage_modifier_address = RetailRuntimeValue::Known(None);
    entity
        .collision
        .pair_callbacks
        .damage_modifier_identity_context_empty = RetailRuntimeValue::Known(true);
    entity.collision.pair_callbacks.type_hit_callback_address = RetailRuntimeValue::Known(None);
    let runtime = Type38Runtime {
        row,
        allocation,
        spawn_index: record.authored_spawn_index,
        anchor_raw,
        sub_d_runtime: sub_d.runtime,
        sub_d_owner: sub_d.frame_owner,
        sub_e_runtime: tasks::emitter_constructor(row),
        kl_components: Intro2KlComponents::from_native_constructor(metadata)
            .map_err(Type38Block::Kl)?,
    };
    publish_birth(entity, metadata, preceding, runtime, next_random)
}

/// 09A80 builds H/D/K/L/A/E, then D4A0/AC60 publishes the weighted task.
/// A20450, selector25680 and each selected 06070 suffix consume separate words.
fn publish_birth(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    preceding: &[Entity],
    runtime: Type38Runtime,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Type38Publication, Type38Block> {
    let RetailRuntimeValue::Known(Some(a)) = metadata.sub_a_propulsion_descriptor else {
        unreachable!()
    };
    let row = runtime.row;
    let h =
        SubHRuntimeState::new(row.sub_h_records()).map_err(|_| Type38Block::ComponentStorage)?;
    let a = SubAPropulsionRuntime::from_20450_constructor(a, next_random() as u16);
    entity.set_position_raw(runtime.anchor_raw);
    entity.sub_h_external_frame_runtime = RetailRuntimeValue::Known(Some(h));
    entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(a));
    entity.native_type38_runtime = Some(runtime);
    // Labeled native allocation policy, shared with the actual authored
    // constructor: transient B2 is unwritten allocator residue. Seed it once;
    // later callbacks and animation contribute through their typed writer.
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
    let mut owner = shared::candidate(entity);
    owner.attached_entity_handle = entity.collision.recent_relation_id_at_0x60;
    let candidates: Vec<_> = preceding
        .iter()
        .filter(|entity| entity.active)
        .map(shared::candidate)
        .collect();
    // Only rule 6 reads candidate state: PlayerNearby within the strict axis.
    // Type119 weighs both classes Always and never reads it.
    let player_nearby = match row {
        Type38Row::Type38 => shared::nearby::<profile::Type38Profile>(owner, &candidates, 1),
        Type38Row::Type129 => shared::nearby::<profile::Type129Profile>(owner, &candidates, 1),
        Type38Row::Type119 => Ok(false),
    }
    .map_err(Type38Block::Ground)?;
    let mut selector_word = 0;
    let selection = tasks::select(row, player_nearby, &mut || {
        selector_word = next_random();
        selector_word
    })?;
    let context = BehaviorContextRuntime::from_fresh_weighted_selection(selection)
        .ok_or(Type38Block::Runtime("weighted selection context"))?;
    entity.initial_behavior = RetailRuntimeValue::Known(Some(selection));
    let succeeded = tasks::publish_acquiring(entity, metadata, selection, context, next_random);
    Ok(Type38Publication {
        selection,
        selector_word,
        player_nearby,
        initializer_fallback: !succeeded,
    })
}
