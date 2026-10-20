//! Real 438080 -> 104B0/09A80/D4A0 -> D720 Type105 allocations for Type31's
//! class18 split.

use super::*;
use crate::{
    native_type31::{self, Type105BirthPublication, Type31Block, Type31Row, Type31Runtime},
    resource_cache::ResourceCache,
    split_and_explode::SplitChildRequest,
};

impl EntityManager {
    /// Each child is appended synchronously before D720 returns. The birth
    /// record has no authored index, relation, health buffer, score or model
    /// overrides, and its null handle selects 104B0's fresh `43A290` handle.
    pub fn construct_native_type105(
        &mut self,
        request: SplitChildRequest,
        resources: &ResourceCache,
        fx: &mut WorldFx,
        retail_tick: u32,
    ) -> Result<Type105BirthPublication, Type31Block> {
        let row = Type31Row::Type105;
        if request.entity_type != row.entity_type() {
            return Err(Type31Block::Identity);
        }
        if request.requested_entity_handle_raw != 0 {
            return Err(Type31Block::Runtime("nonzero requested native handle"));
        }
        let metadata = resources
            .global_entity_type(105)
            .map(EntityTypeRuntimeMetadata::from_section12)
            .ok_or(Type31Block::Metadata)?;
        if self.type_metadata.get(105) != Some(&metadata) {
            return Err(Type31Block::Metadata);
        }
        native_type31::authenticate_metadata(row, &metadata)?;
        if resources.global_model(row.model()).is_none() {
            return Err(Type31Block::Metadata);
        }
        let terrain = resources.level_terrain().ok_or(Type31Block::Terrain)?;
        let surface = crate::entity_initializer::constructor_surface_bits_at_tick(
            request.position_raw,
            terrain,
            retail_tick,
            self.common_environment_physics().waves_enabled,
        )
        .ok_or(Type31Block::Terrain)?;
        let [x, _, z] = request.position_raw;
        let cell = terrain
            .cell(usize::from((x as u16) >> 8), usize::from((z as u16) >> 8))
            .ok_or(Type31Block::Terrain)?;
        let RetailRuntimeValue::Known(anchor) = crate::entity_initializer::constructor_position_raw(
            &metadata,
            request.position_raw,
            Some(terrain),
            None,
        ) else {
            return Err(Type31Block::Terrain);
        };
        if !matches!(
            self.next_common_body_ordinal(),
            RetailRuntimeValue::Known(_)
        ) {
            return Err(Type31Block::Runtime("native body stamp lineage"));
        }
        let id = self.next_entity_id.max(1);
        if self.entities.iter().any(|entity| entity.id == id) {
            return Err(Type31Block::Allocation);
        }
        let stamp = self.begin_common_body_attempt();
        let mut entity = Entity::unresolved_port_entity(id, EntityKind::from_type(105), 105);
        entity.construction_stamp_at_0xb4 = stamp;
        entity.set_motion_raw(request.position_raw, request.velocity_raw);
        entity.set_rotation_heading_pitch_roll_raw(
            request
                .rotation_heading_pitch_roll_raw
                .map(|word| word as i16),
        );
        entity.mass_raw = metadata.mass_raw;
        entity.capability_flags = metadata.capability_flags;
        entity.model_slots = metadata.model_slots.map(|word| Some(usize::from(word)));
        entity.authored_follow_beacon_priority_raw = Some(0); // zeroed birth+20 -> actor+88
        entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(0);
        entity.sub_n_runtime = entity_sub_n_runtime_from_constructor(
            Some(&metadata),
            false,
            request.position_raw,
            None,
        );
        entity.base_factory_runtime =
            base_factory_runtime_from_constructor(105, Some(&metadata), None);
        entity.actor_animation_runtime = actor_animation_runtime_from_constructor(Some(&metadata));
        entity.sub_g_06070_runtime = sub_g_06070_runtime_from_constructor(Some(&metadata));
        entity.sub_h_external_frame_runtime =
            sub_h_external_frame_runtime_from_constructor(Some(&metadata));
        // 09A80 builds the process-wide D before 20450 consumes its word.
        let sub_d = fx.construct_entity_sub_d(row.sub_d());
        entity.sub_j_attachment_runtime =
            sub_j_attachment_runtime_from_constructor(Some(&metadata), None, None);
        entity.actor_common_axis_descriptor =
            actor_common_axis_descriptor_from_constructor(Some(&metadata));
        let mut flags = crate::entity_collision_state::RetailStateWord::exact(
            0x0607_8801 | if request.objective { 0x0100_0000 } else { 0 },
        );
        flags.overwrite(crate::entity_collision_state::SURFACE_STATE_MASK, surface);
        flags.overwrite(
            crate::entity_collision_state::ACTIVE_MODEL_SLOT_HIGH_STATE_BIT,
            if cell.terrain_type & 0x10 != 0 {
                crate::entity_collision_state::ACTIVE_MODEL_SLOT_HIGH_STATE_BIT
            } else {
                0
            },
        );
        let policy = crate::entity_behavior::translate_state_policy(row.default_flags());
        flags.overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);
        flags.overwrite(4, 0);
        entity.collision = EntityCollisionRuntimeState::from_constructor(Some(&metadata), 0, flags);
        // 13500 builds 02DA0 on every shared Wander/Retarget/RunAway primary;
        // the remaining task slots have null component contacts.
        entity.collision.pair_callbacks = EntityPairCallbackRuntimeState::audited_local(
            Some(0x0040_2DA0),
            RetailRuntimeValue::Unresolved,
        );
        entity.model_index = model_for_constructor_state(entity.model_slots, flags);
        let allocation = observe_main_base_abort_actor(&entity, self.allocation_generation).lease;
        let receipt = Type31Runtime {
            row,
            allocation,
            spawn_index: None,
            birth: Some(request),
            anchor_raw: anchor,
            sub_d_runtime: sub_d.runtime,
            sub_d_owner: sub_d.frame_owner,
            sub_e_runtime: native_type31::emitter_constructor(),
            split_terminal: None,
            quiet_death_context: None,
        };
        let publication = native_type31::construction::publish_birth(
            &mut entity,
            &metadata,
            receipt,
            retail_tick,
            &mut || u32::from(fx.next_shared_retail_random_u16()),
        )?;
        assert_eq!(self.allocate_live_entity_id(), id);
        self.append_live_entity(entity);
        // 104B0 links first; the successful D720 wrapper builds 13F70 and bit4.
        let entity = self.entities.last_mut().unwrap();
        let [heading, pitch, roll] = entity.rotation_heading_pitch_roll_raw();
        entity.physical_body_basis_q31 =
            RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(heading, pitch, roll));
        entity.collision.state_flags_at_0x08.overwrite(4, 4);
        let owner = native_type31::Type105Owner::adopt(self, id).map_err(Type31Block::Ground)?;
        Ok(Type105BirthPublication { owner, publication })
    }
}
