//! Read-only retail player-fire state inspection.
//!
//! The primary weapon does not add a normal `0xCC` entity to the global
//! entity list. `FUN_004147A0` allocates a transient `0x2C` command and
//! appends it to an intrusive list rooted at the resolved shooter entity:
//!
//! - entity `+0x38` contains the first node, or the address of entity `+0x3C`
//!   when the list is empty;
//! - entity `+0x3C` is the zero end sentinel;
//! - entity `+0x40` contains the tail node, or the address of entity `+0x38`
//!   when the list is empty.
//!
//! `FUN_00411400` consumes and frees these commands during an entity update,
//! so every read is bracketed by both the 50 Hz clock and the list roots.

use std::collections::HashSet;

use serde::Serialize;

use crate::process::{i16_at, plausible_heap_pointer, u16_at, u32_at, Process};

const TICK_50HZ: usize = 0x004F_ED60;
const ENTITY_HEAD_PTR: usize = 0x004D_B090;
const ENTITY_END_SENTINEL: usize = 0x004D_B094;
const ENTITY_TAIL_PTR: usize = 0x004D_B098;
const PLAYER_ENTITY_TYPE: u32 = 46;
const ENTITY_PREFIX_BYTES: usize = 0x60;
const ENTITY_POSE_OFFSET: usize = 0x96;
const ENTITY_POSE_BYTES: usize = 0x12;
const MAX_ENTITY_NODES: usize = 4096;

const SHOT_HEAD_OFFSET: usize = 0x38;
const SHOT_END_OFFSET: usize = 0x3C;
const SHOT_TAIL_OFFSET: usize = 0x40;
const SHOT_RECORD_BYTES: usize = 0x2C;
const MAX_SHOT_RECORDS: usize = 128;

const CONTROLLER_WEAPON_DESCRIPTORS_OFFSET: usize = 0x2A8;
const WEAPON_DESCRIPTOR_BYTES: usize = 0x18;
const WEAPON_DESCRIPTOR_COUNT: usize = 32;
const CONTROLLER_SELECTED_SLOT_OFFSET: usize = 0x5A8;
const WEAPON_DESCRIPTOR_BLOCK_BYTES: usize = WEAPON_DESCRIPTOR_BYTES * WEAPON_DESCRIPTOR_COUNT;
const MASTER_WEAPON_TABLE: usize = 0x004C_DC08;
const MASTER_WEAPON_DESCRIPTOR_COUNT: usize = 24;
const MASTER_WEAPON_TABLE_BYTES: usize = WEAPON_DESCRIPTOR_BYTES * MASTER_WEAPON_DESCRIPTOR_COUNT;

const CONTROLLER_PICKUP_STATE_OFFSET: usize = 0x194;
const CONTROLLER_PICKUP_STATE_BYTES: usize = 0xEC;
const CONTROLLER_ATTACHMENT_LIST_OFFSET: usize = 0x220;
const CONTROLLER_TARGETTER_OFFSET: usize = 0x22C;
const TARGETTER_BYTES: usize = 0x54;
const TARGETTER_CALLBACK_TABLE_POINTER_OFFSET: usize = 0x44;
const TARGETTER_CALLBACK_TABLE_BYTES: usize = 0x14;

const PLAYER_COMPONENT_ROOT_OFFSET: usize = 0x4C;
const COMPONENT_STATE_OFFSET: usize = 0x0C;
const COMPONENT_WEAPON_STATE_OFFSET: usize = 0x2C;
const RUNTIME_WEAPON_STATE_BYTES: usize = 0x3C;

/// One transient `FUN_004147A0` command from the shooter's `+0x38` list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ShotRecord {
    pub pointer: u32,
    pub next: u32,
    /// Previous node, or the address of entity `+0x38` for the first node.
    pub previous_link: u32,
    pub projectile_or_effect_selector: u32,
    pub selector_variant: u16,
    pub origin_raw_8_8: [i16; 3],
    pub direction_raw: [i32; 3],
    /// Shifted right by five before the direction/speed position advance.
    pub travel_or_time_scalar: i32,
    pub source_reference: u32,
    /// Filled through `FUN_0044EA50(selector)` when authored as zero.
    pub speed_or_scale: i16,
    /// Set to one or two by `FUN_004147A0`.
    pub status: u16,
    pub raw_hex: String,
}

/// A clock- and topology-bracketed view of the player's transient shot list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ShotListSnapshot {
    pub tick_before: u32,
    pub tick_after: u32,
    pub tick_stable: bool,
    pub head_link_address: u32,
    pub end_sentinel_address: u32,
    pub head_before: u32,
    pub head_after: u32,
    pub tail_before: u32,
    pub tail_after: u32,
    pub sentinel_value_before: u32,
    pub sentinel_value_after: u32,
    pub topology_stable: bool,
    pub traversal_complete: bool,
    pub records: Vec<ShotRecord>,
    pub warnings: Vec<String>,
}

/// One of the controller's 32 exact 0x18-byte weapon descriptors.
///
/// Only offsets directly observed in the retail accessors are split out. The
/// neutral field names deliberately avoid assigning unproven gameplay names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WeaponDescriptor {
    pub index: usize,
    pub field_00: u32,
    pub field_04: u32,
    pub byte_08: u8,
    pub byte_09: u8,
    pub byte_0a: u8,
    pub byte_0b: u8,
    pub byte_0c: u8,
    pub byte_0d: u8,
    pub field_0e: u16,
    pub field_10: u32,
    pub field_14: u32,
    pub raw_hex: String,
}

/// Player controller state reached through
/// `*(0x004F72C8) + 0x27C -> controller`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ControllerWeaponState {
    pub session_pointer: u32,
    pub controller_pointer: u32,
    pub entity_handle: Option<u32>,
    pub selected_slot: Option<u32>,
    pub selected_slot_valid: bool,
    pub descriptors: Vec<WeaponDescriptor>,
    pub descriptor_block_raw_hex: Option<String>,
    /// FNV-1a over the exact 32 × 0x18 descriptor bytes.
    pub descriptor_block_hash: Option<u64>,
    /// Acquisition/capability and Targetter state retained beside the weapon
    /// descriptors so a type-61 pickup can be tied to its recipient mutation.
    pub pickup_state: Option<ControllerPickupState>,
}

/// Exact controller state mutated by non-weapon Power Ups in
/// `FUN_00445A90`, plus the independently initialized Targetter block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ControllerPickupState {
    pub window_offset: u32,
    pub window_raw_hex: String,
    pub window_hash: u64,
    pub byte_at_0x194: u8,
    pub byte_at_0x195: u8,
    /// Active sub-model/weapon slot; retained under the established static
    /// interpretation without treating it as the selected descriptor at
    /// controller `+0x5A8` or runtime weapon `+0x14`.
    pub active_sub_model_or_weapon_slot_at_0x196: u8,
    pub capability_flags_at_0x197: u8,
    pub targetter_acquired: bool,
    pub turbo_acquired: bool,
    pub byte_at_0x198: u8,
    pub cargo_capacity_at_0x199: u8,
    pub byte_at_0x19a: u8,
    pub byte_at_0x19b: u8,
    pub attachment_list_links_at_0x220: [u32; 3],
    pub targetter: TargetterState,
}

/// The exact 0x54-byte structure installed for pickup selector 0x3C.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TargetterState {
    pub base_at_controller_0x22c: u32,
    pub raw_hex: String,
    pub raw_hash: u64,
    pub owner_handle_at_0x00: u32,
    pub prior_target_handle_at_0x04: u32,
    pub current_target_handle_at_0x0c: u32,
    pub ray_distance_or_parameter_at_0x14: u32,
    pub field_at_0x24: u32,
    pub field_at_0x28: u32,
    pub result_bits_at_0x34: u32,
    pub scan_radius_at_0x38: u32,
    pub callback_table_pointer_at_0x44: u32,
    pub callback_table_raw_hex: Option<String>,
    pub callback_table_hash: Option<u64>,
    pub callback_table_read_error: Option<String>,
}

/// Fixed executable master records copied into the controller when a new
/// weapon selector below 0x32 is accepted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MasterWeaponCatalog {
    pub address: u32,
    pub descriptor_count: usize,
    pub raw_hex: String,
    pub raw_hash: u64,
    pub descriptors: Vec<WeaponDescriptor>,
}

/// Runtime weapon state reached from the player's component chain:
/// entity `+0x4C -> +0x0C -> +0x2C`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuntimeWeaponState {
    pub component_root_pointer: u32,
    pub component_state_pointer: u32,
    pub weapon_state_pointer: u32,
    pub joint_pointer_at_00: u32,
    pub joint_value_at_00: Option<u16>,
    pub alternate_joint_pointer_at_04: u32,
    pub alternate_joint_value_at_04: Option<u16>,
    pub barrel_joint_pointer_at_08: u32,
    pub barrel_joint_angle_raw: Option<u16>,
    pub selector_output_pointer_at_0c: u32,
    pub selector_output_value_at_0c: Option<u16>,
    /// Compatibility alias for captures produced before the selector pointer
    /// at `+0x0C` was safely dereferenced.
    pub field_0c: u32,
    pub field_10: u32,
    pub selected_slot_at_14: u32,
    pub field_18: u32,
    pub field_1c: u32,
    pub field_20: u32,
    /// Zero selects target-leading aim; nonzero uses the entity/body basis.
    pub free_fire_mode_at_24: u32,
    pub trigger_accumulator_at_28: i32,
    /// Compatibility alias retained for existing capture consumers.
    pub accumulator_at_28: i32,
    pub cadence_at_2c: i32,
    pub resource_count_at_30: i32,
    /// Compatibility alias for the formerly unnamed `+0x30` field.
    pub interval_at_30: i32,
    pub cooldown_at_34: i32,
    pub aim_adjustment_pointer_at_38: u32,
    pub raw_hex: String,
    /// FNV-1a over the exact bounded runtime-state bytes.
    pub raw_hash: u64,
}

/// Raw player transform captured beside each transient shot observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlayerFirePose {
    pub position_raw_8_8: [i16; 3],
    /// Unsigned torus coordinates for X/Z. This keeps values at or beyond cell
    /// 128 readable without changing the exact signed memory representation
    /// above (for example raw Z `33829`, rather than `-31707`).
    pub position_wrapped_xz_raw_8_8: [u16; 2],
    pub velocity_raw: [i16; 3],
    /// Entity `+0xA2/+0xA4/+0xA6` (yaw, pitch, roll) in the 16-bit turn domain.
    pub euler_raw: [u16; 3],
}

/// Complete evidence for one high-frequency player-fire sample.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FireStateSnapshot {
    pub tick_before: u32,
    pub tick_after: u32,
    pub tick_stable: bool,
    pub player_pointer: u32,
    pub player_handle: u32,
    pub player_pose: PlayerFirePose,
    pub entity_head_before: u32,
    pub entity_head_after: u32,
    pub entity_tail_before: u32,
    pub entity_tail_after: u32,
    pub entity_topology_stable: bool,
    pub shots: ShotListSnapshot,
    pub controller: Option<ControllerWeaponState>,
    pub runtime_weapon: Option<RuntimeWeaponState>,
    pub warnings: Vec<String>,
}

/// A compact change detector suitable for high-frequency JSONL capture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FireStateSignature {
    pub shot_head_before: u32,
    pub shot_head_after: u32,
    pub shot_tail_before: u32,
    pub shot_tail_after: u32,
    pub shot_traversal_complete: bool,
    pub shot_records: Vec<ShotRecordSignature>,
    pub selected_slot: Option<u32>,
    pub selected_descriptor_raw_hex: Option<String>,
    pub descriptor_block_hash: Option<u64>,
    pub controller_pickup_state_hash: Option<u64>,
    pub targetter_callback_table_hash: Option<u64>,
    pub runtime_weapon_state_pointer: Option<u32>,
    pub runtime_weapon_state_hash: Option<u64>,
    pub barrel_joint_angle_raw: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ShotRecordSignature {
    pub pointer: u32,
    pub raw_hex: String,
}

impl FireStateSnapshot {
    pub fn signature(&self) -> FireStateSignature {
        let selected_slot = self
            .controller
            .as_ref()
            .and_then(|controller| controller.selected_slot);
        let selected_descriptor_raw_hex = self.controller.as_ref().and_then(|controller| {
            usize::try_from(selected_slot?).ok().and_then(|slot| {
                controller
                    .descriptors
                    .get(slot)
                    .map(|descriptor| descriptor.raw_hex.clone())
            })
        });
        FireStateSignature {
            shot_head_before: self.shots.head_before,
            shot_head_after: self.shots.head_after,
            shot_tail_before: self.shots.tail_before,
            shot_tail_after: self.shots.tail_after,
            shot_traversal_complete: self.shots.traversal_complete,
            shot_records: self
                .shots
                .records
                .iter()
                .map(|record| ShotRecordSignature {
                    pointer: record.pointer,
                    raw_hex: record.raw_hex.clone(),
                })
                .collect(),
            selected_slot,
            selected_descriptor_raw_hex,
            descriptor_block_hash: self
                .controller
                .as_ref()
                .and_then(|controller| controller.descriptor_block_hash),
            controller_pickup_state_hash: self
                .controller
                .as_ref()
                .and_then(|controller| controller.pickup_state.as_ref())
                .map(|state| state.window_hash),
            targetter_callback_table_hash: self
                .controller
                .as_ref()
                .and_then(|controller| controller.pickup_state.as_ref())
                .and_then(|state| state.targetter.callback_table_hash),
            runtime_weapon_state_pointer: self
                .runtime_weapon
                .as_ref()
                .map(|state| state.weapon_state_pointer),
            runtime_weapon_state_hash: self.runtime_weapon.as_ref().map(|state| state.raw_hash),
            barrel_joint_angle_raw: self
                .runtime_weapon
                .as_ref()
                .and_then(|state| state.barrel_joint_angle_raw),
        }
    }
}

struct PlayerLocation {
    pointer: u32,
    handle: u32,
    pose: PlayerFirePose,
    tick_before: u32,
    tick_after: u32,
    head_before: u32,
    head_after: u32,
    tail_before: u32,
    tail_after: u32,
    warnings: Vec<String>,
}

/// Read the 24 canonical descriptors that retail copies into empty controller
/// slots when a weapon Power Up is accepted.
pub fn read_master_weapon_catalog(process: &Process) -> Result<MasterWeaponCatalog, String> {
    let bytes = process
        .read_bytes(MASTER_WEAPON_TABLE, MASTER_WEAPON_TABLE_BYTES)
        .map_err(|error| {
            format!("could not read master weapon table at {MASTER_WEAPON_TABLE:08X}: {error}")
        })?;
    Ok(MasterWeaponCatalog {
        address: MASTER_WEAPON_TABLE as u32,
        descriptor_count: MASTER_WEAPON_DESCRIPTOR_COUNT,
        raw_hex: encode_hex(&bytes),
        raw_hash: fnv1a64(&bytes),
        descriptors: parse_weapon_descriptors(&bytes, MASTER_WEAPON_DESCRIPTOR_COUNT),
    })
}

/// Read one bounded, externally consistent view of the live player fire state.
pub fn read_snapshot(process: &Process) -> Result<FireStateSnapshot, String> {
    let tick_before = process.read_u32(TICK_50HZ)?;
    let player = locate_player(process)?;
    let shots = read_shot_list(process, player.pointer)?;
    let mut warnings = player.warnings;

    if player.tick_before != player.tick_after {
        warnings.push(format!(
            "50 Hz tick changed while locating player: {} -> {}",
            player.tick_before, player.tick_after
        ));
    }
    let entity_topology_stable =
        player.head_before == player.head_after && player.tail_before == player.tail_after;
    if !entity_topology_stable {
        warnings.push(format!(
            "entity topology changed while locating player: head {:08X}->{:08X}, tail {:08X}->{:08X}",
            player.head_before, player.head_after, player.tail_before, player.tail_after
        ));
    }

    let controller = read_controller_weapon_state(process, player.handle, &mut warnings);
    let runtime_weapon = read_runtime_weapon_state(process, player.pointer, &mut warnings);
    let tick_after = process.read_u32(TICK_50HZ)?;

    Ok(FireStateSnapshot {
        tick_before,
        tick_after,
        tick_stable: tick_before == tick_after,
        player_pointer: player.pointer,
        player_handle: player.handle,
        player_pose: player.pose,
        entity_head_before: player.head_before,
        entity_head_after: player.head_after,
        entity_tail_before: player.tail_before,
        entity_tail_after: player.tail_after,
        entity_topology_stable,
        shots,
        controller,
        runtime_weapon,
        warnings,
    })
}

fn locate_player(process: &Process) -> Result<PlayerLocation, String> {
    let tick_before = process.read_u32(TICK_50HZ)?;
    let head_before = process.read_u32(ENTITY_HEAD_PTR)?;
    let tail_before = process.read_u32(ENTITY_TAIL_PTR)?;
    let mut current = head_before;
    let mut visited = HashSet::new();
    let mut warnings = Vec::new();
    let mut found = None;

    for _ in 0..MAX_ENTITY_NODES {
        if current == 0 || current as usize == ENTITY_END_SENTINEL {
            break;
        }
        if !plausible_heap_pointer(current as usize) {
            return Err(format!(
                "implausible entity pointer {current:08X} while locating type {PLAYER_ENTITY_TYPE}"
            ));
        }
        if !visited.insert(current) {
            return Err(format!(
                "entity-list cycle at {current:08X} while locating type {PLAYER_ENTITY_TYPE}"
            ));
        }
        let bytes = process.read_bytes(current as usize, ENTITY_PREFIX_BYTES)?;
        let next = u32_at(&bytes, 0x00);
        if u32_at(&bytes, 0x58) == PLAYER_ENTITY_TYPE {
            found = Some((current, u32_at(&bytes, 0x5C)));
            break;
        }
        current = next;
    }
    if visited.len() == MAX_ENTITY_NODES {
        warnings.push(format!(
            "entity search reached its {MAX_ENTITY_NODES}-node safety bound"
        ));
    }

    let head_after = process.read_u32(ENTITY_HEAD_PTR)?;
    let tail_after = process.read_u32(ENTITY_TAIL_PTR)?;
    let tick_after = process.read_u32(TICK_50HZ)?;
    let (pointer, handle) = found.ok_or_else(|| {
        format!(
            "no type-{PLAYER_ENTITY_TYPE} player in the retail entity list (head {head_before:08X})"
        )
    })?;
    let pose_bytes =
        process.read_bytes(pointer as usize + ENTITY_POSE_OFFSET, ENTITY_POSE_BYTES)?;
    let pose = parse_player_pose(&pose_bytes);

    Ok(PlayerLocation {
        pointer,
        handle,
        pose,
        tick_before,
        tick_after,
        head_before,
        head_after,
        tail_before,
        tail_after,
        warnings,
    })
}

fn parse_player_pose(bytes: &[u8]) -> PlayerFirePose {
    debug_assert!(bytes.len() >= ENTITY_POSE_BYTES);
    PlayerFirePose {
        position_raw_8_8: [
            i16_at(bytes, 0x00),
            i16_at(bytes, 0x02),
            i16_at(bytes, 0x04),
        ],
        position_wrapped_xz_raw_8_8: [u16_at(bytes, 0x00), u16_at(bytes, 0x04)],
        velocity_raw: [
            i16_at(bytes, 0x06),
            i16_at(bytes, 0x08),
            i16_at(bytes, 0x0A),
        ],
        euler_raw: [
            u16_at(bytes, 0x0C),
            u16_at(bytes, 0x0E),
            u16_at(bytes, 0x10),
        ],
    }
}

fn read_shot_list(process: &Process, player_pointer: u32) -> Result<ShotListSnapshot, String> {
    let player = player_pointer as usize;
    let head_link_address = address_u32(player, SHOT_HEAD_OFFSET)?;
    let end_sentinel_address = address_u32(player, SHOT_END_OFFSET)?;
    let tick_before = process.read_u32(TICK_50HZ)?;
    let head_before = process.read_u32(player + SHOT_HEAD_OFFSET)?;
    let sentinel_value_before = process.read_u32(player + SHOT_END_OFFSET)?;
    let tail_before = process.read_u32(player + SHOT_TAIL_OFFSET)?;

    let mut records = Vec::new();
    let mut warnings = Vec::new();
    let mut visited = HashSet::new();
    let mut current = head_before;
    let mut traversal_complete = false;

    while records.len() < MAX_SHOT_RECORDS {
        if current == end_sentinel_address {
            traversal_complete = true;
            break;
        }
        if current == 0 {
            warnings.push(format!(
                "shot list reached null before end sentinel {end_sentinel_address:08X}"
            ));
            break;
        }
        if !plausible_heap_pointer(current as usize) {
            warnings.push(format!(
                "shot list contains implausible node pointer {current:08X}"
            ));
            break;
        }
        if !visited.insert(current) {
            warnings.push(format!("shot-list cycle returned to {current:08X}"));
            break;
        }
        match process.read_bytes(current as usize, SHOT_RECORD_BYTES) {
            Ok(bytes) => {
                let record = parse_shot_record(current, &bytes);
                current = record.next;
                records.push(record);
            }
            Err(error) => {
                warnings.push(format!("could not read shot node {current:08X}: {error}"));
                break;
            }
        }
    }
    if records.len() == MAX_SHOT_RECORDS && !traversal_complete {
        warnings.push(format!(
            "shot list reached its {MAX_SHOT_RECORDS}-record safety bound"
        ));
    }

    let head_after = process.read_u32(player + SHOT_HEAD_OFFSET)?;
    let sentinel_value_after = process.read_u32(player + SHOT_END_OFFSET)?;
    let tail_after = process.read_u32(player + SHOT_TAIL_OFFSET)?;
    let tick_after = process.read_u32(TICK_50HZ)?;
    let topology_stable = tick_before == tick_after
        && head_before == head_after
        && tail_before == tail_after
        && sentinel_value_before == sentinel_value_after;

    if sentinel_value_before != 0 || sentinel_value_after != 0 {
        warnings.push(format!(
            "shot end sentinel {end_sentinel_address:08X} was not zero ({sentinel_value_before:08X}->{sentinel_value_after:08X})"
        ));
    }
    if !topology_stable {
        warnings.push(format!(
            "shot list changed during read: tick {tick_before}->{tick_after}, head {head_before:08X}->{head_after:08X}, tail {tail_before:08X}->{tail_after:08X}"
        ));
    }
    validate_shot_links(
        head_link_address,
        end_sentinel_address,
        head_before,
        tail_before,
        &records,
        traversal_complete,
        &mut warnings,
    );

    Ok(ShotListSnapshot {
        tick_before,
        tick_after,
        tick_stable: tick_before == tick_after,
        head_link_address,
        end_sentinel_address,
        head_before,
        head_after,
        tail_before,
        tail_after,
        sentinel_value_before,
        sentinel_value_after,
        topology_stable,
        traversal_complete,
        records,
        warnings,
    })
}

fn read_controller_weapon_state(
    process: &Process,
    player_handle: u32,
    warnings: &mut Vec<String>,
) -> Option<ControllerWeaponState> {
    let location = crate::controller::locate_controller(process, Some(player_handle), warnings)?;
    let session_pointer = location.session_pointer;
    let controller_pointer = location.controller_pointer;
    let entity_handle = location.entity_handle_at_0x68;

    let block = match process.read_bytes(
        controller_pointer as usize + CONTROLLER_WEAPON_DESCRIPTORS_OFFSET,
        WEAPON_DESCRIPTOR_BLOCK_BYTES + 4,
    ) {
        Ok(bytes) => Some(bytes),
        Err(error) => {
            warnings.push(format!(
                "could not read controller weapon descriptors at {controller_pointer:08X}+0x2A8: {error}"
            ));
            None
        }
    };
    let (descriptors, descriptor_block_raw_hex, descriptor_block_hash, selected_slot) = block
        .as_deref()
        .map_or((Vec::new(), None, None, None), |bytes| {
            let descriptor_bytes = &bytes[..WEAPON_DESCRIPTOR_BLOCK_BYTES];
            (
                parse_weapon_descriptors(descriptor_bytes, WEAPON_DESCRIPTOR_COUNT),
                Some(encode_hex(descriptor_bytes)),
                Some(fnv1a64(descriptor_bytes)),
                Some(u32_at(
                    bytes,
                    CONTROLLER_SELECTED_SLOT_OFFSET - CONTROLLER_WEAPON_DESCRIPTORS_OFFSET,
                )),
            )
        });
    let selected_slot_valid =
        selected_slot.is_some_and(|slot| slot < WEAPON_DESCRIPTOR_COUNT as u32);
    if let Some(slot) = selected_slot.filter(|slot| *slot >= WEAPON_DESCRIPTOR_COUNT as u32) {
        warnings.push(format!(
            "controller selected weapon slot {slot} exceeds the 32-entry descriptor table"
        ));
    }
    let pickup_state = read_controller_pickup_state(process, controller_pointer, warnings);

    Some(ControllerWeaponState {
        session_pointer,
        controller_pointer,
        entity_handle,
        selected_slot,
        selected_slot_valid,
        descriptors,
        descriptor_block_raw_hex,
        descriptor_block_hash,
        pickup_state,
    })
}

fn read_controller_pickup_state(
    process: &Process,
    controller_pointer: u32,
    warnings: &mut Vec<String>,
) -> Option<ControllerPickupState> {
    let window_address = controller_pointer as usize + CONTROLLER_PICKUP_STATE_OFFSET;
    let bytes = match process.read_bytes(window_address, CONTROLLER_PICKUP_STATE_BYTES) {
        Ok(bytes) => bytes,
        Err(error) => {
            warnings.push(format!(
                "could not read controller pickup/Targetter state at {controller_pointer:08X}+0x194: {error}"
            ));
            return None;
        }
    };
    let relative = |absolute_offset: usize| absolute_offset - CONTROLLER_PICKUP_STATE_OFFSET;
    let targetter_offset = relative(CONTROLLER_TARGETTER_OFFSET);
    let targetter_bytes = &bytes[targetter_offset..targetter_offset + TARGETTER_BYTES];
    let callback_table_pointer = u32_at(targetter_bytes, TARGETTER_CALLBACK_TABLE_POINTER_OFFSET);
    let (callback_table_raw_hex, callback_table_hash, callback_table_read_error) =
        if callback_table_pointer == 0 {
            (None, None, None)
        } else if plausible_heap_pointer(callback_table_pointer as usize) {
            match process.read_bytes(
                callback_table_pointer as usize,
                TARGETTER_CALLBACK_TABLE_BYTES,
            ) {
                Ok(callback_table) => (
                    Some(encode_hex(&callback_table)),
                    Some(fnv1a64(&callback_table)),
                    None,
                ),
                Err(error) => (None, None, Some(error)),
            }
        } else {
            (
                None,
                None,
                Some(format!(
                    "callback table pointer {callback_table_pointer:08X} is implausible"
                )),
            )
        };
    let capability_flags = bytes[relative(0x197)];
    let attachment_offset = relative(CONTROLLER_ATTACHMENT_LIST_OFFSET);

    Some(ControllerPickupState {
        window_offset: CONTROLLER_PICKUP_STATE_OFFSET as u32,
        window_raw_hex: encode_hex(&bytes),
        window_hash: fnv1a64(&bytes),
        byte_at_0x194: bytes[relative(0x194)],
        byte_at_0x195: bytes[relative(0x195)],
        active_sub_model_or_weapon_slot_at_0x196: bytes[relative(0x196)],
        capability_flags_at_0x197: capability_flags,
        targetter_acquired: capability_flags & 1 != 0,
        turbo_acquired: capability_flags & 2 != 0,
        byte_at_0x198: bytes[relative(0x198)],
        cargo_capacity_at_0x199: bytes[relative(0x199)],
        byte_at_0x19a: bytes[relative(0x19A)],
        byte_at_0x19b: bytes[relative(0x19B)],
        attachment_list_links_at_0x220: [
            u32_at(&bytes, attachment_offset),
            u32_at(&bytes, attachment_offset + 4),
            u32_at(&bytes, attachment_offset + 8),
        ],
        targetter: TargetterState {
            base_at_controller_0x22c: controller_pointer + CONTROLLER_TARGETTER_OFFSET as u32,
            raw_hex: encode_hex(targetter_bytes),
            raw_hash: fnv1a64(targetter_bytes),
            owner_handle_at_0x00: u32_at(targetter_bytes, 0x00),
            prior_target_handle_at_0x04: u32_at(targetter_bytes, 0x04),
            current_target_handle_at_0x0c: u32_at(targetter_bytes, 0x0C),
            ray_distance_or_parameter_at_0x14: u32_at(targetter_bytes, 0x14),
            field_at_0x24: u32_at(targetter_bytes, 0x24),
            field_at_0x28: u32_at(targetter_bytes, 0x28),
            result_bits_at_0x34: u32_at(targetter_bytes, 0x34),
            scan_radius_at_0x38: u32_at(targetter_bytes, 0x38),
            callback_table_pointer_at_0x44: callback_table_pointer,
            callback_table_raw_hex,
            callback_table_hash,
            callback_table_read_error,
        },
    })
}

fn read_runtime_weapon_state(
    process: &Process,
    player_pointer: u32,
    warnings: &mut Vec<String>,
) -> Option<RuntimeWeaponState> {
    let component_root_pointer = read_pointer(
        process,
        player_pointer as usize + PLAYER_COMPONENT_ROOT_OFFSET,
        "player component root",
        warnings,
    )?;
    let component_state_pointer = read_pointer(
        process,
        component_root_pointer as usize + COMPONENT_STATE_OFFSET,
        "player component state at root +0x0C",
        warnings,
    )?;
    let weapon_state_pointer = read_pointer(
        process,
        component_state_pointer as usize + COMPONENT_WEAPON_STATE_OFFSET,
        "runtime weapon state at component +0x2C",
        warnings,
    )?;
    let bytes = match process.read_bytes(weapon_state_pointer as usize, RUNTIME_WEAPON_STATE_BYTES)
    {
        Ok(bytes) => bytes,
        Err(error) => {
            warnings.push(format!(
                "could not read runtime weapon state {weapon_state_pointer:08X}: {error}"
            ));
            return None;
        }
    };
    let joint_pointer_at_00 = u32_at(&bytes, 0x00);
    let alternate_joint_pointer_at_04 = u32_at(&bytes, 0x04);
    let barrel_joint_pointer_at_08 = u32_at(&bytes, 0x08);
    let selector_output_pointer_at_0c = u32_at(&bytes, 0x0C);
    let joint_value_at_00 = read_optional_u16_pointer(
        process,
        joint_pointer_at_00,
        "weapon-state joint pointer +0x00",
        warnings,
    );
    let alternate_joint_value_at_04 = read_optional_u16_pointer(
        process,
        alternate_joint_pointer_at_04,
        "weapon-state alternate joint pointer +0x04",
        warnings,
    );
    let barrel_joint_angle_raw = read_optional_u16_pointer(
        process,
        barrel_joint_pointer_at_08,
        "weapon-state barrel joint pointer +0x08",
        warnings,
    );
    let selector_output_value_at_0c = read_optional_u16_pointer(
        process,
        selector_output_pointer_at_0c,
        "weapon-state selector output pointer +0x0C",
        warnings,
    );

    Some(RuntimeWeaponState {
        component_root_pointer,
        component_state_pointer,
        weapon_state_pointer,
        joint_pointer_at_00,
        joint_value_at_00,
        alternate_joint_pointer_at_04,
        alternate_joint_value_at_04,
        barrel_joint_pointer_at_08,
        barrel_joint_angle_raw,
        selector_output_pointer_at_0c,
        selector_output_value_at_0c,
        field_0c: selector_output_pointer_at_0c,
        field_10: u32_at(&bytes, 0x10),
        selected_slot_at_14: u32_at(&bytes, 0x14),
        field_18: u32_at(&bytes, 0x18),
        field_1c: u32_at(&bytes, 0x1C),
        field_20: u32_at(&bytes, 0x20),
        free_fire_mode_at_24: u32_at(&bytes, 0x24),
        trigger_accumulator_at_28: u32_at(&bytes, 0x28) as i32,
        accumulator_at_28: u32_at(&bytes, 0x28) as i32,
        cadence_at_2c: u32_at(&bytes, 0x2C) as i32,
        resource_count_at_30: u32_at(&bytes, 0x30) as i32,
        interval_at_30: u32_at(&bytes, 0x30) as i32,
        cooldown_at_34: u32_at(&bytes, 0x34) as i32,
        aim_adjustment_pointer_at_38: u32_at(&bytes, 0x38),
        raw_hex: encode_hex(&bytes),
        raw_hash: fnv1a64(&bytes),
    })
}

fn read_optional_u16_pointer(
    process: &Process,
    pointer: u32,
    label: &str,
    warnings: &mut Vec<String>,
) -> Option<u16> {
    if pointer == 0 {
        return None;
    }
    if !plausible_u16_pointer(pointer as usize) {
        warnings.push(format!(
            "{label} contains implausible pointer {pointer:08X}"
        ));
        return None;
    }
    match process.read_u16(pointer as usize) {
        Ok(value) => Some(value),
        Err(error) => {
            warnings.push(format!(
                "could not read {label} target {pointer:08X}: {error}"
            ));
            None
        }
    }
}

fn plausible_u16_pointer(pointer: usize) -> bool {
    (0x1_0000..=0x7FFF_FFFE).contains(&pointer) && pointer & 1 == 0
}

fn parse_shot_record(pointer: u32, bytes: &[u8]) -> ShotRecord {
    debug_assert!(bytes.len() >= SHOT_RECORD_BYTES);
    ShotRecord {
        pointer,
        next: u32_at(bytes, 0x00),
        previous_link: u32_at(bytes, 0x04),
        projectile_or_effect_selector: u32_at(bytes, 0x08),
        selector_variant: u16_at(bytes, 0x0C),
        origin_raw_8_8: [
            i16_at(bytes, 0x0E),
            i16_at(bytes, 0x10),
            i16_at(bytes, 0x12),
        ],
        direction_raw: [
            u32_at(bytes, 0x14) as i32,
            u32_at(bytes, 0x18) as i32,
            u32_at(bytes, 0x1C) as i32,
        ],
        travel_or_time_scalar: u32_at(bytes, 0x20) as i32,
        source_reference: u32_at(bytes, 0x24),
        speed_or_scale: i16_at(bytes, 0x28),
        status: u16_at(bytes, 0x2A),
        raw_hex: encode_hex(&bytes[..SHOT_RECORD_BYTES]),
    }
}

fn parse_weapon_descriptors(bytes: &[u8], descriptor_count: usize) -> Vec<WeaponDescriptor> {
    debug_assert!(bytes.len() >= WEAPON_DESCRIPTOR_BYTES * descriptor_count);
    bytes
        .chunks_exact(WEAPON_DESCRIPTOR_BYTES)
        .take(descriptor_count)
        .enumerate()
        .map(|(index, record)| WeaponDescriptor {
            index,
            field_00: u32_at(record, 0x00),
            field_04: u32_at(record, 0x04),
            byte_08: record[0x08],
            byte_09: record[0x09],
            byte_0a: record[0x0A],
            byte_0b: record[0x0B],
            byte_0c: record[0x0C],
            byte_0d: record[0x0D],
            field_0e: u16_at(record, 0x0E),
            field_10: u32_at(record, 0x10),
            field_14: u32_at(record, 0x14),
            raw_hex: encode_hex(record),
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn validate_shot_links(
    head_link_address: u32,
    end_sentinel_address: u32,
    head: u32,
    tail: u32,
    records: &[ShotRecord],
    traversal_complete: bool,
    warnings: &mut Vec<String>,
) {
    if records.is_empty() {
        if traversal_complete && head != end_sentinel_address {
            warnings.push(format!(
                "empty traversal began at {head:08X}, not end sentinel {end_sentinel_address:08X}"
            ));
        }
        if traversal_complete && tail != head_link_address {
            warnings.push(format!(
                "empty shot list tail {tail:08X} is not head-link address {head_link_address:08X}"
            ));
        }
        return;
    }

    if records[0].pointer != head {
        warnings.push(format!(
            "first decoded shot {:08X} does not match head {head:08X}",
            records[0].pointer
        ));
    }
    let mut expected_previous = head_link_address;
    for (index, record) in records.iter().enumerate() {
        if record.previous_link != expected_previous {
            warnings.push(format!(
                "shot node {:08X} previous link {:08X}, expected {expected_previous:08X}",
                record.pointer, record.previous_link
            ));
        }
        let expected_next = records
            .get(index + 1)
            .map_or(end_sentinel_address, |next| next.pointer);
        if traversal_complete && record.next != expected_next {
            warnings.push(format!(
                "shot node {:08X} next {:08X}, expected {expected_next:08X}",
                record.pointer, record.next
            ));
        }
        expected_previous = record.pointer;
    }
    if traversal_complete && tail != records.last().unwrap().pointer {
        warnings.push(format!(
            "shot-list tail {tail:08X} does not match final node {:08X}",
            records.last().unwrap().pointer
        ));
    }
}

fn read_pointer(
    process: &Process,
    address: usize,
    label: &str,
    warnings: &mut Vec<String>,
) -> Option<u32> {
    match process.read_u32(address) {
        Ok(pointer) if plausible_heap_pointer(pointer as usize) => Some(pointer),
        Ok(pointer) => {
            warnings.push(format!(
                "{label} at {address:08X} contains implausible pointer {pointer:08X}"
            ));
            None
        }
        Err(error) => {
            warnings.push(format!("could not read {label} at {address:08X}: {error}"));
            None
        }
    }
}

fn address_u32(base: usize, offset: usize) -> Result<u32, String> {
    let address = base
        .checked_add(offset)
        .ok_or_else(|| format!("address overflow: {base:08X} + {offset:#x}"))?;
    u32::try_from(address).map_err(|_| format!("address {address:X} exceeds retail PE32 range"))
}

fn encode_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    use std::fmt::Write as _;
    for byte in bytes {
        let _ = write!(output, "{byte:02X}");
    }
    output
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put_u16(bytes: &mut [u8], offset: usize, value: u16) {
        bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn put_i16(bytes: &mut [u8], offset: usize, value: i16) {
        put_u16(bytes, offset, value as u16);
    }

    fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn put_i32(bytes: &mut [u8], offset: usize, value: i32) {
        put_u32(bytes, offset, value as u32);
    }

    fn link_only_record(pointer: u32, next: u32, previous_link: u32) -> ShotRecord {
        let mut bytes = [0u8; SHOT_RECORD_BYTES];
        put_u32(&mut bytes, 0x00, next);
        put_u32(&mut bytes, 0x04, previous_link);
        parse_shot_record(pointer, &bytes)
    }

    #[test]
    fn parses_player_pose_adjacent_to_transient_fire_state() {
        let mut bytes = [0u8; ENTITY_POSE_BYTES];
        put_i16(&mut bytes, 0x00, 14_781);
        put_i16(&mut bytes, 0x02, -1_029);
        put_i16(&mut bytes, 0x04, -31_707);
        put_i16(&mut bytes, 0x06, 12);
        put_i16(&mut bytes, 0x08, -34);
        put_i16(&mut bytes, 0x0A, 56);
        put_u16(&mut bytes, 0x0C, 0x4000);
        put_u16(&mut bytes, 0x0E, 0xF800);
        put_u16(&mut bytes, 0x10, 0x1234);

        let pose = parse_player_pose(&bytes);
        assert_eq!(pose.position_raw_8_8, [14_781, -1_029, -31_707]);
        assert_eq!(pose.position_wrapped_xz_raw_8_8, [14_781, 33_829]);
        assert_eq!(pose.velocity_raw, [12, -34, 56]);
        assert_eq!(pose.euler_raw, [0x4000, 0xF800, 0x1234]);
    }

    #[test]
    fn joint_targets_accept_two_byte_alignment() {
        assert!(plausible_u16_pointer(0x02A5_BA02));
        assert!(!plausible_u16_pointer(0x02A5_BA03));
        assert!(!plausible_u16_pointer(0));
    }

    #[test]
    fn parses_exact_packed_shot_record_layout() {
        let mut bytes = [0u8; SHOT_RECORD_BYTES];
        put_u32(&mut bytes, 0x00, 0x0020_0030);
        put_u32(&mut bytes, 0x04, 0x0020_0000);
        put_u32(&mut bytes, 0x08, 0x1A);
        put_u16(&mut bytes, 0x0C, 0xBEEF);
        put_i16(&mut bytes, 0x0E, 0x1234);
        put_i16(&mut bytes, 0x10, -0x2345);
        put_i16(&mut bytes, 0x12, 0x3456);
        put_i32(&mut bytes, 0x14, 0x1020_3040);
        put_i32(&mut bytes, 0x18, -0x1020_3040);
        put_i32(&mut bytes, 0x1C, 0x5060_7080);
        put_i32(&mut bytes, 0x20, -640);
        put_u32(&mut bytes, 0x24, 0x04BE_0001);
        put_i16(&mut bytes, 0x28, -1234);
        put_u16(&mut bytes, 0x2A, 2);

        let record = parse_shot_record(0x0020_0010, &bytes);
        assert_eq!(record.next, 0x0020_0030);
        assert_eq!(record.previous_link, 0x0020_0000);
        assert_eq!(record.projectile_or_effect_selector, 0x1A);
        assert_eq!(record.selector_variant, 0xBEEF);
        assert_eq!(record.origin_raw_8_8, [0x1234, -0x2345, 0x3456]);
        assert_eq!(
            record.direction_raw,
            [0x1020_3040, -0x1020_3040, 0x5060_7080]
        );
        assert_eq!(record.travel_or_time_scalar, -640);
        assert_eq!(record.source_reference, 0x04BE_0001);
        assert_eq!(record.speed_or_scale, -1234);
        assert_eq!(record.status, 2);
        assert_eq!(record.raw_hex.len(), SHOT_RECORD_BYTES * 2);
    }

    #[test]
    fn parses_all_32_weapon_descriptors_without_stride_drift() {
        let mut bytes = vec![0u8; WEAPON_DESCRIPTOR_BLOCK_BYTES];
        for index in 0..WEAPON_DESCRIPTOR_COUNT {
            let base = index * WEAPON_DESCRIPTOR_BYTES;
            put_u32(&mut bytes, base, 0x1000 + index as u32);
            put_u32(&mut bytes, base + 4, 0x2000 + index as u32);
            bytes[base + 8] = index as u8;
            bytes[base + 0x0A] = 8;
            bytes[base + 0x0C] = 0xA0 | index as u8;
            put_u16(&mut bytes, base + 0x0E, 0x3000 + index as u16);
            put_u32(&mut bytes, base + 0x14, 0x4000 + index as u32);
        }

        let descriptors = parse_weapon_descriptors(&bytes, WEAPON_DESCRIPTOR_COUNT);
        assert_eq!(descriptors.len(), 32);
        assert_eq!(descriptors[0].field_00, 0x1000);
        assert_eq!(descriptors[17].field_04, 0x2011);
        assert_eq!(descriptors[31].byte_08, 31);
        assert_eq!(descriptors[31].byte_0a, 8);
        assert_eq!(descriptors[31].field_0e, 0x301F);
        assert_eq!(descriptors[31].field_14, 0x401F);
        assert_eq!(descriptors[31].raw_hex.len(), WEAPON_DESCRIPTOR_BYTES * 2);
    }

    #[test]
    fn parses_the_24_record_master_weapon_catalog_without_controller_padding() {
        let mut bytes = vec![0u8; WEAPON_DESCRIPTOR_BYTES * MASTER_WEAPON_DESCRIPTOR_COUNT];
        for index in 0..MASTER_WEAPON_DESCRIPTOR_COUNT {
            let base = index * WEAPON_DESCRIPTOR_BYTES;
            put_u32(&mut bytes, base, 0x5000 + index as u32);
            put_u32(&mut bytes, base + 0x14, 0x6000 + index as u32);
        }

        let descriptors = parse_weapon_descriptors(&bytes, MASTER_WEAPON_DESCRIPTOR_COUNT);
        assert_eq!(descriptors.len(), 24);
        assert_eq!(descriptors[0].field_00, 0x5000);
        assert_eq!(descriptors[23].field_00, 0x5017);
        assert_eq!(descriptors[23].field_14, 0x6017);
    }

    #[test]
    fn validates_empty_list_sentinel_and_tail_contract() {
        let head_link = 0x0020_0038;
        let end = 0x0020_003C;
        let mut warnings = Vec::new();
        validate_shot_links(head_link, end, end, head_link, &[], true, &mut warnings);
        assert!(warnings.is_empty());

        validate_shot_links(head_link, end, end, end, &[], true, &mut warnings);
        assert!(warnings
            .iter()
            .any(|warning| warning.contains("empty shot list tail")));
    }

    #[test]
    fn validates_first_previous_link_and_node_chain() {
        let head_link = 0x0020_0038;
        let end = 0x0020_003C;
        let first = link_only_record(0x0030_0000, 0x0030_0030, head_link);
        let second = link_only_record(0x0030_0030, end, first.pointer);
        let mut warnings = Vec::new();
        validate_shot_links(
            head_link,
            end,
            first.pointer,
            second.pointer,
            &[first, second],
            true,
            &mut warnings,
        );
        assert!(warnings.is_empty());
    }

    #[test]
    fn fnv_hash_changes_when_runtime_bytes_change() {
        let mut before = [0u8; RUNTIME_WEAPON_STATE_BYTES];
        let mut after = before;
        after[0x28] = 1;
        assert_ne!(fnv1a64(&before), fnv1a64(&after));
        before[0x28] = 1;
        assert_eq!(fnv1a64(&before), fnv1a64(&after));
    }
}
