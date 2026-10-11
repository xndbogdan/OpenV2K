//! `40BC90`'s drop: zero-record `438080 -> 104B0 -> AC60/257F0` Type61 at the
//! dying carrier, then `4575A0` disposes the constructor's result object.

use super::*;
use crate::native_type61::{
    publish_auto_pilot_power_up, AutoPilotPowerUpBirth, AutoPilotPowerUpConstruction,
};

impl EntityManager {
    /// Every input the constructor reads before its selector draw. Class63
    /// checks this before BAF0 commits, so a later birth cannot fail on data.
    pub(crate) fn auto_pilot_power_up_constructor_ready(
        &self,
        position_raw: [i16; 3],
        terrain: &TerrainGrid,
    ) -> bool {
        self.type_metadata
            .get(LEVEL_ONE_TYPE61_ENTITY_TYPE as usize)
            .is_some_and(exact_level_one_type61_metadata)
            && matches!(
                self.next_common_body_ordinal(),
                RetailRuntimeValue::Known(_)
            )
            && std::num::NonZeroU32::new(self.next_entity_id).is_some()
            && terrain
                .cell(
                    usize::from(position_raw[0] as u16 >> 8),
                    usize::from(position_raw[2] as u16 >> 8),
                )
                .is_some()
    }

    /// Tail-append the carrier's Type61. Its surface bits use the same
    /// current-tick comparison as every other runtime `104B0` birth.
    pub(crate) fn append_auto_pilot_power_up(
        &mut self,
        birth: AutoPilotPowerUpBirth,
        terrain: &TerrainGrid,
        retail_tick: u32,
        world_fx: &mut WorldFx,
    ) -> Result<u32, &'static str> {
        if !self.auto_pilot_power_up_constructor_ready(birth.position_raw, terrain) {
            return Err("Type61 auto-pilot constructor inputs");
        }
        let metadata = self.type_metadata[LEVEL_ONE_TYPE61_ENTITY_TYPE as usize].clone();
        let constructor_surface_bits = crate::entity_initializer::constructor_surface_bits_at_tick(
            birth.position_raw,
            terrain,
            retail_tick,
            self.common_environment_physics().waves_enabled,
        )
        .ok_or("Type61 auto-pilot constructor surface")?;
        let id = self.next_entity_id;
        let stamp = self.begin_common_body_attempt();
        let mut entity = native_instance_body(
            id,
            stamp,
            &metadata,
            terrain,
            NativeInstanceBodyRequest {
                entity_type: LEVEL_ONE_TYPE61_ENTITY_TYPE,
                position_raw: birth.position_raw,
                objective: false,
            },
        );
        let allocation = observe_main_base_abort_actor(&entity, self.allocation_generation).lease;
        publish_auto_pilot_power_up(
            AutoPilotPowerUpConstruction {
                entity: &mut entity,
                allocation,
                metadata: &metadata,
                birth,
                terrain,
                constructor_surface_bits,
            },
            world_fx,
        )?;
        assert_eq!(self.allocate_live_entity_id(), id);
        self.append_live_entity(entity);
        Ok(id)
    }
}
