//! A Type2's method-25 shot: `44E770 -> 438080 -> 104B0/D4A0 -> D720`, then
//! the shooter at `+60`.

use super::*;
use crate::{
    native_type69::{self, Type69Error, Type69Owner, Type69Runtime},
    resource_cache::ResourceCache,
};

/// One modeled `44E770` record for method 25: the shot's position, its
/// velocity (the shooter's plus the table speed of 1) and the shooter's
/// current Euler words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type69BirthRequest {
    pub shooter_id: u32,
    pub position_raw: [i16; 3],
    pub velocity_raw: [i16; 3],
    pub rotation_raw: [i16; 3],
}

impl EntityManager {
    pub fn construct_native_type69(
        &mut self,
        request: Type69BirthRequest,
        resources: &ResourceCache,
        fx: &mut WorldFx,
        retail_tick: u32,
    ) -> Result<Type69Owner, Type69Error> {
        let entity_type = native_type69::ENTITY_TYPE;
        let metadata = resources
            .global_entity_type(entity_type as usize)
            .map(EntityTypeRuntimeMetadata::from_section12)
            .ok_or(Type69Error::Metadata)?;
        if self.type_metadata.get(entity_type as usize) != Some(&metadata) {
            return Err(Type69Error::Metadata);
        }
        native_type69::authenticate_metadata(&metadata)?;
        if resources.global_model(native_type69::MODEL).is_none() {
            return Err(Type69Error::Metadata);
        }
        let terrain = resources.level_terrain().ok_or(Type69Error::Terrain)?;
        let surface_bits = crate::entity_initializer::constructor_surface_bits_at_tick(
            request.position_raw,
            terrain,
            retail_tick,
            self.common_environment_physics().waves_enabled,
        )
        .ok_or(Type69Error::Terrain)?;
        // 14870 serialises the shooter's bit31; 44E770 refuses a remote one.
        let source = self
            .entities
            .iter()
            .find(|actor| actor.id == request.shooter_id)
            .ok_or(Type69Error::SourceAllocation)?;
        if !source.active
            || source.collision.state_flags_at_0x08.masked(0x8000_0000)
                != RetailRuntimeValue::Known(0)
            || self.main_base_abort_actor_observation(source.id).is_none()
        {
            return Err(Type69Error::SourceAllocation);
        }
        if !matches!(
            self.next_common_body_ordinal(),
            RetailRuntimeValue::Known(_)
        ) {
            return Err(Type69Error::Runtime("native body stamp lineage"));
        }
        let id = self.next_entity_id.max(1);
        if self.entities.iter().any(|actor| actor.id == id) {
            return Err(Type69Error::Allocation);
        }
        let stamp = self.begin_common_body_attempt();
        let mut entity =
            Entity::unresolved_port_entity(id, EntityKind::from_type(entity_type), entity_type);
        entity.construction_stamp_at_0xb4 = stamp;
        entity.set_motion_raw(request.position_raw, request.velocity_raw);
        entity.set_rotation_heading_pitch_roll_raw(request.rotation_raw);
        entity.mass_raw = metadata.mass_raw;
        entity.capability_flags = metadata.capability_flags;
        entity.model_slots = metadata.model_slots.map(|model| Some(usize::from(model)));
        entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(0);
        entity.sub_n_runtime = entity_sub_n_runtime_from_constructor(
            Some(&metadata),
            false,
            request.position_raw,
            None,
        );
        entity.base_factory_runtime =
            base_factory_runtime_from_constructor(entity_type, Some(&metadata), None);
        entity.actor_animation_runtime = actor_animation_runtime_from_constructor(Some(&metadata));
        entity.sub_g_06070_runtime = sub_g_06070_runtime_from_constructor(Some(&metadata));
        entity.sub_h_external_frame_runtime =
            sub_h_external_frame_runtime_from_constructor(Some(&metadata));
        entity.sub_j_attachment_runtime =
            sub_j_attachment_runtime_from_constructor(Some(&metadata), None, None);
        entity.actor_common_axis_descriptor =
            actor_common_axis_descriptor_from_constructor(Some(&metadata));
        entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(None);
        // AC60's singleton selector word, then B500.
        let selected =
            native_type69::select(&mut || u32::from(fx.next_shared_retail_random_u16()))?;
        let resolution =
            crate::entity_initializer::resolve_entity_initializer_with_selected_behavior(
                EntityInitializerRequest {
                    metadata: Some(&metadata),
                    spawn_param: 0,
                    authored_position_raw: request.position_raw,
                    terrain: Some(terrain),
                    resource_domain: ResourceDomainRelation::Current,
                },
                Some(selected),
            );
        let mut state_flags = resolution.state_flags;
        state_flags.overwrite(
            crate::entity_collision_state::SURFACE_STATE_MASK,
            surface_bits,
        );
        state_flags.overwrite(4, 0);
        entity.collision =
            EntityCollisionRuntimeState::from_constructor(Some(&metadata), 0, state_flags);
        // Labeled native policy for 104B0's unwritten transient B2 residue,
        // as for the other native births.
        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
        // 104B0's +44 modifier and the common vtable's +30 are null.
        entity.collision.pair_callbacks =
            EntityPairCallbackRuntimeState::audited_local(None, RetailRuntimeValue::Unresolved);
        entity.model_index = model_for_constructor_state(entity.model_slots, state_flags);
        entity.initial_behavior = RetailRuntimeValue::Known(Some(selected));
        let context = BehaviorContextRuntime::from_fresh_weighted_selection(selected)
            .ok_or(Type69Error::Selection)?;
        let allocation = observe_main_base_abort_actor(&entity, self.allocation_generation).lease;
        entity.native_type69_runtime = Some(Type69Runtime {
            allocation,
            shooter_id: request.shooter_id,
            quiet_death_context: None,
        });
        native_type69::publish_search(&mut entity, context);
        assert_eq!(self.allocate_live_entity_id(), id);
        self.append_live_entity(entity);
        // D720 links first, then builds 13F70 and bit4; 44E770 then stores
        // the shooter at +60.
        let linked = self.entities.last_mut().unwrap();
        let [heading, pitch, roll] = request.rotation_raw;
        linked.physical_body_basis_q31 =
            RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(heading, pitch, roll));
        linked.collision.state_flags_at_0x08.overwrite(4, 4);
        linked.collision.recent_relation_id_at_0x60 =
            RetailRuntimeValue::Known(Some(request.shooter_id));
        Type69Owner::adopt(self, id)
    }

    pub(crate) fn queue_born_native_type69(&mut self, owner: Type69Owner) {
        self.born_native_type69.push(owner);
    }

    pub(crate) fn take_born_native_type69(&mut self) -> Vec<Type69Owner> {
        std::mem::take(&mut self.born_native_type69)
    }
}
