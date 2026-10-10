//! Shared104B0 body for source zero-instance requests, before family publication.

use super::*;

/// Source4C creation-record fields supplied by the caller. All other instance
/// words are zero in the supported zero-record and Hive-child creation paths.
#[derive(Debug, Clone, Copy)]
pub(crate) struct NativeInstanceBodyRequest {
    pub entity_type: u32,
    /// Signed8.8 words copied before the constructor's surface comparison.
    pub position_raw: [i16; 3],
    pub objective: bool,
}

impl NativeInstanceBodyRequest {
    pub const fn zeroed(entity_type: u32) -> Self {
        Self {
            entity_type,
            position_raw: [0; 3],
            objective: false,
        }
    }
}

/// Body defaults and intrinsic components, independent from family tasks.
/// The caller owns the successful Sub-D allocation, family RNG/publication,
/// current-tick surface bits and any post-D4A0 placement/anchor suffix.
pub(crate) fn native_instance_body(
    id: u32,
    construction_stamp_at_0xb4: RetailRuntimeValue<u16>,
    metadata: &EntityTypeRuntimeMetadata,
    terrain: &TerrainGrid,
    request: NativeInstanceBodyRequest,
) -> Entity {
    let entity_type = request.entity_type;
    let resolution = resolve_entity_initializer(EntityInitializerRequest {
        metadata: Some(metadata),
        spawn_param: u32::from(request.objective),
        authored_position_raw: request.position_raw,
        terrain: Some(terrain),
        resource_domain: ResourceDomainRelation::Current,
    });
    let model_slots = metadata
        .model_slots
        .map(|model| (model != 0).then_some(usize::from(model)));
    let mut collision =
        EntityCollisionRuntimeState::from_constructor(Some(metadata), 0, resolution.state_flags);
    // 104B0 zeroes +44 before a family may install a damage modifier. Other
    // callback slots still require their actual family publication.
    collision.pair_callbacks =
        EntityPairCallbackRuntimeState::unresolved_with_constructor_null_modifier();
    Entity {
        id,
        construction_stamp_at_0xb4,
        authored_spawn_index: None,
        kind: EntityKind::from_type(entity_type),
        entity_type,
        authored_follow_beacon_priority_raw: None,
        power_up_payload_packed: None,
        auto_pilot_payload_packed: None,
        factory_type61_birth_provenance: None,
        type60_construction_provenance: None,
        main_base_type54_sea_delta_source: RetailRuntimeValue::Unresolved,
        position: request.position_raw.map(|word| f32::from(word) / 256.0),
        heading: 0.0,
        pitch_roll_raw: [0; 2],
        physical_body_basis_q31: RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(
            0, 0, 0,
        )),
        velocity: [0.0; 3],
        surface_lifetime_timer_ms_at_0x48: RetailRuntimeValue::Known(0),
        mass_raw: metadata.mass_raw,
        capability_flags: metadata.capability_flags,
        attached_to: None,
        model_slots,
        model_index: model_for_constructor_state(model_slots, resolution.state_flags),
        collision,
        initial_behavior: resolution.initial_behavior,
        current_behavior_context: resolution.current_behavior_context,
        authored_radial_emitter: None,
        sub_n_runtime: entity_sub_n_runtime_from_constructor(
            Some(metadata),
            false,
            request.position_raw,
            None,
        ),
        base_factory_runtime: base_factory_runtime_from_constructor(
            entity_type,
            Some(metadata),
            None,
        ),
        actor_animation_runtime: actor_animation_runtime_from_constructor(Some(metadata)),
        sub_a_propulsion_runtime: sub_a_propulsion_runtime_from_constructor(Some(metadata)),
        sub_g_06070_runtime: sub_g_06070_runtime_from_constructor(Some(metadata)),
        intro2_type13_common_mover_runtime: None,
        native_type13_allocation: None,
        intro2_type13_aim_runtime: None,
        intro2_type16_aim_runtime: None,
        intro2_type58_aim_runtime: None,
        intro2_type94_aim_runtime: None,
        intro2_flyer_aim_runtime: None,
        sub_h_external_frame_runtime: sub_h_external_frame_runtime_from_constructor(Some(metadata)),
        sub_j_attachment_runtime: sub_j_attachment_runtime_from_constructor(
            Some(metadata),
            None,
            None,
        ),
        actor_common_axis_descriptor: actor_common_axis_descriptor_from_constructor(Some(metadata)),
        actor_tasks: ActorTaskOwner::new(),
        ordinary_type9_pending_initial_selection: None,
        ordinary_type9_selected_component_runtime: None,
        main_base_type9_death_component_runtime: None,
        ordinary_type47_aim_and_fire_runtime: None,
        native_type61_allocation: None,
        native_type26_allocation: None,
        intro2_type26_sub_d_frame_owner: None,
        intro2_type26_sub_d_runtime: None,
        intro2_type47_sub_d_frame_owner: None,
        intro2_type47_sub_d_runtime: None,
        native_type47_construction: None,
        intro2_flyer_frame_owner: None,
        intro2_type53_runtime: None,
        native_type122_runtime: None,
        native_type30_runtime: None,
        native_type30_aim_runtime: None,
        native_type40_runtime: None,
        native_type40_aim_runtime: None,
        native_type43_runtime: None,
        native_type43_aim_runtime: None,
        native_type56_runtime: None,
        native_type56_aim_runtime: None,
        native_type122_aim_runtime: None,
        shared_fish_runtime: None,
        cleansing_vehicle_runtime: None,
        intro2_type16_runtime: None,
        intro2_type58_runtime: None,
        intro2_type66_runtime: None,
        intro2_gun_turret_runtime: None,
        intro2_gun_turret_aim_runtime: None,
        class49_death_runtime: None,
        native_entity_weapon_runtime: None,
        intro2_type10_runtime: None,
        intro2_type10_aim_runtime: None,
        intro2_type57_runtime: None,
        intro2_type57_aim_runtime: None,
        intro2_type94_runtime: None,
        intro2_type17_runtime: None,
        native_capture_relation: None,
        intro2_type8_runtime: None,
        native_type123_runtime: None,
        native_type123_anchor_raw_at_0x90: RetailRuntimeValue::Unresolved,
        native_type86_runtime: None,
        native_type86_anchor_raw_at_0x90: RetailRuntimeValue::Unresolved,
        intro2_type9_runtime: None,
        ordinary_type9_native_receipt: None,
        main_base_runtime: None,
        type17_sub_d_frame_owner: None,
        type17_sub_d_runtime: None,
        type8_sub_d_frame_owner: None,
        type8_wander_anchor_raw_at_0x90: RetailRuntimeValue::Unresolved,
        type8_sub_d_runtime: None,
        type47_immutable_anchor_raw_at_0x90: RetailRuntimeValue::Unresolved,
        active: true,
    }
}
