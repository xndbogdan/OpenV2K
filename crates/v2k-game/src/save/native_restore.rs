//! Native mode-6 restoration (`448E50 -> 443560`, then `451C00`).
//!
//! Keep the original payload alongside decoded owner inputs. Bytes copied by
//! retail but not yet owned by the port must not disappear at menu dispatch.

use super::{NativeCompatibilityPreview, SavedPlayerState};
use crate::entity::CampaignCargoControllerState;
use crate::player::{PlayerCraft, VehicleMode};
use crate::player_hull::PlayerHull;
use crate::power_up_contact::{PlayerCampaignProgress, RETAIL_CONTROL_SLOT_COUNT};
use crate::weapon_inventory::{PlayerCapabilities, WeaponInventory};
use v2k_formats::saves::STATE_PAYLOAD_SIZE;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeSaveRestoreError {
    UnsupportedLogicalWorld(u32),
    UnsupportedMovementStyle(u8),
    UnsupportedPlayerType(u32),
    InvalidPlayerHealth(i32),
    UnsupportedWeaponInventory,
    CargoProfileOverflow(u8),
}

/// Source-proven inputs to the native player-first world load. This is not a
/// serialization of the running world: retail reconstructs authored actors,
/// then restores cargo through their saved type/stamp identities.
#[derive(Debug, Clone)]
pub struct NativeSaveRestore {
    pub logical_level_id: u32,
    pub state_payload: [u8; STATE_PAYLOAD_SIZE],
    pub player: SavedPlayerState,
    pub mode: VehicleMode,
    pub fuel_raw: i32,
    pub inventory: WeaponInventory,
    pub capabilities: PlayerCapabilities,
    pub cargo: CampaignCargoControllerState,
    pub campaign: PlayerCampaignProgress,
    /// Session +1C4, applied by 44F830 ->450FC0 independently of progress.
    pub session_flags_raw: u32,
    /// 443560 writes saved-profile +148 to entity +50, the shield buffer.
    pub pre_health_damage_buffer_raw: i32,
    /// Controller +195, retained separately from lives/cargo/trophy bytes.
    pub controller_195_raw: u8,
}

impl NativeSaveRestore {
    pub fn decode(native: &NativeCompatibilityPreview) -> Result<Self, NativeSaveRestoreError> {
        let payload = &native.state_payload;
        let logical_level_id = word(payload, 0x20);
        if !(1..=36).contains(&logical_level_id) {
            return Err(NativeSaveRestoreError::UnsupportedLogicalWorld(
                logical_level_id,
            ));
        }
        let mode = match payload[0x36] {
            0 => VehicleMode::Hover,
            1 => VehicleMode::Vtol,
            style => return Err(NativeSaveRestoreError::UnsupportedMovementStyle(style)),
        };
        // 4438A0 constructs the controller's +190 type, rather than using a
        // guessed Section-13 player. This port owns the ordinary type46 body.
        if word(payload, 0x140) != 46 {
            return Err(NativeSaveRestoreError::UnsupportedPlayerType(word(
                payload, 0x140,
            )));
        }
        let player = SavedPlayerState {
            position_raw: std::array::from_fn(|axis| short(payload, 0x24 + axis * 2)),
            velocity_raw: std::array::from_fn(|axis| short(payload, 0x2a + axis * 2)),
            heading_raw: short(payload, 0x30) as u16,
            pitch_raw: short(payload, 0x32) as u16,
            roll_raw: short(payload, 0x34) as u16,
            health_raw: word(payload, 0x3c) as i32,
        };
        if player.health_raw <= 0 {
            return Err(NativeSaveRestoreError::InvalidPlayerHealth(
                player.health_raw,
            ));
        }
        let inventory = WeaponInventory::restore_from_native_payload(payload)
            .ok_or(NativeSaveRestoreError::UnsupportedWeaponInventory)?;
        // +19C..+1BB are eight complete dwords; +1BC is entity +50, and
        // +1C0 begins the distinct auxiliary-owned list. Never consume either
        // as extra conventional cargo when an invalid unlock byte overflows.
        if payload[0x149] > 8 {
            return Err(NativeSaveRestoreError::CargoProfileOverflow(payload[0x149]));
        }
        let cargo = CampaignCargoControllerState::from_packed(
            payload[0x149],
            (0..8).map(|index| word(payload, 0x14c + index * 4)),
        )
        .with_auxiliary_owned_types((0..16).map(|index| word(payload, 0x170 + index * 4)));
        let campaign = PlayerCampaignProgress::from_native_snapshot(
            logical_level_id as usize,
            std::array::from_fn::<_, RETAIL_CONTROL_SLOT_COUNT, _>(|index| {
                word(payload, 0x1b4 + index * 4)
            }),
            payload[0x144],
            payload[0x14a],
        );
        Ok(Self {
            logical_level_id,
            state_payload: *payload,
            player,
            mode,
            fuel_raw: word(payload, 0x38) as i32,
            inventory,
            capabilities: PlayerCapabilities::from_native_flags(payload[0x147]),
            cargo,
            campaign,
            session_flags_raw: word(payload, 0x1b0),
            pre_health_damage_buffer_raw: word(payload, 0x16c) as i32,
            controller_195_raw: payload[0x145],
        })
    }

    /// Apply persistent controller fields to the newly constructed component;
    /// its timers, animation joints, and constructor RNG state stay fresh.
    pub fn restore_craft(&self, craft: &mut PlayerCraft) {
        craft.mode = self.mode;
        craft.fuel_raw = self.fuel_raw;
        craft.restore_body_angle_words([self.player.pitch_raw as i16, self.player.roll_raw as i16]);
        craft.select_weapon_callback(self.inventory.selected_descriptor().callback_selector());
    }

    pub fn restore_hull(&self, hull: &mut PlayerHull) {
        hull.health_raw = self.player.health_raw;
        hull.pre_health_damage_buffer_raw = self.pre_health_damage_buffer_raw;
    }
}

fn word(payload: &[u8; STATE_PAYLOAD_SIZE], offset: usize) -> u32 {
    u32::from_le_bytes(payload[offset..offset + 4].try_into().unwrap())
}

fn short(payload: &[u8; STATE_PAYLOAD_SIZE], offset: usize) -> i16 {
    i16::from_le_bytes(payload[offset..offset + 2].try_into().unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn native() -> NativeCompatibilityPreview {
        let mut payload = [0; STATE_PAYLOAD_SIZE];
        payload[0x20..0x24].copy_from_slice(&18_u32.to_le_bytes());
        payload[0x3c..0x40].copy_from_slice(&39_000_i32.to_le_bytes());
        payload[0x40..0x44].copy_from_slice(&1_u32.to_le_bytes());
        payload[0x140..0x144].copy_from_slice(&46_u32.to_le_bytes());
        payload[0x149] = 1;
        NativeCompatibilityPreview {
            logical_level_id: 18,
            state_payload: payload,
        }
    }

    #[test]
    fn exact_controller_campaign_and_cargo_fields_survive_native_dispatch() {
        let mut native = native();
        let p = &mut native.state_payload;
        p[0x24..0x26].copy_from_slice(&(-32000_i16).to_le_bytes());
        p[0x2a..0x2c].copy_from_slice(&i16::MIN.to_le_bytes());
        p[0x32..0x34].copy_from_slice(&(-1234_i16).to_le_bytes());
        p[0x34..0x36].copy_from_slice(&2345_i16.to_le_bytes());
        p[0x36] = 1;
        p[0x38..0x3c].copy_from_slice(&199_123_i32.to_le_bytes());
        p[0x48..0x4c].copy_from_slice(&2_u32.to_le_bytes());
        p[0x4c..0x50].copy_from_slice(&123_u32.to_le_bytes());
        p[0x144] = 7;
        p[0x146] = 1;
        p[0x147] = 3;
        p[0x149] = 5;
        p[0x14a] = 19;
        p[0x14c..0x150].copy_from_slice(&0x0401_0044_u32.to_le_bytes());
        p[0x150..0x154].copy_from_slice(&0x8000_0000_u32.to_le_bytes());
        p[0x154..0x158].copy_from_slice(&0x0402_0044_u32.to_le_bytes());
        p[0x16c..0x170].copy_from_slice(&0x7654_3210_u32.to_le_bytes());
        p[0x170..0x174].copy_from_slice(&49_u32.to_le_bytes());
        p[0x1b0..0x1b4].copy_from_slice(&0x29_u32.to_le_bytes());
        for index in 0..37 {
            let offset = 0x1b4 + index * 4;
            p[offset..offset + 4].copy_from_slice(&(0x8000_0209_u32 + index as u32).to_le_bytes());
        }
        let restore = NativeSaveRestore::decode(&native).unwrap();
        assert_eq!(restore.state_payload, native.state_payload);
        assert_eq!(restore.player.position_raw[0], -32000);
        assert_eq!(restore.player.velocity_raw[0], i16::MIN);
        assert_eq!(restore.inventory.selected_slot(), 1);
        assert_eq!(
            restore
                .inventory
                .selected_descriptor()
                .stored_resource_count(),
            123
        );
        assert!(restore.capabilities.has_targetter());
        assert!(restore.capabilities.has_turbo());
        assert_eq!(restore.cargo.unlock_raw, 5);
        assert_eq!(restore.cargo.carried_identities()[1].entity_type(), 0);
        assert_eq!(restore.cargo.carried_identities()[2].entity_type(), 68);
        assert_eq!(restore.cargo.auxiliary_owned_types()[0], 49);
        assert_eq!(restore.pre_health_damage_buffer_raw, 0x7654_3210);
        assert_eq!(restore.session_flags_raw, 0x29);
        assert_eq!(restore.campaign.current_control_slot(), Some(18));
        assert_eq!(restore.campaign.extra_lives(), 7);
        assert_eq!(restore.campaign.trophy_count(), 19);
        for index in 0..37 {
            assert_eq!(
                restore.campaign.control_slot_bits(index),
                Some(0x8000_0209 + index as u32)
            );
        }
        let mut craft = PlayerCraft::new();
        restore.restore_craft(&mut craft);
        assert_eq!(craft.mode, VehicleMode::Vtol);
        assert_eq!(craft.fuel_raw, 199_123);
        assert_eq!(craft.body_angle_words(), [-1234, 2345]);
    }

    #[test]
    fn invalid_native_inventory_is_not_repaired_or_narrowed() {
        for (selector, selected) in [(0_u32, 0_u8), (0x101, 0), (1, 1), (1, 32)] {
            let mut native = native();
            native.state_payload[0x40..0x44].copy_from_slice(&selector.to_le_bytes());
            native.state_payload[0x146] = selected;
            assert_eq!(
                NativeSaveRestore::decode(&native).unwrap_err(),
                NativeSaveRestoreError::UnsupportedWeaponInventory
            );
        }
    }

    #[test]
    fn first_zero_weapon_terminates_without_consuming_stale_following_rows() {
        let mut native = native();
        native.state_payload[0x50..0x54].copy_from_slice(&0xffff_ffff_u32.to_le_bytes());
        let restore = NativeSaveRestore::decode(&native).unwrap();
        assert_eq!(restore.inventory.occupied_slot_count(), 1);
    }

    #[test]
    fn unsupported_controller_modes_and_overflowing_cargo_remain_explicit() {
        let mut native = native();
        native.state_payload[0x36] = 2;
        assert_eq!(
            NativeSaveRestore::decode(&native).unwrap_err(),
            NativeSaveRestoreError::UnsupportedMovementStyle(2)
        );
        native.state_payload[0x36] = 0;
        native.state_payload[0x149] = 9;
        assert_eq!(
            NativeSaveRestore::decode(&native).unwrap_err(),
            NativeSaveRestoreError::CargoProfileOverflow(9)
        );
        native.state_payload[0x149] = 1;
        native.state_payload[0x140..0x144].copy_from_slice(&51_u32.to_le_bytes());
        assert_eq!(
            NativeSaveRestore::decode(&native).unwrap_err(),
            NativeSaveRestoreError::UnsupportedPlayerType(51)
        );
    }
}
