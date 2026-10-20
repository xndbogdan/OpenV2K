//! Shared authored 104B0/09A80/D4A0 entry for Type31, and the weighted
//! publication both rows run once their components exist.

use super::*;
use crate::{
    actor_task_owner::ActorTaskSlot,
    common_mover::{sub_d::NativeSubDConstruction, SubAPropulsionRuntime},
    entity::EntityConstructionResources,
    entity_behavior::{select_initial_behavior, translate_state_policy, BehaviorWeightRule},
    entity_collision_state::{
        RetailStateWord, ACTIVE_MODEL_SLOT_HIGH_STATE_BIT, SURFACE_STATE_MASK,
    },
};
use v2k_formats::levels::EntitySpawn;

pub(crate) struct NativeType31ConstructionRequest<'a> {
    pub entity: &'a mut Entity,
    pub allocation: MainBaseAbortActorLease,
    pub metadata: &'a EntityTypeRuntimeMetadata,
    pub spawn: &'a EntitySpawn,
    pub preceding: &'a [Entity],
    pub resources: EntityConstructionResources<'a>,
    pub constructor_surface_bits: u32,
    pub retail_tick: u32,
    pub sub_d: NativeSubDConstruction,
}

pub(crate) fn publish_native_type31(
    request: NativeType31ConstructionRequest<'_>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Type31Publication, Type31Block> {
    let NativeType31ConstructionRequest {
        entity,
        allocation,
        metadata,
        spawn,
        preceding,
        resources,
        constructor_surface_bits,
        retail_tick,
        sub_d,
    } = request;
    let row = Type31Row::Type31;
    authenticate_metadata(row, metadata)?;
    let slots = std::array::from_fn(|slot| {
        let model = if spawn.model_overrides[slot] == 0 {
            u32::from(metadata.model_slots[slot])
        } else {
            spawn.model_overrides[slot]
        };
        (model != 0).then_some(model as usize)
    });
    if !entity.active
        || entity.entity_type != row.entity_type()
        || u32::from(spawn.entity_type) != row.entity_type()
        || entity.id != allocation.entity_id
        || entity.authored_spawn_index != Some(spawn.index)
        || entity.capability_flags != metadata.capability_flags
        || entity.model_slots != slots
        || slots != [Some(row.model()); 4]
        || entity.rotation_heading_pitch_roll_raw() != spawn.rotation.map(|word| word as i16)
        || spawn.has_animation
        || spawn.animation.is_some()
        || spawn.has_config
        || spawn.config.is_some()
    {
        return Err(Type31Block::Identity);
    }
    if entity.native_type31_runtime.is_some()
        || entity.initial_behavior != RetailRuntimeValue::Unresolved
        || entity.current_behavior_context != RetailRuntimeValue::Unresolved
        || ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .iter()
            .any(|&slot| entity.actor_task_state(slot).is_some())
    {
        return Err(Type31Block::AlreadyPublished);
    }
    // Type31 evaluates only UnderAttack and Always: no candidate scan.
    if preceding.iter().any(|candidate| {
        candidate.id == entity.id
            || candidate
                .authored_spawn_index
                .is_some_and(|index| index >= spawn.index)
    }) || preceding.windows(2).any(|pair| {
        matches!((pair[0].authored_spawn_index, pair[1].authored_spawn_index),
            (Some(a), Some(b)) if a >= b)
    }) {
        return Err(Type31Block::Prefix);
    }
    if entity.actor_common_axis_descriptor != RetailRuntimeValue::Known(row.axis())
        || entity.collision.default_state_flags_at_0xc8
            != RetailRuntimeValue::Known(row.default_flags())
        || entity.sub_a_propulsion_runtime
            != RetailRuntimeValue::Known(Some(SubAPropulsionRuntime::pending_constructor_rng(
                row.sub_a(),
            )))
        || entity.collision.health_raw != RetailRuntimeValue::Known(row.health())
        || entity.collision.pre_health_damage_buffer_raw
            != RetailRuntimeValue::Known(spawn.initial_damage_buffer_raw)
        || entity.collision.last_hit_presentation_tick_at_0x34 != RetailRuntimeValue::Known(0)
        || entity.collision.recent_relation_id_at_0x60 != RetailRuntimeValue::Known(None)
        || entity.sub_j_attachment_runtime != RetailRuntimeValue::Known(None)
        || metadata.sub_d_steering_descriptor != RetailRuntimeValue::Known(Some(sub_d.descriptor))
        || sub_d.runtime != Type9SubDRuntime::from_constructor()
        || sub_d.frame_owner.classifier_cache().stagger_counter() != sub_d.seed
    {
        return Err(Type31Block::ComponentStorage);
    }
    if constructor_surface_bits & !SURFACE_STATE_MASK != 0 {
        return Err(Type31Block::Runtime("constructor surface bits"));
    }
    let terrain = resources
        .terrain
        .ok_or(Type31Block::Runtime("constructor terrain"))?;
    let [x, _, z] = spawn.position_raw();
    let cell = terrain
        .cell(usize::from((x as u16) >> 8), usize::from((z as u16) >> 8))
        .ok_or(Type31Block::Runtime("constructor terrain cell"))?;
    let RetailRuntimeValue::Known(anchor_raw) = crate::entity_initializer::constructor_position_raw(
        metadata,
        spawn.position_raw(),
        Some(terrain),
        None,
    ) else {
        return Err(Type31Block::Metadata);
    };
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
    entity.sub_h_external_frame_runtime = RetailRuntimeValue::Known(Some(
        crate::sub_h_external_frame::SubHRuntimeState::new(TYPE31_SUB_H_REFS.len())
            .map_err(|_| Type31Block::ComponentStorage)?,
    ));
    let runtime = Type31Runtime {
        row,
        allocation,
        spawn_index: Some(spawn.index),
        birth: None,
        anchor_raw,
        sub_d_runtime: sub_d.runtime,
        sub_d_owner: sub_d.frame_owner,
        sub_e_runtime: emitter_constructor(),
        split_terminal: None,
        quiet_death_context: None,
    };
    let publication = publish_birth(entity, metadata, runtime, retail_tick, next_random)?;
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
    Ok(publication)
}

/// 09A80's A draw (20450), then 25680's selector and the selected 06070
/// suffix. A fresh `+34` of zero leaves UnderAttack false: `16490` needs
/// the tick both below 250 and, signed, above 249.
pub(crate) fn publish_birth(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    runtime: Type31Runtime,
    retail_tick: u32,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Type31Publication, Type31Block> {
    let row = runtime.row;
    let a = SubAPropulsionRuntime::from_20450_constructor(row.sub_a(), next_random() as u16);
    entity.set_position_raw(runtime.anchor_raw);
    entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(a));
    entity.native_type31_runtime = Some(runtime);
    // Labeled native allocation policy shared with the other native bodies:
    // transient B2 is unwritten allocator residue, seeded once at birth.
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
    let under_attack = retail_tick < 250 && (retail_tick as i32) > 249;
    let mut selector_word = 0;
    let selection = select_initial_behavior(
        row.choices(),
        |rule| match rule {
            BehaviorWeightRule::Always => 1,
            BehaviorWeightRule::UnderAttack => i32::from(under_attack),
            _ => unreachable!("authenticated Type31 rule list"),
        },
        || {
            selector_word = next_random();
            selector_word
        },
    )
    .map_err(|_| Type31Block::Runtime("weighted selector"))?
    .ok_or(Type31Block::Runtime("empty weighted selector"))?;
    let context = BehaviorContextRuntime::from_fresh_weighted_selection(selection)
        .ok_or(Type31Block::Runtime("weighted selection context"))?;
    entity.initial_behavior = RetailRuntimeValue::Known(Some(selection));
    let succeeded = publish_acquiring(entity, metadata, selection, context, next_random);
    Ok(Type31Publication {
        selection,
        selector_word,
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
    let Some(row) = entity.native_type31_runtime.map(|runtime| runtime.row) else {
        return false;
    };
    row.choices()
        .iter()
        .any(|choice| u32::from(choice.behavior_class_id) == u32::from(selection.program.class_id))
        && shared::publication::publish_shared_root_class(
            entity,
            metadata,
            selection,
            context,
            row.axis(),
            next_random,
        )
}
