//! Shared 104B0/09A80/D4A0 entry with actual process Sub-D custody, for
//! authored spawns and zeroed (Hive) creation records.

use super::*;
use crate::sub_h_external_frame::SubHRuntimeState;
use crate::{
    actor_task_owner::ActorTaskSlot,
    common_mover::{sub_d::NativeSubDConstruction, SubAPropulsionRuntime},
    entity::{EntityConstructionResources, NativeCreationRecord},
    entity_behavior::{select_initial_behavior, translate_state_policy, BehaviorWeightRule},
    entity_collision_state::{
        RetailStateWord, ACTIVE_MODEL_SLOT_HIGH_STATE_BIT, SURFACE_STATE_MASK,
    },
};

pub(crate) struct NativeType12ConstructionRequest<'a> {
    pub entity: &'a mut Entity,
    pub allocation: MainBaseAbortActorLease,
    pub metadata: &'a EntityTypeRuntimeMetadata,
    pub record: NativeCreationRecord,
    pub preceding: &'a [Entity],
    pub resources: EntityConstructionResources<'a>,
    pub constructor_surface_bits: u32,
    pub sub_d: NativeSubDConstruction,
}

pub(crate) fn publish_native_type12(
    request: NativeType12ConstructionRequest<'_>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Type12Publication, Type12Block> {
    let NativeType12ConstructionRequest {
        entity,
        allocation,
        metadata,
        record,
        preceding,
        resources,
        constructor_surface_bits,
        sub_d,
    } = request;
    let row = Type12Row::from_entity_type(record.entity_type).ok_or(Type12Block::Identity)?;
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
        || slots != [Some(MODEL); 4]
        || entity.rotation_heading_pitch_roll_raw() != record.rotation.map(|word| word as i16)
        || !record.plain
    {
        return Err(Type12Block::Identity);
    }
    if entity.native_type12_runtime.is_some()
        || entity.initial_behavior != RetailRuntimeValue::Unresolved
        || entity.current_behavior_context != RetailRuntimeValue::Unresolved
        || ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .iter()
            .any(|&slot| entity.actor_task_state(slot).is_some())
    {
        return Err(Type12Block::AlreadyPublished);
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
        return Err(Type12Block::Prefix);
    }
    let RetailRuntimeValue::Known(Some(a)) = metadata.sub_a_propulsion_descriptor else {
        unreachable!()
    };
    if entity.actor_common_axis_descriptor != RetailRuntimeValue::Known(row.axis())
        || entity.sub_a_propulsion_runtime
            != RetailRuntimeValue::Known(Some(SubAPropulsionRuntime::pending_constructor_rng(a)))
        || entity.sub_j_attachment_runtime != RetailRuntimeValue::Known(None)
        || entity.collision.health_raw != RetailRuntimeValue::Known(HEALTH)
        || entity.collision.pre_health_damage_buffer_raw
            != RetailRuntimeValue::Known(record.initial_damage_buffer_raw)
        || entity.collision.last_hit_presentation_tick_at_0x34 != RetailRuntimeValue::Known(0)
        || entity.collision.recent_relation_id_at_0x60 != RetailRuntimeValue::Known(None)
        || metadata.sub_d_steering_descriptor != RetailRuntimeValue::Known(Some(sub_d.descriptor))
        || sub_d.runtime != Type9SubDRuntime::from_constructor()
        || sub_d.frame_owner.classifier_cache().stagger_counter() != sub_d.seed
    {
        return Err(Type12Block::ComponentStorage);
    }
    if constructor_surface_bits & !SURFACE_STATE_MASK != 0 {
        return Err(Type12Block::Runtime("constructor surface bits"));
    }
    let terrain = resources
        .terrain
        .ok_or(Type12Block::Runtime("constructor terrain"))?;
    let [x, _, z] = record.position_raw;
    let cell = terrain
        .cell(usize::from((x as u16) >> 8), usize::from((z as u16) >> 8))
        .ok_or(Type12Block::Runtime("constructor terrain cell"))?;
    let RetailRuntimeValue::Known(anchor_raw) = crate::entity_initializer::constructor_position_raw(
        metadata,
        record.position_raw,
        Some(terrain),
        None,
    ) else {
        return Err(Type12Block::Metadata);
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
    let policy = translate_state_policy(DEFAULT_FLAGS);
    state.overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);
    entity.collision.state_flags_at_0x08 = state;
    entity.model_index = Some(MODEL);
    // Common 104B0 +44 and the type-vtable +30 are both null.
    entity.collision.pair_callbacks.damage_modifier_address = RetailRuntimeValue::Known(None);
    entity
        .collision
        .pair_callbacks
        .damage_modifier_identity_context_empty = RetailRuntimeValue::Known(true);
    entity.collision.pair_callbacks.type_hit_callback_address = RetailRuntimeValue::Known(None);
    let runtime = Type12Runtime {
        row,
        allocation,
        spawn_index: record.authored_spawn_index,
        anchor_raw,
        sub_d_runtime: sub_d.runtime,
        sub_d_owner: sub_d.frame_owner,
        sub_e_runtime: emitter_constructor(),
        kl_components: Intro2KlComponents::from_native_constructor(metadata)
            .map_err(Type12Block::Kl)?,
    };
    publish_birth(entity, metadata, preceding, runtime, resources, next_random)
}

/// 09A80 builds H/D/K/L/A/E, then D4A0/AC60 publishes the weighted task.
/// A20450, selector25680 and each selected 06070 suffix consume separate words.
fn publish_birth(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    preceding: &[Entity],
    runtime: Type12Runtime,
    resources: EntityConstructionResources<'_>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Type12Publication, Type12Block> {
    let RetailRuntimeValue::Known(Some(a)) = metadata.sub_a_propulsion_descriptor else {
        unreachable!()
    };
    let row = runtime.row;
    let h = SubHRuntimeState::new(SUB_H_RECORDS).map_err(|_| Type12Block::ComponentStorage)?;
    let a = SubAPropulsionRuntime::from_20450_constructor(a, next_random() as u16);
    entity.set_position_raw(runtime.anchor_raw);
    entity.sub_h_external_frame_runtime = RetailRuntimeValue::Known(Some(h));
    entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(a));
    entity.native_type12_runtime = Some(runtime);
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
    // 425680 evaluates each rule in choice order: PlayerNearby, PeopleNearby
    // (candidate mask 0xC00) and BuildingsNearby; none draws a word.
    let (player_nearby, people_nearby) = match row {
        Type12Row::Type12 => (
            shared::nearby::<profile::Type12Profile>(owner, &candidates, 1),
            shared::nearby::<profile::Type12Profile>(owner, &candidates, 0x0c00),
        ),
        Type12Row::Type127 => (
            shared::nearby::<profile::Type127Profile>(owner, &candidates, 1),
            shared::nearby::<profile::Type127Profile>(owner, &candidates, 0x0c00),
        ),
    };
    let player_nearby = player_nearby.map_err(Type12Block::Ground)?;
    let people_nearby = people_nearby.map_err(Type12Block::Ground)?;
    // Rule9: a kind-3 object within the full strict axis (166B0 -> 230C0).
    let buildings_nearby = crate::trash_furniture::find_furniture(
        resources
            .terrain
            .ok_or(Type12Block::Runtime("buildings selector terrain"))?,
        resources.terrain_objects,
        entity.position_raw(),
        row.axis().strict_axis_limit_raw,
        3,
    )
    .is_some();
    let mut selector_word = 0;
    let selection = select_initial_behavior(
        &CHOICES,
        |rule| match rule {
            BehaviorWeightRule::Always => 1,
            BehaviorWeightRule::PlayerNearby => i32::from(player_nearby),
            BehaviorWeightRule::PeopleNearby => i32::from(people_nearby),
            BehaviorWeightRule::BuildingsNearby => i32::from(buildings_nearby),
            _ => unreachable!("authenticated Type12 rule list"),
        },
        || {
            selector_word = next_random();
            selector_word
        },
    )
    .map_err(|_| Type12Block::Runtime("weighted selector"))?
    .ok_or(Type12Block::Runtime("empty weighted selector"))?;
    let context = BehaviorContextRuntime::from_fresh_weighted_selection(selection)
        .ok_or(Type12Block::Runtime("weighted selection context"))?;
    entity.initial_behavior = RetailRuntimeValue::Known(Some(selection));
    let succeeded = publish_acquiring(entity, metadata, selection, context, next_random);
    // The successful 104B0 wrapper builds the body matrix after the
    // initializer, preserving every authored Euler word.
    let [heading, pitch, roll] = entity.rotation_heading_pitch_roll_raw();
    entity.physical_body_basis_q31 = RetailRuntimeValue::Known(
        crate::common_mover::type9_attitude::Type9BodyBasis::from_angle_words(heading, pitch, roll),
    );
    entity.collision.state_flags_at_0x08.overwrite(
        crate::entity_collision_state::BODY_BASIS_REBUILT_STATE_BIT,
        crate::entity_collision_state::BODY_BASIS_REBUILT_STATE_BIT,
    );
    Ok(Type12Publication {
        selection,
        selector_word,
        people_nearby,
        player_nearby,
        buildings_nearby,
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
    let Some(row) = entity.native_type12_runtime.as_ref().map(|r| r.row) else {
        return false;
    };
    matches!(selection.program.class_id, 7 | 9 | 15 | 33)
        && shared::publication::publish_shared_root_class(
            entity,
            metadata,
            selection,
            context,
            row.axis(),
            next_random,
        )
}
