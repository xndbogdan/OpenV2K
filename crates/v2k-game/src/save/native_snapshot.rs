//! `451FB0 -> 443260 / 42ECA0 -> 448E30` save checkpoint publication.
//! The route's logical world and the current body pose are independent: on
//! the campaign map the destination has changed before its player is born.

use super::{NativeCompatibilityPreview, SavedPlayerState};
use crate::{
    entity::CampaignCargoControllerState,
    entity_collision_state::RetailRuntimeValue,
    player::{PlayerCraft, VehicleMode},
    player_hull::PlayerHull,
    power_up_contact::{PlayerCampaignProgress, RETAIL_CONTROL_SLOT_COUNT},
    weapon_inventory::{PlayerCapabilities, WeaponInventory},
};
use v2k_formats::saves::STATE_PAYLOAD_SIZE;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeSaveSnapshotError {
    LogicalWorld(u32),
    CargoCapacity(u8),
    CargoSlots(usize),
    UnresolvedCargoStamp(usize),
    AuxiliarySlots(usize),
}

/// Inputs owned by the live session at the snapshot boundary. The caller
/// supplies the retained native payload so unknown controller/padding fields
/// survive a load-save round trip; this API never invents their values.
pub struct NativeSaveSnapshot<'a> {
    pub logical_level_id: u32,
    pub player: SavedPlayerState,
    pub craft: &'a PlayerCraft,
    pub hull: &'a PlayerHull,
    pub inventory: &'a WeaponInventory,
    pub capabilities: &'a PlayerCapabilities,
    pub cargo: &'a CampaignCargoControllerState,
    pub campaign: &'a PlayerCampaignProgress,
}

impl NativeSaveSnapshot<'_> {
    /// `42E270 ->448070` stages flags0xF and the route's arrival. After body
    /// teardown, `443440` applies those overrides: XYZ, zero velocity, heading
    /// 4000/zero attitude, and health40000. Fuel, mode and shield are retained.
    pub fn encode_campaign_arrival(
        &self,
        baseline: &[u8; STATE_PAYLOAD_SIZE],
        arrival: crate::campaign_transition::WarpArrival,
        display_name: &str,
    ) -> Result<NativeCompatibilityPreview, NativeSaveSnapshotError> {
        let mut craft = self.craft.clone();
        craft.restore_body_angle_words([0, 0]);
        let hull = self.hull.campaign_world_replacement(self.hull.profile());
        let snapshot = NativeSaveSnapshot {
            logical_level_id: self.logical_level_id,
            player: SavedPlayerState {
                position_raw: arrival.position_raw,
                velocity_raw: [0; 3],
                heading_raw: arrival.heading_raw,
                pitch_raw: 0,
                roll_raw: 0,
                health_raw: hull.health_raw,
            },
            craft: &craft,
            hull: &hull,
            inventory: self.inventory,
            capabilities: self.capabilities,
            cargo: self.cargo,
            campaign: self.campaign,
        };
        let mut native = snapshot.encode(baseline)?;
        // 451FB0 copies at most31 bytes and terminates the 32-byte name field.
        native.state_payload[..32].fill(0);
        let name = display_name.as_bytes();
        let count = name.len().min(31);
        native.state_payload[..count].copy_from_slice(&name[..count]);
        Ok(native)
    }

    pub fn encode(
        &self,
        baseline: &[u8; STATE_PAYLOAD_SIZE],
    ) -> Result<NativeCompatibilityPreview, NativeSaveSnapshotError> {
        if !(1..=36).contains(&self.logical_level_id) {
            return Err(NativeSaveSnapshotError::LogicalWorld(self.logical_level_id));
        }
        if self.cargo.unlock_raw > 8 {
            return Err(NativeSaveSnapshotError::CargoCapacity(
                self.cargo.unlock_raw,
            ));
        }
        if self.cargo.carried_identities().len() > 8 {
            return Err(NativeSaveSnapshotError::CargoSlots(
                self.cargo.carried_identities().len(),
            ));
        }
        if self.cargo.auxiliary_owned_types().len() > 16 {
            return Err(NativeSaveSnapshotError::AuxiliarySlots(
                self.cargo.auxiliary_owned_types().len(),
            ));
        }
        let mut payload = *baseline;
        write_word(&mut payload, 0x20, self.logical_level_id);
        for (offset, words) in [
            (0x24, self.player.position_raw),
            (0x2a, self.player.velocity_raw),
        ] {
            for (axis, word) in words.into_iter().enumerate() {
                payload[offset + axis * 2..offset + axis * 2 + 2]
                    .copy_from_slice(&word.to_le_bytes());
            }
        }
        let [pitch, roll] = self.craft.body_angle_words();
        for (offset, word) in [
            (0x30, self.player.heading_raw),
            (0x32, pitch as u16),
            (0x34, roll as u16),
        ] {
            payload[offset..offset + 2].copy_from_slice(&word.to_le_bytes());
        }
        payload[0x36] = match self.craft.mode {
            VehicleMode::Hover => 0,
            VehicleMode::Vtol => 1,
        };
        write_word(&mut payload, 0x38, self.craft.fuel_raw as u32);
        write_word(&mut payload, 0x3c, self.hull.health_raw as u32);
        self.inventory.encode_into_native_payload(&mut payload);
        write_word(&mut payload, 0x140, 46);
        payload[0x144] = self.campaign.extra_lives();
        payload[0x147] = (payload[0x147] & !3)
            | u8::from(self.capabilities.has_targetter())
            | (u8::from(self.capabilities.has_turbo()) << 1);
        payload[0x149] = self.cargo.unlock_raw;
        payload[0x14a] = self.campaign.trophy_count();
        payload[0x14c..0x16c].fill(0);
        for (index, identity) in self.cargo.carried_identities().iter().enumerate() {
            let RetailRuntimeValue::Known(word) = identity.packed_word() else {
                return Err(NativeSaveSnapshotError::UnresolvedCargoStamp(index));
            };
            write_word(&mut payload, 0x14c + index * 4, word);
        }
        write_word(
            &mut payload,
            0x16c,
            self.hull.pre_health_damage_buffer_raw as u32,
        );
        payload[0x170..0x1b0].fill(0);
        for (index, word) in self
            .cargo
            .auxiliary_owned_types()
            .iter()
            .copied()
            .enumerate()
        {
            write_word(&mut payload, 0x170 + index * 4, word);
        }
        for slot in 0..RETAIL_CONTROL_SLOT_COUNT {
            write_word(
                &mut payload,
                0x1b4 + slot * 4,
                self.campaign.control_slot_bits(slot).unwrap(),
            );
        }
        Ok(NativeCompatibilityPreview {
            logical_level_id: self.logical_level_id,
            state_payload: payload,
            saved_hint_mask: None,
        })
    }
}

fn write_word(payload: &mut [u8; STATE_PAYLOAD_SIZE], offset: usize, value: u32) {
    payload[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{player_hull::PLAYER_TYPE_46_HULL_PROFILE, save::NativeSaveRestore};

    #[test]
    fn snapshot_preserves_live_fields_and_campaign_arrival_overrides_only_route_state() {
        let mut baseline = [0xa5; STATE_PAYLOAD_SIZE];
        baseline[0x147] = 0xf0;
        let mut craft = PlayerCraft::new();
        craft.mode = VehicleMode::Vtol;
        craft.fuel_raw = 123456;
        craft.restore_body_angle_words([-123, 456]);
        let mut hull = PlayerHull::new(PLAYER_TYPE_46_HULL_PROFILE);
        hull.health_raw = 32109;
        hull.pre_health_damage_buffer_raw = 80833;
        let inventory = WeaponInventory::default();
        let capabilities = PlayerCapabilities::from_native_flags(3);
        let cargo = CampaignCargoControllerState::from_packed(3, [0x08010044, 0, 0xffff0049])
            .with_auxiliary_owned_types([49, 64]);
        let campaign = PlayerCampaignProgress::from_native_snapshot(3, [0x8000042f; 37], 4, 9);
        let snapshot = NativeSaveSnapshot {
            logical_level_id: 3,
            player: SavedPlayerState {
                position_raw: [0x2a00, -512, -13568],
                velocity_raw: [7, 8, 9],
                heading_raw: 0x4000,
                pitch_raw: 0,
                roll_raw: 0,
                health_raw: 1,
            },
            craft: &craft,
            hull: &hull,
            inventory: &inventory,
            capabilities: &capabilities,
            cargo: &cargo,
            campaign: &campaign,
        };
        let native = snapshot.encode(&baseline).unwrap();
        let restored = NativeSaveRestore::decode(&native).unwrap();
        assert_eq!(restored.logical_level_id, 3);
        assert_eq!(restored.player.position_raw, [0x2a00, -512, -13568]);
        assert_eq!(restored.player.health_raw, 32109);
        assert_eq!(restored.player.pitch_raw, (-123_i16) as u16);
        assert_eq!(restored.player.roll_raw, 456);
        assert_eq!(restored.fuel_raw, 123456);
        assert_eq!(restored.pre_health_damage_buffer_raw, 80833);
        assert_eq!(
            restored.cargo.carried_identities()[2].packed_word(),
            RetailRuntimeValue::Known(0xffff0049)
        );
        assert_eq!(restored.cargo.auxiliary_owned_types()[..3], [49, 64, 0]);
        assert_eq!(restored.campaign.control_slot_bits(36), Some(0x8000042f));
        for offset in [0x37, 0x145, 0x148, 0x14b, 0x1b0, 0x1b1, 0x1b2, 0x1b3] {
            assert_eq!(
                native.state_payload[offset], baseline[offset],
                "unowned byte {offset:x}"
            );
        }
        assert_eq!(native.state_payload[0x147], 0xf3);
        let route = crate::campaign_transition::CampaignWarpRoute {
            source_level_id: 14,
            destination_level_id: 15,
            destination_logical_level: 3,
            marker_cell: [0; 2],
            marker_kind: 22,
            marker_subtype: 1,
            marker_args: [0; 2],
            arrival: crate::campaign_transition::WarpArrival {
                position_raw: [0x2b00, 0x0a00, 0x6000],
                heading_raw: 0x4000,
            },
        };
        let checkpoint = snapshot
            .encode_campaign_arrival(&baseline, route.arrival, "World Three")
            .unwrap();
        let restored = NativeSaveRestore::decode(&checkpoint).unwrap();
        assert_eq!(restored.player.position_raw, route.arrival.position_raw);
        assert_eq!(restored.player.velocity_raw, [0; 3]);
        assert_eq!(
            (
                restored.player.heading_raw,
                restored.player.pitch_raw,
                restored.player.roll_raw
            ),
            (0x4000, 0, 0)
        );
        assert_eq!(restored.player.health_raw, 40_000);
        assert_eq!(restored.pre_health_damage_buffer_raw, 80_833);
        assert_eq!(restored.fuel_raw, 123456);
        assert_eq!(restored.mode, VehicleMode::Vtol);
        assert_eq!(&checkpoint.state_payload[..12], b"World Three\0");
        assert_eq!(
            craft.body_angle_words(),
            [-123, 456],
            "snapshot must not mutate the outgoing body"
        );
    }
}
