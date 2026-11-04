//! The attract demo's player: `FUN_00451AF0` in phase 4.
//!
//! `FUN_004438A0` builds the controller's body through `FUN_00438080` with
//! the phase-4 flag, whose request carries type 51's behavior list at
//! `+0x48` (class 7, the network peer craft's) instead of type 46's (class
//! 24, Player Control). The request's pose is the controller's (the record
//! at `DAT_004D0B30` after boot); `FUN_00443560` then moves the body to the
//! session record, whose position the attract row wrote. It is the world's
//! first body, before the gate helpers and the authored records.

use super::*;
use crate::intro2_flyer_aim::CraftEmitterArming;
use crate::intro2_flyers_live::{
    flyer_runtime_metadata, publish_attract_craft, Intro2FlyerSchedulerOwner, TYPE46_ENTITY_TYPE,
};

/// What `FUN_00451AF0` needs from the session for the phase-4 craft.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttractCraftArrival {
    /// Session `+0x38`: the row's arrival, lifted by `FUN_00451C00`.
    pub position_raw: [i16; 3],
    /// Controller `+0x20C`, the Self Righting word.
    pub self_righting: u8,
    /// Type 46's Sub-E record as an earlier demo's `FUN_004503C0` left it.
    pub sub_e_record: Option<CraftEmitterArming>,
}

/// A phase-4 world: the craft, when its row has a player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttractWorld {
    pub craft: Option<AttractCraftArrival>,
}

impl EntityManager {
    /// The world was built in phase 4 (`FUN_00456F10() == 4`).
    pub fn session_phase_4(&self) -> bool {
        self.session_phase_4
    }

    /// Phase 4's player: the craft, while it lives.
    pub fn attract_craft_id(&self) -> Option<u32> {
        if !self.session_phase_4 {
            return None;
        }
        self.entities
            .iter()
            .find(|entity| {
                entity.active
                    && entity.entity_type == TYPE46_ENTITY_TYPE
                    && entity.intro2_flyer_frame_owner.is_some()
            })
            .map(|entity| entity.id)
    }

    /// `FUN_004438A0(controller, 1)` then `FUN_00443560`. Returns the body.
    pub(super) fn append_attract_craft(
        &mut self,
        arrival: AttractCraftArrival,
        resources: EntityConstructionResources<'_>,
        world_fx: &mut WorldFx,
    ) -> Result<u32, String> {
        let metadata = flyer_runtime_metadata(self, TYPE46_ENTITY_TYPE)
            .ok_or("missing type 46/51 metadata")?;
        let terrain = resources.terrain.ok_or("missing constructor terrain")?;
        let id = self.next_entity_id;
        let controller = crate::campaign_transition::RETAIL_CONTROLLER_DEFAULT_ARRIVAL;
        let mut entity = native_instance_body(
            id,
            RetailRuntimeValue::Unresolved,
            &metadata,
            terrain,
            NativeInstanceBodyRequest {
                entity_type: TYPE46_ENTITY_TYPE,
                position_raw: controller.position_raw,
                objective: false,
            },
        );
        // Not the port's player: Player Control and its keys are phase 5's.
        entity.kind = EntityKind::Unknown(TYPE46_ENTITY_TYPE);
        // 104B0's +44 for type 46.
        entity.collision.pair_callbacks.damage_modifier_address =
            RetailRuntimeValue::Known(Some(0x0044_84A0));
        entity.set_rotation_heading_pitch_roll_raw([controller.heading_raw as i16, 0, 0]);
        entity.apply_d720_euler_body_basis();
        entity.mass_raw = metadata.mass_raw;
        let allocation = observe_main_base_abort_actor(&entity, self.allocation_generation).lease;
        // The craft takes the player's ordinal (451710).
        entity.construction_stamp_at_0xb4 = self.begin_common_body_attempt();
        let owner: Intro2FlyerSchedulerOwner = publish_attract_craft(
            &mut entity,
            allocation,
            &metadata,
            arrival.self_righting,
            INITIAL_PLAYER_CARGO_CAPACITY as u8,
            world_fx,
        )
        .map_err(|reason| format!("attract craft: {reason:?}"))?;
        // FUN_00443560: the boot record's pose, velocity zero, at the row's
        // arrival, then FUN_00413F70's basis.
        entity.set_motion_raw(arrival.position_raw, [0; 3]);
        entity.set_rotation_heading_pitch_roll_raw([controller.heading_raw as i16, 0, 0]);
        entity.apply_d720_euler_body_basis();
        if let (Some(record), Some(runtime)) = (
            arrival.sub_e_record,
            entity.intro2_flyer_aim_runtime.as_mut(),
        ) {
            runtime.restore_craft_record(record);
        }
        assert_eq!(self.allocate_live_entity_id(), id);
        self.append_live_entity(entity);
        let _ = owner;
        Ok(id)
    }

    /// `FUN_004503C0`'s per-visit writes to the craft's Sub-E runtime and
    /// type 46's shared Sub-E record.
    pub fn arm_attract_craft(&mut self, id: u32, arming: CraftEmitterArming) -> bool {
        let Some(runtime) = self
            .entity_mut(id)
            .and_then(|entity| entity.intro2_flyer_aim_runtime.as_mut())
        else {
            return false;
        };
        runtime.arm_craft(arming);
        true
    }
}
