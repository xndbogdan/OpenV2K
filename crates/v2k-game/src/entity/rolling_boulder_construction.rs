//! Real `438080 -> 104B0/09A80/D4A0 -> AC60 -> 40B950` Type3 boulders for
//! Type27's class18 split.

use super::*;
use crate::{
    resource_cache::ResourceCache,
    rolling_boulder::{
        publish_split_rolling_boulder, RollingBoulderBlock, RollingBoulderOwner,
        RollingBoulderProfile, DEFAULT_STATE_POLICY,
    },
    split_and_explode::SplitChildRequest,
};

impl EntityManager {
    /// Each child is linked synchronously before D720 returns. The zeroed birth
    /// record carries no authored index, relation, health buffer or model
    /// override; birth+00 must be the null/default handle, so 104B0 takes a
    /// fresh handle through 43A290. A boulder has no components, so 09A80
    /// builds none and AC60's one choice word is the only constructor draw.
    pub fn construct_native_rolling_boulder(
        &mut self,
        request: SplitChildRequest,
        resources: &ResourceCache,
        fx: &mut WorldFx,
        retail_tick: u32,
    ) -> Result<RollingBoulderOwner, RollingBoulderBlock> {
        let profile = RollingBoulderProfile::for_entity_type(request.entity_type)
            .ok_or(RollingBoulderBlock::Identity)?;
        if request.requested_entity_handle_raw != 0 {
            return Err(RollingBoulderBlock::Runtime(
                "nonzero requested native handle",
            ));
        }
        let entity_type = profile.entity_type();
        let metadata = resources
            .global_entity_type(entity_type as usize)
            .map(EntityTypeRuntimeMetadata::from_section12)
            .ok_or(RollingBoulderBlock::Metadata)?;
        if self.type_metadata.get(entity_type as usize) != Some(&metadata) {
            return Err(RollingBoulderBlock::Metadata);
        }
        crate::rolling_boulder::authenticate_metadata(profile, &metadata)?;
        let model = resources
            .global_model(usize::from(profile.model()))
            .ok_or(RollingBoulderBlock::Model)?;
        let terrain = resources
            .level_terrain()
            .ok_or(RollingBoulderBlock::Runtime("constructor terrain"))?;
        let surface = crate::entity_initializer::constructor_surface_bits_at_tick(
            request.position_raw,
            terrain,
            retail_tick,
            self.common_environment_physics().waves_enabled,
        )
        .ok_or(RollingBoulderBlock::Runtime("constructor surface"))?;
        let [x, _, z] = request.position_raw;
        let cell = terrain
            .cell(usize::from((x as u16) >> 8), usize::from((z as u16) >> 8))
            .ok_or(RollingBoulderBlock::Runtime("constructor cell"))?;
        // D4A0 bit20 grounds on bilinear terrain; bit40 adds the active
        // model's header+08. All four slots hold the same boulder model.
        let RetailRuntimeValue::Known(anchor) = crate::entity_initializer::constructor_position_raw(
            &metadata,
            request.position_raw,
            Some(terrain),
            Some(model.radius as i16),
        ) else {
            return Err(RollingBoulderBlock::Runtime("constructor position"));
        };
        if !matches!(
            self.next_common_body_ordinal(),
            RetailRuntimeValue::Known(_)
        ) {
            return Err(RollingBoulderBlock::Runtime("native body stamp lineage"));
        }
        let id = self.next_entity_id.max(1);
        if self.entities.iter().any(|entity| entity.id == id) {
            return Err(RollingBoulderBlock::Allocation);
        }
        let stamp = self.begin_common_body_attempt();
        let mut entity =
            Entity::unresolved_port_entity(id, EntityKind::from_type(entity_type), entity_type);
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
            base_factory_runtime_from_constructor(entity_type, Some(&metadata), None);
        entity.actor_animation_runtime = actor_animation_runtime_from_constructor(Some(&metadata));
        entity.sub_g_06070_runtime = sub_g_06070_runtime_from_constructor(Some(&metadata));
        entity.sub_h_external_frame_runtime =
            sub_h_external_frame_runtime_from_constructor(Some(&metadata));
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
        let policy = crate::entity_behavior::translate_state_policy(DEFAULT_STATE_POLICY);
        flags.overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);
        flags.overwrite(BODY_BASIS_REBUILT_STATE_BIT, 0);
        entity.collision = EntityCollisionRuntimeState::from_constructor(Some(&metadata), 0, flags);
        entity.model_index = model_for_constructor_state(entity.model_slots, flags);
        let allocation = observe_main_base_abort_actor(&entity, self.allocation_generation).lease;
        publish_split_rolling_boulder(
            &mut entity,
            &metadata,
            profile,
            allocation,
            anchor,
            &mut || u32::from(fx.next_shared_retail_random_u16()),
        )?;
        assert_eq!(self.allocate_live_entity_id(), id);
        self.append_live_entity(entity);
        // 104B0 links first; the successful D720 wrapper builds 13F70 and bit4.
        self.entities
            .last_mut()
            .unwrap()
            .apply_d720_euler_body_basis();
        RollingBoulderOwner::adopt(self, id)
    }
}
