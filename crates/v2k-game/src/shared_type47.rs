//! Native Type47 `104B0 -> 09A80 -> D4A0 -> AC60` construction.
//!
//! The loaded manager owns allocation identity; the process owns Sub-D's seed.
//! Neither value comes from a captured spawn table. Guard and Wander retain
//! their shared task constructors and the exact 20450/selector/06070 RNG order.

use crate::{
    actor_task_owner::ActorTaskSlot,
    common_mover::{
        sub_d::{NativeSubDConstruction, Type9SubDRuntime},
        type9_attitude::Type9BodyBasis,
        SubAPropulsionRuntime,
    },
    entity::{Entity, EntityConstructionResources, EntityManager},
    entity_behavior::{
        select_initial_behavior, translate_state_policy, BehaviorContextRuntime, BehaviorWeightRule,
    },
    entity_collision_state::{
        EntityTypeRuntimeMetadata, RetailRuntimeValue, RetailStateWord,
        ACTIVE_MODEL_SLOT_HIGH_STATE_BIT, BODY_BASIS_REBUILT_STATE_BIT, SURFACE_STATE_MASK,
    },
    intro2_type47_live::{
        authenticate_type47_metadata, Intro2Type47Admission, Intro2Type47PublicationError,
        INTRO2_TYPE47_MODEL_ID,
    },
    main_base_abort::MainBaseAbortActorLease,
    ordinary_type47_death_live::{
        TYPE47_COMMON_DYING_BEHAVIOR_CHOICES, TYPE47_COMMON_DYING_COMMON_AXIS_DESCRIPTOR,
        TYPE47_COMMON_DYING_SUB_A_DESCRIPTOR,
    },
    ordinary_type47_live::{
        OrdinaryType47AimAndFireRuntime, FRESH_LEVEL1_ORDINARY_TYPE47_SUB_H_RECORD_COUNT,
    },
    sub_h_external_frame::SubHRuntimeState,
    type47_initial_behavior_live::{
        publish_type47_guard_location, publish_type47_wander_near,
        FreshType47InitializerPublication, TYPE47_GUARD_BEHAVIOR_CLASS_ID,
        TYPE47_WANDER_BEHAVIOR_CLASS_ID,
    },
};
use v2k_formats::levels::EntitySpawn;

/// Immutable birth custody, independent of mutable task, pose and death state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeType47Construction {
    allocation: MainBaseAbortActorLease,
    spawn_index: usize,
    model_slots: [Option<usize>; 4],
    anchor_raw: [i16; 3],
    sub_d_seed: u8,
}

impl NativeType47Construction {
    pub(crate) fn entity_authenticates(self, entity: &Entity) -> bool {
        entity.active
            && entity.entity_type == 47
            && entity.id == self.allocation.entity_id
            && entity.authored_spawn_index == Some(self.spawn_index)
            && entity.model_slots == self.model_slots
            && self.model_slots == [Some(INTRO2_TYPE47_MODEL_ID); 4]
            && entity.capability_flags == 8
            && entity.type47_immutable_anchor_raw_at_0x90
                == RetailRuntimeValue::Known(self.anchor_raw)
    }

    pub(crate) const fn sub_d_seed(self) -> u8 {
        self.sub_d_seed
    }
}

/// A present native receipt must prove its issuing manager, even if its spawn
/// number happens to overlap a replay cohort. The fallback authenticates the
/// explicit captured Intro2 emitter and Sub-D ownership, never a model alone.
pub(crate) fn type47_manager_allocation_authenticates(manager: &EntityManager, id: u32) -> bool {
    let Some(entity) = manager.iter_all().find(|entity| entity.id == id) else {
        return false;
    };
    if let Some(receipt) = entity.native_type47_construction {
        return receipt.entity_authenticates(entity)
            && manager
                .main_base_abort_actor_observation(id)
                .is_some_and(|observation| observation.lease == receipt.allocation);
    }
    crate::intro2_type47_live::intro2_type47_cohort_runtime_authenticates(entity)
}

pub(crate) struct Type47AuthoredConstructionRequest<'a> {
    pub entity: &'a mut Entity,
    pub allocation: MainBaseAbortActorLease,
    pub metadata: &'a EntityTypeRuntimeMetadata,
    pub spawn: &'a EntitySpawn,
    pub resources: EntityConstructionResources<'a>,
    /// D720 compares authored Y against the current wave surface before D4A0.
    pub constructor_surface_bits: u32,
    pub sub_d: NativeSubDConstruction,
}

pub(crate) fn publish_authored_type47(
    request: Type47AuthoredConstructionRequest<'_>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Intro2Type47Admission, Intro2Type47PublicationError> {
    let Type47AuthoredConstructionRequest {
        entity,
        allocation,
        metadata,
        spawn,
        resources,
        constructor_surface_bits,
        sub_d,
    } = request;
    authenticate_type47_metadata(metadata)?;
    if metadata.model_slots != [INTRO2_TYPE47_MODEL_ID as u16; 4]
        || metadata.mass_raw != 100
        || metadata.capability_flags != 8
        || metadata.initial_health_raw != Some(3000)
    {
        return Err(Intro2Type47PublicationError::MetadataMismatch);
    }
    let slots = std::array::from_fn(|slot| {
        let model = if spawn.model_overrides[slot] == 0 {
            u32::from(metadata.model_slots[slot])
        } else {
            spawn.model_overrides[slot]
        };
        (model != 0).then_some(model as usize)
    });
    if !entity.active
        || entity.entity_type != 47
        || entity.id != allocation.entity_id
        || spawn.entity_type != 47
        || entity.authored_spawn_index != Some(spawn.index)
        || entity.capability_flags != metadata.capability_flags
        || entity.model_slots != slots
        || slots != [Some(INTRO2_TYPE47_MODEL_ID); 4]
        || entity.rotation_heading_pitch_roll_raw() != spawn.rotation.map(|word| word as i16)
        || spawn.has_animation
        || spawn.animation.is_some()
        || spawn.has_config
        || spawn.config.is_some()
    {
        return Err(Intro2Type47PublicationError::EntityIdentityMismatch);
    }
    if entity.native_type47_construction.is_some()
        || entity.intro2_type47_sub_d_frame_owner.is_some()
        || entity.intro2_type47_sub_d_runtime.is_some()
        || entity.ordinary_type47_aim_and_fire_runtime.is_some()
        || entity.initial_behavior != RetailRuntimeValue::Unresolved
        || entity.current_behavior_context != RetailRuntimeValue::Unresolved
        || ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .iter()
            .any(|&slot| entity.actor_task_state(slot).is_some())
    {
        return Err(Intro2Type47PublicationError::AlreadyPublished);
    }
    if entity.actor_common_axis_descriptor
        != RetailRuntimeValue::Known(TYPE47_COMMON_DYING_COMMON_AXIS_DESCRIPTOR)
        || entity.sub_a_propulsion_runtime
            != RetailRuntimeValue::Known(Some(SubAPropulsionRuntime::pending_constructor_rng(
                TYPE47_COMMON_DYING_SUB_A_DESCRIPTOR,
            )))
        || entity.collision.health_raw != RetailRuntimeValue::Known(3000)
        || entity.collision.pre_health_damage_buffer_raw
            != RetailRuntimeValue::Known(spawn.initial_damage_buffer_raw)
        || metadata.sub_d_steering_descriptor != RetailRuntimeValue::Known(Some(sub_d.descriptor))
        || sub_d.runtime != Type9SubDRuntime::from_constructor()
        || sub_d.frame_owner.classifier_cache().stagger_counter() != sub_d.seed
    {
        return Err(Intro2Type47PublicationError::MetadataMismatch);
    }
    match (
        &metadata.sub_j_attachment_descriptor,
        &entity.sub_j_attachment_runtime,
    ) {
        (RetailRuntimeValue::Known(Some(descriptor)), RetailRuntimeValue::Known(Some(runtime)))
            if runtime.is_empty() && runtime.authored_slot_count() == descriptor.slots.len() => {}
        _ => return Err(Intro2Type47PublicationError::MetadataMismatch),
    }
    let emitter =
        OrdinaryType47AimAndFireRuntime::from_authenticated_native_metadata(metadata, sub_d.seed)
            .ok_or(Intro2Type47PublicationError::MetadataMismatch)?;
    let sub_h = SubHRuntimeState::new(FRESH_LEVEL1_ORDINARY_TYPE47_SUB_H_RECORD_COUNT)
        .map_err(|_| Intro2Type47PublicationError::SubHRuntimeUnavailable)?;
    if constructor_surface_bits & !SURFACE_STATE_MASK != 0 {
        return Err(Intro2Type47PublicationError::MetadataMismatch);
    }
    let terrain = resources
        .terrain
        .ok_or(Intro2Type47PublicationError::Setup)?;
    let [x, _, z] = spawn.position_raw();
    let cell = terrain
        .cell(usize::from((x as u16) >> 8), usize::from((z as u16) >> 8))
        .ok_or(Intro2Type47PublicationError::Setup)?;
    let RetailRuntimeValue::Known(anchor_raw) = crate::entity_initializer::constructor_position_raw(
        metadata,
        spawn.position_raw(),
        Some(terrain),
        None,
    ) else {
        return Err(Intro2Type47PublicationError::MetadataMismatch);
    };

    // 104B0's common body, D720 surface/model bits and authored type policy.
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
            .expect("authenticated initializer")
            .initializer_state_flags_raw,
    );
    state.overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);
    entity.collision.state_flags_at_0x08 = state;
    entity.model_index = Some(INTRO2_TYPE47_MODEL_ID);
    entity.collision.pair_callbacks.damage_modifier_address = RetailRuntimeValue::Known(None);
    entity
        .collision
        .pair_callbacks
        .damage_modifier_identity_context_empty = RetailRuntimeValue::Known(true);
    entity.collision.pair_callbacks.type_hit_callback_address = RetailRuntimeValue::Known(None);

    // 09A80 reaches 20450 before the selector. Native Sub-D is already the
    // actual component allocation; never replace it with a captured seed.
    entity.sub_a_propulsion_runtime =
        RetailRuntimeValue::Known(Some(SubAPropulsionRuntime::from_20450_constructor(
            TYPE47_COMMON_DYING_SUB_A_DESCRIPTOR,
            next_random() as u16,
        )));
    entity.sub_h_external_frame_runtime = RetailRuntimeValue::Known(Some(sub_h));
    entity.intro2_type47_sub_d_frame_owner = Some(sub_d.frame_owner);
    entity.intro2_type47_sub_d_runtime = Some(sub_d.runtime);
    entity.ordinary_type47_aim_and_fire_runtime = Some(emitter);
    entity.set_position_raw(anchor_raw);
    entity.type47_immutable_anchor_raw_at_0x90 = RetailRuntimeValue::Known(anchor_raw);
    entity.native_type47_construction = Some(NativeType47Construction {
        allocation,
        spawn_index: spawn.index,
        model_slots: slots,
        anchor_raw,
        sub_d_seed: sub_d.seed,
    });
    // Explicit native policy for allocator residue, only at this birth.
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
    let selection = select_initial_behavior(
        &TYPE47_COMMON_DYING_BEHAVIOR_CHOICES,
        |rule| i32::from(rule == BehaviorWeightRule::Always),
        &mut *next_random,
    )
    .map_err(|_| Intro2Type47PublicationError::Setup)?
    .ok_or(Intro2Type47PublicationError::BehaviorContextUnavailable)?;
    let context = BehaviorContextRuntime::from_fresh_weighted_selection(selection)
        .ok_or(Intro2Type47PublicationError::BehaviorContextUnavailable)?;
    let publication = match selection.program.class_id {
        TYPE47_GUARD_BEHAVIOR_CLASS_ID => {
            publish_type47_guard_location(entity, context, metadata, next_random)
        }
        TYPE47_WANDER_BEHAVIOR_CLASS_ID => {
            publish_type47_wander_near(entity, context, metadata, next_random)
        }
        _ => return Err(Intro2Type47PublicationError::Setup),
    };
    entity.initial_behavior = RetailRuntimeValue::Known(Some(selection));
    // The successful104B0 wrapper uses actual authored Euler words, including
    // the nonzero headings in ordinary worlds and Intro2's third allocation.
    let [heading, pitch, roll] = entity.rotation_heading_pitch_roll_raw();
    entity.physical_body_basis_q31 =
        RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(heading, pitch, roll));
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(BODY_BASIS_REBUILT_STATE_BIT, BODY_BASIS_REBUILT_STATE_BIT);
    if matches!(
        publication,
        FreshType47InitializerPublication::InitializerFallback { .. }
    ) {
        return Err(Intro2Type47PublicationError::Setup);
    }
    Ok(Intro2Type47Admission {
        spawn_index: spawn.index,
        seed: sub_d.seed,
    })
}

#[cfg(test)]
pub(crate) mod tests;
