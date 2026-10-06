//! Shared authored104B0/09A80/D4A0 entry with actual process component custody.

use super::*;
use crate::{
    actor_task_owner::ActorTaskSlot,
    common_mover::{sub_d::NativeSubDConstruction, SubAPropulsionRuntime},
    entity::EntityConstructionResources,
    entity_behavior::translate_state_policy,
    entity_collision_state::{
        RetailStateWord, ACTIVE_MODEL_SLOT_HIGH_STATE_BIT, SURFACE_STATE_MASK,
    },
};
use v2k_formats::levels::EntitySpawn;

pub(crate) struct NativeType40ConstructionRequest<'a> {
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

pub(crate) fn publish_native_type40(
    request: NativeType40ConstructionRequest<'_>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Type40Publication, Type40Error> {
    let NativeType40ConstructionRequest {
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
    authenticate_metadata(metadata)?;
    let slots = std::array::from_fn(|slot| {
        let model = if spawn.model_overrides[slot] == 0 {
            u32::from(metadata.model_slots[slot])
        } else {
            spawn.model_overrides[slot]
        };
        (model != 0).then_some(model as usize)
    });
    if !entity.active
        || entity.entity_type != 40
        || spawn.entity_type != 40
        || entity.id != allocation.entity_id
        || entity.authored_spawn_index != Some(spawn.index)
        || entity.capability_flags != metadata.capability_flags
        || entity.model_slots != slots
        || slots != [Some(MODEL); 4]
        || entity.rotation_heading_pitch_roll_raw() != spawn.rotation.map(|word| word as i16)
        || spawn.has_animation
        || spawn.animation.is_some()
        || spawn.has_config
        || spawn.config.is_some()
    {
        return Err(Type40Error::Identity);
    }
    if entity.native_type40_runtime.is_some()
        || entity.initial_behavior != RetailRuntimeValue::Unresolved
        || entity.current_behavior_context != RetailRuntimeValue::Unresolved
        || ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .iter()
            .any(|&slot| entity.actor_task_state(slot).is_some())
    {
        return Err(Type40Error::AlreadyPublished);
    }
    // Retain the actual successful authored prefix. Type40 only evaluates
    // UnderAttack/Always; no fabricated PeopleNearby or PlayerNearby query.
    if preceding.iter().any(|candidate| {
        candidate.id == entity.id
            || candidate
                .authored_spawn_index
                .is_some_and(|index| index >= spawn.index)
    }) || preceding.windows(2).any(|pair| {
        matches!(
            (pair[0].authored_spawn_index, pair[1].authored_spawn_index),
            (Some(a), Some(b)) if a >= b)
    }) {
        return Err(Type40Error::Prefix);
    }
    let RetailRuntimeValue::Known(Some(a)) = metadata.sub_a_propulsion_descriptor else {
        unreachable!()
    };
    if entity.actor_common_axis_descriptor != RetailRuntimeValue::Known(AXIS)
        || entity.collision.default_state_flags_at_0xc8 != RetailRuntimeValue::Known(0x39)
        || entity.sub_a_propulsion_runtime
            != RetailRuntimeValue::Known(Some(SubAPropulsionRuntime::pending_constructor_rng(a)))
        || entity.collision.health_raw != RetailRuntimeValue::Known(8000)
        || entity.collision.pre_health_damage_buffer_raw
            != RetailRuntimeValue::Known(spawn.initial_damage_buffer_raw)
        || entity.collision.last_hit_presentation_tick_at_0x34 != RetailRuntimeValue::Known(0)
        || entity.collision.recent_relation_id_at_0x60 != RetailRuntimeValue::Known(None)
        || entity.sub_j_attachment_runtime != RetailRuntimeValue::Known(None)
        || metadata.sub_d_steering_descriptor != RetailRuntimeValue::Known(Some(sub_d.descriptor))
        || sub_d.runtime != Type9SubDRuntime::from_constructor()
        || sub_d.frame_owner.classifier_cache().stagger_counter() != sub_d.seed
    {
        return Err(Type40Error::ComponentStorage);
    }
    if constructor_surface_bits & !SURFACE_STATE_MASK != 0 {
        return Err(Type40Error::Runtime("constructor surface bits"));
    }
    let terrain = resources
        .terrain
        .ok_or(Type40Error::Runtime("constructor terrain"))?;
    let [x, _, z] = spawn.position_raw();
    let cell = terrain
        .cell(usize::from((x as u16) >> 8), usize::from((z as u16) >> 8))
        .ok_or(Type40Error::Runtime("constructor terrain cell"))?;
    let RetailRuntimeValue::Known(anchor_raw) = crate::entity_initializer::constructor_position_raw(
        metadata,
        spawn.position_raw(),
        Some(terrain),
        None,
    ) else {
        return Err(Type40Error::Metadata);
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
    let policy = translate_state_policy(
        metadata
            .initializer
            .as_ref()
            .unwrap()
            .initializer_state_flags_raw,
    );
    state.overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);
    entity.collision.state_flags_at_0x08 = state;
    entity.model_index = Some(MODEL);
    // Common104B0 +44 and Type40 type-vtable+30 are both null.
    entity.collision.pair_callbacks.damage_modifier_address = RetailRuntimeValue::Known(None);
    entity
        .collision
        .pair_callbacks
        .damage_modifier_identity_context_empty = RetailRuntimeValue::Known(true);
    entity.collision.pair_callbacks.type_hit_callback_address = RetailRuntimeValue::Known(None);
    let receipt = Type40Runtime {
        entity_id: entity.id,
        spawn_index: spawn.index,
        model_slots: slots,
        anchor_raw,
        allocation,
        sub_d_runtime: sub_d.runtime,
        sub_d_owner: sub_d.frame_owner,
        sub_e_runtime: emitter_constructor(),
        split_terminal: None,
    };
    publish_birth(
        entity,
        metadata,
        preceding,
        receipt,
        retail_tick,
        next_random,
    )
}

fn publish_birth(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    _preceding: &[Entity],
    receipt: Type40Runtime,
    retail_tick: u32,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Type40Publication, Type40Error> {
    let RetailRuntimeValue::Known(Some(a)) = metadata.sub_a_propulsion_descriptor else {
        return Err(Type40Error::Metadata);
    };
    entity.native_type40_runtime = Some(receipt);
    entity.sub_h_external_frame_runtime = RetailRuntimeValue::Known(Some(
        crate::sub_h_external_frame::SubHRuntimeState::new(14)
            .map_err(|_| Type40Error::ComponentStorage)?,
    ));
    entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(
        crate::common_mover::SubAPropulsionRuntime::from_20450_constructor(a, next_random() as u16),
    ));
    entity.set_position_raw(receipt.anchor_raw);
    // Fresh+34=0 makes16490 false: tick<250 and signedtick>249 cannot both hold.
    let under_attack = retail_tick < 250 && (retail_tick as i32) > 249;
    let mut selector_word = 0;
    let selection = crate::entity_behavior::select_initial_behavior(
        &CHOICES,
        |rule| match rule {
            crate::entity_behavior::BehaviorWeightRule::Always => 1,
            crate::entity_behavior::BehaviorWeightRule::UnderAttack => i32::from(under_attack),
            _ => unreachable!("authenticated Type40 choices"),
        },
        || {
            selector_word = next_random();
            selector_word
        },
    )
    .map_err(|_| Type40Error::Selection)?
    .ok_or(Type40Error::Selection)?;
    let context = BehaviorContextRuntime::from_fresh_weighted_selection(selection)
        .ok_or(Type40Error::Selection)?;
    // Labeled native unwritten+B2 allocator-residue policy, shared with current
    // native bodies. This first write is bounded and does not assert retail acceptance.
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
    entity.initial_behavior = RetailRuntimeValue::Known(Some(selection));
    let succeeded =
        super::behavior::publish_acquiring(entity, metadata, selection, context, next_random);
    let [heading, pitch, roll] = entity.rotation_heading_pitch_roll_raw();
    entity.physical_body_basis_q31 = RetailRuntimeValue::Known(
        crate::common_mover::type9_attitude::Type9BodyBasis::from_angle_words(heading, pitch, roll),
    );
    entity.collision.state_flags_at_0x08.overwrite(
        crate::entity_collision_state::BODY_BASIS_REBUILT_STATE_BIT,
        crate::entity_collision_state::BODY_BASIS_REBUILT_STATE_BIT,
    );
    Ok(Type40Publication {
        selection,
        selector_word,
        initializer_fallback: !succeeded,
    })
}
