use std::collections::HashSet;

use serde::Serialize;

use crate::process::{i16_at, plausible_heap_pointer, u16_at, u32_at, Process};

pub const ENTITY_HEAD_PTR: usize = 0x004D_B090;
pub const ENTITY_END_SENTINEL: usize = 0x004D_B094;
pub const ENTITY_TAIL_PTR: usize = 0x004D_B098;
pub const TICK_50HZ: usize = 0x004F_ED60;
pub const MODEL_POOL_PTR: usize = 0x004F_E640;
const ENTITY_TYPE_POOL_PTR: usize = 0x004F_E650;
const ENTITY_BYTES: usize = 0xCC;
const MAX_ENTITIES: usize = 4096;
const POWER_UP_ENTITY_TYPE: u32 = 61;
const BEHAVIOR_WINDOW_BYTES: usize = 0x58;
const BEHAVIOR_CONTEXT_WINDOW_BYTES: usize = 0x80;
const TYPE_VTABLE_WINDOW_BYTES: usize = 0x4C;
const MODEL_DYNAMIC_CALLBACK_PAIR_BYTES: usize = 0x08;
const MODEL_DYNAMIC_CALLBACK_DATA_WINDOW_BYTES: usize = 0x40;
const COMPONENT_ROOT_WINDOW_BYTES: usize = 0x10;
const COMPONENT_SLOT_TABLE_BYTES: usize = 0x0C;
const COMPONENT_TABLE_WINDOW_BYTES: usize = 0x34;
const DAMAGE_STATE_WINDOW_BYTES: usize = 0xB8;
const COMPONENT_WRAPPER_WINDOW_BYTES: usize = 0x10;
const COMPONENT_STATE_WINDOW_BYTES: usize = 0x40;
const COMPONENT_TRANSITION_TABLE_BYTES: usize = 0x08;
const WIND_STATE_ADDRESS: usize = 0x004F_7194;
const WIND_STATE_BYTES: usize = 0x1A;
pub const NEARBY_ENTITY_RADIUS_RAW: i32 = 0x1000;
pub const MAX_NEARBY_ENTITIES: usize = 16;

/// Exact payload consumed by the retail Power Up contact callback
/// `FUN_00425AF0`.
///
/// The callback takes the low byte of entity `+0x88` as the inventory
/// selector and arithmetic-shifts the remaining bits right by eight for the
/// signed amount. Keeping the packed dword beside the decode makes later
/// capture analysis independent of a semantic guess about either field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct PowerUpPayload {
    pub packed_at_entity_0x88: u32,
    pub selector: u8,
    pub amount: i32,
}

impl PowerUpPayload {
    fn decode(packed: u32) -> Self {
        Self {
            packed_at_entity_0x88: packed,
            selector: packed as u8,
            amount: (packed as i32) >> 8,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct EntityRecord {
    /// Zero-based position in the intrusive list beginning at
    /// `0x004DB090`. This is the order consumed by `FUN_00411AD0`; it is not
    /// an allocation-address or proximity sort.
    pub intrusive_list_index: usize,
    pub pointer: u32,
    pub next: u32,
    pub flags: u32,
    pub entity_type: u32,
    pub handle: u32,
    pub parent_handle: Option<u32>,
    pub parent_pointer: Option<u32>,
    pub parent_entity_type: Option<u32>,
    pub body_basis_raw: [i32; 9],
    pub position_raw_8_8: [i16; 3],
    pub position: [f32; 3],
    pub position_wrapped: [f32; 3],
    pub velocity_raw_8_8: [i16; 3],
    pub velocity: [f32; 3],
    pub rotation: [u16; 3],
    pub models: [u16; 4],
    pub model_resources: [u32; 4],
    pub active_slot: usize,
    pub active_model: u16,
    pub active_model_resource: u32,
    pub active_model_header_hex: Option<String>,
    pub active_model_radius_raw_at_0x08: Option<u16>,
    pub active_model_collision_radius_raw_at_0x0a: Option<u16>,
    pub self_mass: u16,
    /// Signed live health consumed by the common collision/damage path.
    pub collision_health_at_0x30: i32,
    /// Written by several projectile/damage entry paths. Terrain collision
    /// does not write it, so it is supporting evidence rather than a generic
    /// contact marker.
    pub last_damage_tick_at_0x34: u32,
    /// Optional per-instance packet modifier invoked before generic damage
    /// arithmetic. Zero is an exact null callback.
    pub damage_modifier_callback_at_0x44: u32,
    /// Pre-health damage buffer drained before `collision_health_at_0x30` by
    /// the common damage path.
    pub damage_buffer_at_0x50: i32,
    /// Entity handle at `+0x60` used by `FUN_00411A20`'s symmetric recent-
    /// relation suppression. Zero means that no relation is named.
    pub recent_relation_handle_at_0x60: u32,
    /// Marker/category bits consumed by the persistent terrain-radar pass
    /// `FUN_0044AFF0`. Kept on the tick-bracketed entity record so radar
    /// capture never needs a second unbracketed read of the live entity.
    pub radar_marker_flags_at_0x64: u32,
    /// Elapsed microseconds paired with `recent_relation_handle_at_0x60`.
    pub recent_relation_elapsed_us_at_0x68: u32,
    /// Secondary accumulator advanced by the common entity scheduler.
    pub callback_scheduler_accumulator_us_at_0x6c: u32,
    /// Opaque subject-side scheduler gate read by `FUN_00411AD0`. Pair scans
    /// run only while this live value is zero.
    pub subject_scan_gate_at_0x70: u32,
    /// Remaining lifetime in microseconds for the common transient-entity
    /// scheduler path. Retail decrements this dword before destroying an
    /// expired entity; other entity classes may retain or reuse the field.
    pub remaining_lifetime_us_at_0x74: u32,
    /// Unconditional entity `+0x80` dword. Bit `0x1000` makes the same value a
    /// parent/relation handle, but AI styles also use the slot while that bit
    /// is clear, so the raw value is retained independently of `parent_handle`.
    pub raw_dword_at_0x80: u32,
    /// Logical sound record retained at entity `+0x8C`. Constructor loops and
    /// the player's lower fan both store their `FUN_0044C830` handle here.
    pub sound_attachment_handle_at_0x8c: u32,
    /// Signed 8.8 triple at entity `+0x90`. Guard Location uses it as an
    /// immutable spawn anchor; the exact raw triple is retained because other
    /// styles may give the slot a different meaning.
    pub anchor_raw_8_8_at_0x90: [i16; 3],
    /// Exact state bits consumed by the active-entity pair pass. These names
    /// describe only that pass; other retail subsystems may reuse the bits.
    pub pair_collision_enabled_state_0x00008000: bool,
    pub pair_collision_ineligible_state_0x00001000: bool,
    pub active_model_slot_low_state_0x00004000: bool,
    pub active_model_slot_high_state_0x00002000: bool,
    pub pair_collision_fixed_state_0x08000000: bool,
    pub pair_collision_cross_domain_state_0x80000000: bool,
    pub terrain_water_collision_enabled_flag_0x00010000: bool,
    pub deferred_destroy_flag_0x00100000: bool,
    pub fully_below_water_flag_0x00200000: bool,
    pub fully_above_water_flag_0x00400000: bool,
    /// Solid-contact correction / terrain-snap response bit. Although common
    /// scheduler paths can clear it, the accepted 2026-07-19 type-46 contact
    /// survey latched it at the first seabed response for the rest of the
    /// 228-second run. It is not a reliable per-contact pulse and is not by
    /// itself proof that damage was dealt or that contact persists.
    pub contact_response_flag_0x00800000: bool,
    /// Pointer at entity +0x4C. `FUN_0040A800` dereferences this before
    /// visiting its three component slots.
    pub component_root: u32,
    /// Animation/frame offset cleared by the common callback-wait branch.
    pub animation_offset_at_0xb2: u16,
    /// One-unit scheduler-delta override byte.
    pub scheduler_unit_delta_flag_at_0xb6: u8,
    pub behavior: u32,
    /// Opaque behavior context passed beside the current style.
    pub behavior_context_at_0xc0: u32,
    /// Present only for type 61. This is inline entity state, not data behind
    /// `behavior_context_at_0xc0`.
    pub power_up_payload: Option<PowerUpPayload>,
    /// Live environment/physics mask consumed by `FUN_0040E870` and related
    /// update paths. `FUN_0040D3C0` initializes it from type record `+0xC0`.
    pub environment_flags_at_0xc8: u32,
    pub raw_hex: String,
}

/// A deliberately bounded read from a remote pointer. The requested length is
/// evidence-capture policy, not a claim about the pointed-to allocation size.
#[derive(Debug, Clone, Serialize)]
pub struct RemoteWindow {
    pub pointer: u32,
    pub requested_bytes: usize,
    pub raw_hex: Option<String>,
    pub read_error: Option<String>,
}

/// A deliberately bounded read from a fixed retail executable address.
#[derive(Debug, Clone, Serialize)]
pub struct FixedMemoryWindow {
    pub address: u32,
    pub requested_bytes: usize,
    pub raw_hex: Option<String>,
    pub read_error: Option<String>,
}

/// One of the three pointers visited by `FUN_0040A800`. Field names describe
/// only offsets and accesses proven in `FUN_00401120`; unknown bytes remain in
/// the bounded raw windows.
#[derive(Debug, Clone, Serialize)]
pub struct ComponentSlotEvidence {
    pub slot_index: usize,
    pub wrapper_pointer: u32,
    pub wrapper_window: RemoteWindow,
    pub state_pointer_at_wrapper_0x00: Option<u32>,
    pub state_window: Option<RemoteWindow>,
    pub tick_callback_at_state_0x10: Option<u32>,
    /// Callback invoked by `FUN_00401290` during active-entity pair contact.
    /// The inspector records this address but never invokes it.
    pub pair_contact_callback_at_state_0x18: Option<u32>,
    /// Opaque fourth argument passed to the pair-contact callback.
    pub pair_contact_context_at_state_0x1c: Option<u32>,
    pub elapsed_milliseconds_at_state_0x2c: Option<u32>,
    pub transition_table_pointer_at_state_0x30: Option<u32>,
    pub transition_table_window: Option<RemoteWindow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ComponentChainEvidence {
    pub root_pointer_at_entity_0x4c: u32,
    pub root_window: RemoteWindow,
    pub slot_table_pointer_at_root_0x00: Option<u32>,
    pub slot_table_window: Option<RemoteWindow>,
    pub slots: Vec<ComponentSlotEvidence>,
}

/// The independently linked health/weapon state reached through
/// `entity+0x4C -> root+0x0C -> table+0x30`. These offset-named values are
/// retained alongside the raw window because not every entity type assigns
/// identical gameplay meaning to the shared allocation.
#[derive(Debug, Clone, Serialize)]
pub struct LinkedDamageStateEvidence {
    pub component_table_pointer_at_root_0x0c: Option<u32>,
    pub component_table_window: Option<RemoteWindow>,
    pub state_pointer_at_table_0x30: Option<u32>,
    pub state_window: Option<RemoteWindow>,
    pub maximum_or_initial_value_at_state_0x04: Option<i32>,
    pub flags_at_state_0x18: Option<u32>,
    pub current_value_at_state_0x68: Option<i32>,
    pub regeneration_counter_at_state_0x6c: Option<i32>,
    pub target_handle_at_state_0x88: Option<u32>,
    pub progressive_damage_at_state_0x94: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BehaviorEvidence {
    pub pointer_at_entity_0xb8: u32,
    pub bounded_window: RemoteWindow,
    /// Raw style context value stored at entity `+0xC0`. It is not assumed to
    /// be a pointer: `context_bounded_window` records an explicit bounded-read
    /// failure when the value is nonzero but not a plausible remote address.
    pub context_value_at_entity_0xc0: u32,
    pub context_bounded_window: RemoteWindow,
    /// Style `+0x0C`, reached by the relation-release dispatcher at
    /// type-vtable `+0x48` (`FUN_0040DC50`, called by `FUN_00416750`).
    pub release_callback_at_style_0x0c: Option<u32>,
    /// Style `+0x18`, reached by the active-pair dispatcher at type-vtable
    /// `+0x38` (`FUN_0040D8D0`, called by `FUN_00411AD0`).
    pub pair_contact_callback_at_style_0x18: Option<u32>,
    /// Style `+0x2C`, reached by the standard death continuation.
    pub death_callback_at_style_0x2c: Option<u32>,
    /// `FUN_0040E870` ORs this dword with entity `+0xC8`.
    pub effective_flags_or_mask_at_0x34: Option<u32>,
    /// `FUN_0040E870` clears the bits in this dword after applying the OR mask.
    pub effective_flags_clear_mask_at_0x38: Option<u32>,
}

/// Per-entity-type callback table referenced by Section-12 `+0x7C`.
#[derive(Debug, Clone, Serialize)]
pub struct EntityTypeVtableEvidence {
    pub pointer_at_type_record_0x7c: Option<u32>,
    pub bounded_window: Option<RemoteWindow>,
    /// Type-vtable `+0x20`, used by the detailed model-submission path when an
    /// entity supplies a type-specific override.
    pub callback_at_vtable_0x20: Option<u32>,
    /// Type-vtable `+0x24`, called by the common scheduler for the alternate
    /// update state selected by entity flag `0x02000000`.
    pub callback_at_vtable_0x24: Option<u32>,
    /// Type-vtable `+0x28`, optional ordinary-update callback called by the
    /// common scheduler.
    pub callback_at_vtable_0x28: Option<u32>,
    /// Type-vtable `+0x2C`, optional dispatcher receiving the two packed
    /// values passed through `FUN_00413600`.
    pub callback_at_vtable_0x2c: Option<u32>,
    /// Type-vtable `+0x30`, requested by generic live damage delivery.
    pub hit_callback_at_vtable_0x30: Option<u32>,
    /// Type-vtable `+0x38`, which dispatches the current style's pair callback.
    pub pair_dispatch_callback_at_vtable_0x38: Option<u32>,
    /// Type-vtable `+0x48`, which dispatches the current style's relation-
    /// release callback.
    pub release_dispatch_callback_at_vtable_0x48: Option<u32>,
}

/// The type-specific model callback pair installed by `FUN_004138F0` before
/// submitting an entity model to `FUN_00465870`.
///
/// The second pair dword is passed opaquely to the callback. Its bounded raw
/// window is therefore capture evidence only: it does not claim that the
/// value points to an array, nor does the inspector invoke the callback.
#[derive(Debug, Clone, Serialize)]
pub struct ModelDynamicCallbackEvidence {
    pub pair_pointer_at_type_record_0x78: Option<u32>,
    pub pair_window: Option<RemoteWindow>,
    pub callback_address_at_pair_0x00: Option<u32>,
    pub callback_data_value_at_pair_0x04: Option<u32>,
    pub callback_data_window: Option<RemoteWindow>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WindStateValues {
    pub height_limit_raw_at_0x004f7194: Option<i32>,
    pub mode_at_0x004f7198: Option<u32>,
    pub configured_vector_raw_at_0x004f719c: Option<[i16; 3]>,
    pub drag_strength_at_0x004f71a4: Option<u32>,
    pub active_vector_raw_at_0x004f71a8: Option<[i16; 3]>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WindRuntimeEvidence {
    pub bounded_window: FixedMemoryWindow,
    #[serde(flatten)]
    pub values: WindStateValues,
}

/// Extra evidence read only for `track-entity`. Keeping it out of the shared
/// entity-list snapshot avoids multiplying nested remote reads across every
/// entity in full-session captures.
#[derive(Debug, Clone, Serialize)]
pub struct EntityRuntimeEvidence {
    pub entity_pointer: u32,
    pub entity_type_table_pointer: Option<u32>,
    pub type_record_pointer: Option<u32>,
    /// Four Section-12 type-record words consumed by the active-pair
    /// pre-response sound path in `FUN_00411AD0`. These are not entity
    /// `+0x88..+0x8E`, which belongs to a different live record layout.
    pub pair_contact_sound_words_at_type_record_0x88: Option<[u16; 4]>,
    pub model_dynamic_callback: ModelDynamicCallbackEvidence,
    pub type_vtable: EntityTypeVtableEvidence,
    pub environment_flags_source_at_type_record_0xc0: Option<u32>,
    /// Retained because the first tracer captured this Section-12 dword, but
    /// it is not the live mask consumed by the entity update functions.
    pub type_record_dword_at_0xc8: Option<u32>,
    /// `FUN_0040E100` reaches this Section-12 Sub-C pointer at type record
    /// `+0xD8`, then requires its signed byte `+0x0C` to be nonzero before
    /// applying the strict underwater-depth response.
    pub underwater_config_pointer_at_type_record_0xd8: Option<u32>,
    pub underwater_config_window: Option<RemoteWindow>,
    pub underwater_response_enable_at_config_0x0c: Option<i8>,
    pub environment_flags_at_entity_0xc8: u32,
    pub effective_environment_flags: Option<u32>,
    pub behavior: BehaviorEvidence,
    pub wind: WindRuntimeEvidence,
    pub component_chain: ComponentChainEvidence,
    pub linked_damage_state: LinkedDamageStateEvidence,
    pub warnings: Vec<String>,
}

/// Compact context written beside a tracked player. The radius is toroidal in
/// X/Z, matching the retail 256-cell world wrap; Y remains ordinary signed
/// 8.8 distance. Keeping this bounded avoids serializing every full 0xCC-byte
/// entity record at 100 Hz while still proving pickup/contact identity and
/// nearby spawn/despawn transitions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NearbyEntityEvidence {
    pub intrusive_list_index: usize,
    pub pointer: u32,
    pub entity_type: u32,
    pub handle: u32,
    pub parent_handle: Option<u32>,
    pub flags: u32,
    /// Exact live orientation used by authored model collision. This is kept
    /// beside nearby position so a contact can be replayed rather than merely
    /// classified by proximity.
    pub body_basis_raw: [i32; 9],
    pub relative_position_raw_8_8: [i32; 3],
    pub horizontal_distance_squared_raw: i64,
    pub position_raw_8_8: [i16; 3],
    pub velocity_raw_8_8: [i16; 3],
    pub rotation: [u16; 3],
    pub models: [u16; 4],
    pub active_slot: usize,
    pub active_model: u16,
    pub active_model_resource: u32,
    pub active_model_radius_raw_at_0x08: Option<u16>,
    pub active_model_collision_radius_raw_at_0x0a: Option<u16>,
    pub self_mass: u16,
    pub collision_health_at_0x30: i32,
    pub damage_buffer_at_0x50: i32,
    pub recent_relation_handle_at_0x60: u32,
    pub recent_relation_elapsed_us_at_0x68: u32,
    pub subject_scan_gate_at_0x70: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EntitySignature {
    pub flags: u32,
    pub entity_type: u32,
    pub handle: u32,
    pub parent_handle: Option<u32>,
    pub models: [u16; 4],
    pub model_resources: [u32; 4],
    pub active_slot: usize,
    pub behavior: u32,
    pub power_up_payload: Option<PowerUpPayload>,
    pub collision_health_at_0x30: i32,
    pub last_damage_tick_at_0x34: u32,
    pub damage_buffer_at_0x50: i32,
}

impl EntityRecord {
    pub fn signature(&self) -> EntitySignature {
        EntitySignature {
            flags: self.flags,
            entity_type: self.entity_type,
            handle: self.handle,
            parent_handle: self.parent_handle,
            models: self.models,
            model_resources: self.model_resources,
            active_slot: self.active_slot,
            behavior: self.behavior,
            power_up_payload: self.power_up_payload,
            collision_health_at_0x30: self.collision_health_at_0x30,
            last_damage_tick_at_0x34: self.last_damage_tick_at_0x34,
            damage_buffer_at_0x50: self.damage_buffer_at_0x50,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct EntitySnapshot {
    pub tick_before: u32,
    pub tick_after: u32,
    pub head_before: u32,
    pub head_after: u32,
    pub tail_before: u32,
    pub tail_after: u32,
    pub tick_stable: bool,
    pub topology_stable: bool,
    pub model_pool_pointer: u32,
    pub records: Vec<EntityRecord>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EntityPairRuntimeScalars {
    damage_modifier_callback_at_0x44: u32,
    callback_scheduler_accumulator_us_at_0x6c: u32,
    animation_offset_at_0xb2: u16,
    scheduler_unit_delta_flag_at_0xb6: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EntityLifecycleScalars {
    remaining_lifetime_us_at_0x74: u32,
    raw_dword_at_0x80: u32,
    anchor_raw_8_8_at_0x90: [i16; 3],
}

pub fn read_snapshot(process: &Process) -> Result<EntitySnapshot, String> {
    let tick_before = process.read_u32(TICK_50HZ)?;
    let head_before = process.read_u32(ENTITY_HEAD_PTR)?;
    let tail_before = process.read_u32(ENTITY_TAIL_PTR)?;
    let model_pool_pointer = process.read_u32(MODEL_POOL_PTR)?;

    let mut records = Vec::new();
    let mut warnings = Vec::new();
    let mut visited = HashSet::new();
    let mut current = head_before as usize;

    while current != 0 && current != ENTITY_END_SENTINEL && records.len() < MAX_ENTITIES {
        if !plausible_heap_pointer(current) {
            warnings.push(format!("implausible entity pointer {current:08X}"));
            break;
        }
        if !visited.insert(current) {
            warnings.push(format!("entity-list cycle returned to {current:08X}"));
            break;
        }

        let bytes = match process.read_bytes(current, ENTITY_BYTES) {
            Ok(bytes) => bytes,
            Err(error) if records.is_empty() => {
                return Err(format!(
                    "entity head {current:08X} is unreadable ({error}); gameplay may not be active"
                ));
            }
            Err(error) => {
                warnings.push(format!("unreadable node {current:08X}: {error}"));
                break;
            }
        };
        let next = u32_at(&bytes, 0x00);
        let flags = u32_at(&bytes, 0x08);
        let entity_type = u32_at(&bytes, 0x58);
        if entity_type > 129 {
            warnings.push(format!(
                "node {current:08X} has implausible entity type {entity_type}"
            ));
            break;
        }
        let handle = u32_at(&bytes, 0x5C);
        let lifecycle = decode_entity_lifecycle_scalars(&bytes)
            .expect("the fixed 0xCC-byte entity read covers every lifecycle scalar");
        let parent_handle = (flags & 0x1000 != 0).then_some(lifecycle.raw_dword_at_0x80);
        let body_basis_raw = std::array::from_fn(|index| u32_at(&bytes, 0x0C + index * 4) as i32);
        let position_raw_8_8 = [
            i16_at(&bytes, 0x96),
            i16_at(&bytes, 0x98),
            i16_at(&bytes, 0x9A),
        ];
        let position = position_raw_8_8.map(|value| value as f32 / 256.0);
        let position_wrapped = [wrap(position[0]), position[1], wrap(position[2])];
        let velocity_raw_8_8 = [
            i16_at(&bytes, 0x9C),
            i16_at(&bytes, 0x9E),
            i16_at(&bytes, 0xA0),
        ];
        let velocity = velocity_raw_8_8.map(|value| value as f32 / 256.0);
        let rotation = [
            u16_at(&bytes, 0xA2),
            u16_at(&bytes, 0xA4),
            u16_at(&bytes, 0xA6),
        ];
        let models = [
            u16_at(&bytes, 0xA8),
            u16_at(&bytes, 0xAA),
            u16_at(&bytes, 0xAC),
            u16_at(&bytes, 0xAE),
        ];
        let active_slot = usize::from(flags & 0x2000 != 0) * 2 + usize::from(flags & 0x4000 != 0);
        let behavior = u32_at(&bytes, 0xB8);
        let environment_flags_at_0xc8 = u32_at(&bytes, 0xC8);
        let pair_runtime = decode_entity_pair_runtime_scalars(&bytes)
            .expect("the fixed 0xCC-byte entity read covers every pair-runtime scalar");

        records.push(EntityRecord {
            intrusive_list_index: records.len(),
            pointer: current as u32,
            next,
            flags,
            entity_type,
            handle,
            parent_handle,
            parent_pointer: None,
            parent_entity_type: None,
            body_basis_raw,
            position_raw_8_8,
            position,
            position_wrapped,
            velocity_raw_8_8,
            velocity,
            rotation,
            models,
            model_resources: [0; 4],
            active_slot,
            active_model: models[active_slot],
            active_model_resource: 0,
            active_model_header_hex: None,
            active_model_radius_raw_at_0x08: None,
            active_model_collision_radius_raw_at_0x0a: None,
            self_mass: u16_at(&bytes, 0xB0),
            collision_health_at_0x30: u32_at(&bytes, 0x30) as i32,
            last_damage_tick_at_0x34: u32_at(&bytes, 0x34),
            damage_modifier_callback_at_0x44: pair_runtime.damage_modifier_callback_at_0x44,
            damage_buffer_at_0x50: u32_at(&bytes, 0x50) as i32,
            recent_relation_handle_at_0x60: u32_at(&bytes, 0x60),
            radar_marker_flags_at_0x64: u32_at(&bytes, 0x64),
            recent_relation_elapsed_us_at_0x68: u32_at(&bytes, 0x68),
            callback_scheduler_accumulator_us_at_0x6c: pair_runtime
                .callback_scheduler_accumulator_us_at_0x6c,
            subject_scan_gate_at_0x70: u32_at(&bytes, 0x70),
            remaining_lifetime_us_at_0x74: lifecycle.remaining_lifetime_us_at_0x74,
            raw_dword_at_0x80: lifecycle.raw_dword_at_0x80,
            sound_attachment_handle_at_0x8c: u32_at(&bytes, 0x8C),
            anchor_raw_8_8_at_0x90: lifecycle.anchor_raw_8_8_at_0x90,
            pair_collision_enabled_state_0x00008000: flags & 0x0000_8000 != 0,
            pair_collision_ineligible_state_0x00001000: flags & 0x0000_1000 != 0,
            active_model_slot_low_state_0x00004000: flags & 0x0000_4000 != 0,
            active_model_slot_high_state_0x00002000: flags & 0x0000_2000 != 0,
            pair_collision_fixed_state_0x08000000: flags & 0x0800_0000 != 0,
            pair_collision_cross_domain_state_0x80000000: flags & 0x8000_0000 != 0,
            terrain_water_collision_enabled_flag_0x00010000: flags & 0x0001_0000 != 0,
            deferred_destroy_flag_0x00100000: flags & 0x0010_0000 != 0,
            fully_below_water_flag_0x00200000: flags & 0x0020_0000 != 0,
            fully_above_water_flag_0x00400000: flags & 0x0040_0000 != 0,
            contact_response_flag_0x00800000: flags & 0x0080_0000 != 0,
            component_root: u32_at(&bytes, 0x4C),
            animation_offset_at_0xb2: pair_runtime.animation_offset_at_0xb2,
            scheduler_unit_delta_flag_at_0xb6: pair_runtime.scheduler_unit_delta_flag_at_0xb6,
            behavior,
            behavior_context_at_0xc0: u32_at(&bytes, 0xC0),
            power_up_payload: (entity_type == POWER_UP_ENTITY_TYPE)
                .then(|| PowerUpPayload::decode(u32_at(&bytes, 0x88))),
            environment_flags_at_0xc8,
            raw_hex: encode_hex(&bytes),
        });

        current = next as usize;
    }

    if records.len() == MAX_ENTITIES {
        warnings.push(format!("safety limit of {MAX_ENTITIES} nodes reached"));
    }
    // Resolve the model indirection table in one remote read. Four individual
    // ReadProcessMemory calls per entity were needlessly expensive at the
    // 50 Hz full-session sampling rate.
    if plausible_heap_pointer(model_pool_pointer as usize) {
        if let Some(max_model) = records.iter().flat_map(|record| record.models).max() {
            match process.read_bytes(
                model_pool_pointer as usize,
                (usize::from(max_model) + 1) * 4,
            ) {
                Ok(table) => {
                    let mut header_cache: std::collections::HashMap<
                        u32,
                        Option<(String, u16, u16)>,
                    > = std::collections::HashMap::new();
                    for record in &mut records {
                        record.model_resources = record
                            .models
                            .map(|model_id| u32_at(&table, usize::from(model_id) * 4));
                        record.active_model_resource = record.model_resources[record.active_slot];
                        let header = header_cache
                            .entry(record.active_model_resource)
                            .or_insert_with(|| {
                                plausible_heap_pointer(record.active_model_resource as usize)
                                    .then(|| {
                                        process
                                            .read_bytes(record.active_model_resource as usize, 0x20)
                                            .ok()
                                    })
                                    .flatten()
                                    .map(|bytes| {
                                        (
                                            encode_hex(&bytes),
                                            u16_at(&bytes, 0x08),
                                            u16_at(&bytes, 0x0A),
                                        )
                                    })
                            })
                            .clone();
                        record.active_model_header_hex =
                            header.as_ref().map(|(hex, _, _)| hex.clone());
                        record.active_model_radius_raw_at_0x08 =
                            header.as_ref().map(|(_, radius, _)| *radius);
                        record.active_model_collision_radius_raw_at_0x0a =
                            header.as_ref().map(|(_, _, radius)| *radius);
                    }
                }
                Err(error) => warnings.push(format!(
                    "model table {model_pool_pointer:08X} is unreadable: {error}"
                )),
            }
        }
    }
    let parents: std::collections::HashMap<u32, (u32, u32)> = records
        .iter()
        .map(|record| (record.handle, (record.pointer, record.entity_type)))
        .collect();
    for record in &mut records {
        if let Some((pointer, entity_type)) = record
            .parent_handle
            .and_then(|handle| parents.get(&handle).copied())
        {
            record.parent_pointer = Some(pointer);
            record.parent_entity_type = Some(entity_type);
        }
    }
    if tail_before != 0 && !visited.contains(&(tail_before as usize)) {
        warnings.push(format!("advertised tail {tail_before:08X} was not reached"));
    }

    let head_after = process.read_u32(ENTITY_HEAD_PTR)?;
    let tail_after = process.read_u32(ENTITY_TAIL_PTR)?;
    let tick_after = process.read_u32(TICK_50HZ)?;

    Ok(EntitySnapshot {
        tick_before,
        tick_after,
        head_before,
        head_after,
        tail_before,
        tail_after,
        tick_stable: tick_before == tick_after,
        topology_stable: head_before == head_after && tail_before == tail_after,
        model_pool_pointer,
        records,
        warnings,
    })
}

pub fn nearby_entities(
    focus: &EntityRecord,
    records: &[EntityRecord],
) -> Vec<NearbyEntityEvidence> {
    let radius_squared = i64::from(NEARBY_ENTITY_RADIUS_RAW).pow(2);
    let mut candidates = records
        .iter()
        .filter(|record| record.pointer != focus.pointer)
        .filter_map(|record| {
            let relative_x =
                wrapped_axis_delta_raw(record.position_raw_8_8[0], focus.position_raw_8_8[0]);
            let relative_z =
                wrapped_axis_delta_raw(record.position_raw_8_8[2], focus.position_raw_8_8[2]);
            let horizontal_distance_squared_raw =
                i64::from(relative_x).pow(2) + i64::from(relative_z).pow(2);
            (horizontal_distance_squared_raw <= radius_squared).then_some((
                horizontal_distance_squared_raw,
                record.pointer,
                NearbyEntityEvidence {
                    intrusive_list_index: record.intrusive_list_index,
                    pointer: record.pointer,
                    entity_type: record.entity_type,
                    handle: record.handle,
                    parent_handle: record.parent_handle,
                    flags: record.flags,
                    body_basis_raw: record.body_basis_raw,
                    relative_position_raw_8_8: [
                        relative_x,
                        i32::from(record.position_raw_8_8[1])
                            - i32::from(focus.position_raw_8_8[1]),
                        relative_z,
                    ],
                    horizontal_distance_squared_raw,
                    position_raw_8_8: record.position_raw_8_8,
                    velocity_raw_8_8: record.velocity_raw_8_8,
                    rotation: record.rotation,
                    models: record.models,
                    active_slot: record.active_slot,
                    active_model: record.active_model,
                    active_model_resource: record.active_model_resource,
                    active_model_radius_raw_at_0x08: record.active_model_radius_raw_at_0x08,
                    active_model_collision_radius_raw_at_0x0a: record
                        .active_model_collision_radius_raw_at_0x0a,
                    self_mass: record.self_mass,
                    collision_health_at_0x30: record.collision_health_at_0x30,
                    damage_buffer_at_0x50: record.damage_buffer_at_0x50,
                    recent_relation_handle_at_0x60: record.recent_relation_handle_at_0x60,
                    recent_relation_elapsed_us_at_0x68: record.recent_relation_elapsed_us_at_0x68,
                    subject_scan_gate_at_0x70: record.subject_scan_gate_at_0x70,
                },
            ))
        })
        .collect::<Vec<_>>();
    candidates.sort_unstable_by_key(|(distance, pointer, _)| (*distance, *pointer));
    candidates.truncate(MAX_NEARBY_ENTITIES);
    candidates
        .into_iter()
        .map(|(_, _, evidence)| evidence)
        .collect()
}

fn wrapped_axis_delta_raw(position: i16, origin: i16) -> i32 {
    i32::from(position.wrapping_sub(origin))
}

pub fn read_runtime_evidence(process: &Process, record: &EntityRecord) -> EntityRuntimeEvidence {
    let mut warnings = Vec::new();

    let entity_type_table_pointer = match process.read_u32(ENTITY_TYPE_POOL_PTR) {
        Ok(pointer) if plausible_heap_pointer(pointer as usize) => Some(pointer),
        Ok(pointer) => {
            warnings.push(format!(
                "entity type table global contains implausible pointer {pointer:08X}"
            ));
            None
        }
        Err(error) => {
            warnings.push(format!("could not read entity type table pointer: {error}"));
            None
        }
    };
    let type_record_pointer = entity_type_table_pointer.and_then(|table| {
        let address = table as usize + record.entity_type as usize * 4;
        match process.read_u32(address) {
            Ok(pointer) if plausible_heap_pointer(pointer as usize) => Some(pointer),
            Ok(pointer) => {
                warnings.push(format!(
                    "type {} table entry contains implausible pointer {pointer:08X}",
                    record.entity_type
                ));
                None
            }
            Err(error) => {
                warnings.push(format!(
                    "could not read type {} table entry: {error}",
                    record.entity_type
                ));
                None
            }
        }
    });
    let mut read_type_record_dword = |offset: usize| {
        type_record_pointer.and_then(
            |pointer| match process.read_u32(pointer as usize + offset) {
                Ok(value) => Some(value),
                Err(error) => {
                    warnings.push(format!(
                        "could not read type record {pointer:08X} +0x{offset:02X}: {error}"
                    ));
                    None
                }
            },
        )
    };
    let environment_flags_source_at_type_record_0xc0 = read_type_record_dword(0xC0);
    let type_record_dword_at_0xc8 = read_type_record_dword(0xC8);
    let pair_contact_sound_dword_0x88 = read_type_record_dword(0x88);
    let pair_contact_sound_dword_0x8c = read_type_record_dword(0x8C);
    let pair_contact_sound_words_at_type_record_0x88 = pair_contact_sound_dword_0x88
        .zip(pair_contact_sound_dword_0x8c)
        .map(|(low, high)| decode_pair_contact_sound_words(low, high));
    let model_callback_pair_pointer = read_type_record_dword(0x78);
    let type_vtable_pointer = read_type_record_dword(0x7C);
    let underwater_config_pointer = read_type_record_dword(0xD8);
    let (underwater_config_window, underwater_config_bytes) =
        underwater_config_pointer.map_or((None, None), |pointer| {
            let (window, bytes) = read_remote_window(process, pointer, 0x10);
            (Some(window), bytes)
        });
    let underwater_response_enable_at_config_0x0c = underwater_config_bytes
        .as_deref()
        .and_then(|bytes| bytes.get(0x0C))
        .map(|value| *value as i8);
    let (model_callback_pair_window, model_callback_pair_bytes) = model_callback_pair_pointer
        .map_or((None, None), |pointer| {
            let (window, bytes) =
                read_remote_window(process, pointer, MODEL_DYNAMIC_CALLBACK_PAIR_BYTES);
            (Some(window), bytes)
        });
    let model_callback_pair = model_callback_pair_bytes
        .as_deref()
        .and_then(decode_model_dynamic_callback_pair);
    let callback_data_window = model_callback_pair.map(|pair| {
        read_remote_window(
            process,
            pair.callback_data_value,
            MODEL_DYNAMIC_CALLBACK_DATA_WINDOW_BYTES,
        )
        .0
    });
    let model_dynamic_callback = ModelDynamicCallbackEvidence {
        pair_pointer_at_type_record_0x78: model_callback_pair_pointer,
        pair_window: model_callback_pair_window,
        callback_address_at_pair_0x00: model_callback_pair.map(|pair| pair.callback_address),
        callback_data_value_at_pair_0x04: model_callback_pair.map(|pair| pair.callback_data_value),
        callback_data_window,
    };

    let (type_vtable_window, type_vtable_bytes) =
        type_vtable_pointer.map_or((None, None), |pointer| {
            let (window, bytes) = read_remote_window(process, pointer, TYPE_VTABLE_WINDOW_BYTES);
            (Some(window), bytes)
        });
    let type_vtable =
        decode_type_vtable(type_vtable_pointer, type_vtable_window, type_vtable_bytes);

    let (behavior_window, behavior_bytes) =
        read_remote_window(process, record.behavior, BEHAVIOR_WINDOW_BYTES);
    let (behavior_context_window, _) = read_remote_window(
        process,
        record.behavior_context_at_0xc0,
        BEHAVIOR_CONTEXT_WINDOW_BYTES,
    );
    let behavior_or_mask = behavior_bytes
        .as_deref()
        .and_then(|bytes| bounded_u32(bytes, 0x34))
        .or((record.behavior == 0).then_some(0));
    let behavior_clear_mask = behavior_bytes
        .as_deref()
        .and_then(|bytes| bounded_u32(bytes, 0x38))
        .or((record.behavior == 0).then_some(0));
    let effective_environment_flags =
        behavior_or_mask
            .zip(behavior_clear_mask)
            .map(|(or_mask, clear_mask)| {
                apply_behavior_environment_masks(
                    record.environment_flags_at_0xc8,
                    or_mask,
                    clear_mask,
                )
            });
    let (release_callback, pair_contact_callback, death_callback) = behavior_bytes
        .as_deref()
        .map(decode_behavior_callbacks)
        .unwrap_or_else(|| {
            let null_callback = (record.behavior == 0).then_some(0);
            (null_callback, null_callback, null_callback)
        });
    let behavior = BehaviorEvidence {
        pointer_at_entity_0xb8: record.behavior,
        bounded_window: behavior_window,
        context_value_at_entity_0xc0: record.behavior_context_at_0xc0,
        context_bounded_window: behavior_context_window,
        release_callback_at_style_0x0c: release_callback,
        pair_contact_callback_at_style_0x18: pair_contact_callback,
        death_callback_at_style_0x2c: death_callback,
        effective_flags_or_mask_at_0x34: behavior_or_mask,
        effective_flags_clear_mask_at_0x38: behavior_clear_mask,
    };
    let wind = read_wind_runtime_evidence(process);

    let (root_window, root_bytes) =
        read_remote_window(process, record.component_root, COMPONENT_ROOT_WINDOW_BYTES);
    let slot_table_pointer = root_bytes
        .as_deref()
        .and_then(|bytes| bounded_u32(bytes, 0));
    let (slot_table_window, slot_table_bytes) =
        slot_table_pointer.map_or((None, None), |pointer| {
            let (window, bytes) = read_remote_window(process, pointer, COMPONENT_SLOT_TABLE_BYTES);
            (Some(window), bytes)
        });
    let mut slots = Vec::with_capacity(3);
    if let Some(table_bytes) = slot_table_bytes.as_deref() {
        for slot_index in 0..3 {
            let wrapper_pointer = bounded_u32(table_bytes, slot_index * 4).unwrap_or(0);
            let (wrapper_window, wrapper_bytes) =
                read_remote_window(process, wrapper_pointer, COMPONENT_WRAPPER_WINDOW_BYTES);
            let state_pointer = wrapper_bytes
                .as_deref()
                .and_then(|bytes| bounded_u32(bytes, 0));
            let (state_window, state_bytes) = state_pointer.map_or((None, None), |pointer| {
                let (window, bytes) =
                    read_remote_window(process, pointer, COMPONENT_STATE_WINDOW_BYTES);
                (Some(window), bytes)
            });
            let tick_callback = state_bytes
                .as_deref()
                .and_then(|bytes| bounded_u32(bytes, 0x10));
            let (pair_contact_callback, pair_contact_context) = state_bytes
                .as_deref()
                .map(decode_component_pair_contact)
                .unwrap_or((None, None));
            let elapsed_milliseconds = state_bytes
                .as_deref()
                .and_then(|bytes| bounded_u32(bytes, 0x2C));
            let transition_table_pointer = state_bytes
                .as_deref()
                .and_then(|bytes| bounded_u32(bytes, 0x30));
            let transition_table_window = transition_table_pointer.and_then(|pointer| {
                (pointer != 0).then(|| {
                    read_remote_window(process, pointer, COMPONENT_TRANSITION_TABLE_BYTES).0
                })
            });
            slots.push(ComponentSlotEvidence {
                slot_index,
                wrapper_pointer,
                wrapper_window,
                state_pointer_at_wrapper_0x00: state_pointer,
                state_window,
                tick_callback_at_state_0x10: tick_callback,
                pair_contact_callback_at_state_0x18: pair_contact_callback,
                pair_contact_context_at_state_0x1c: pair_contact_context,
                elapsed_milliseconds_at_state_0x2c: elapsed_milliseconds,
                transition_table_pointer_at_state_0x30: transition_table_pointer,
                transition_table_window,
            });
        }
    }
    let component_chain = ComponentChainEvidence {
        root_pointer_at_entity_0x4c: record.component_root,
        root_window,
        slot_table_pointer_at_root_0x00: slot_table_pointer,
        slot_table_window,
        slots,
    };

    let component_table_pointer = root_bytes
        .as_deref()
        .and_then(|bytes| bounded_u32(bytes, 0x0C));
    let (component_table_window, component_table_bytes) =
        component_table_pointer.map_or((None, None), |pointer| {
            let (window, bytes) =
                read_remote_window(process, pointer, COMPONENT_TABLE_WINDOW_BYTES);
            (Some(window), bytes)
        });
    let damage_state_pointer = component_table_bytes
        .as_deref()
        .and_then(|bytes| bounded_u32(bytes, 0x30));
    let (damage_state_window, damage_state_bytes) =
        damage_state_pointer.map_or((None, None), |pointer| {
            let (window, bytes) = read_remote_window(process, pointer, DAMAGE_STATE_WINDOW_BYTES);
            (Some(window), bytes)
        });
    let linked_damage_state = LinkedDamageStateEvidence {
        component_table_pointer_at_root_0x0c: component_table_pointer,
        component_table_window,
        state_pointer_at_table_0x30: damage_state_pointer,
        state_window: damage_state_window,
        maximum_or_initial_value_at_state_0x04: damage_state_bytes
            .as_deref()
            .and_then(|bytes| bounded_i32(bytes, 0x04)),
        flags_at_state_0x18: damage_state_bytes
            .as_deref()
            .and_then(|bytes| bounded_u32(bytes, 0x18)),
        current_value_at_state_0x68: damage_state_bytes
            .as_deref()
            .and_then(|bytes| bounded_i32(bytes, 0x68)),
        regeneration_counter_at_state_0x6c: damage_state_bytes
            .as_deref()
            .and_then(|bytes| bounded_i32(bytes, 0x6C)),
        target_handle_at_state_0x88: damage_state_bytes
            .as_deref()
            .and_then(|bytes| bounded_u32(bytes, 0x88)),
        progressive_damage_at_state_0x94: damage_state_bytes
            .as_deref()
            .and_then(|bytes| bounded_i32(bytes, 0x94)),
    };

    EntityRuntimeEvidence {
        entity_pointer: record.pointer,
        entity_type_table_pointer,
        type_record_pointer,
        pair_contact_sound_words_at_type_record_0x88,
        model_dynamic_callback,
        type_vtable,
        environment_flags_source_at_type_record_0xc0,
        type_record_dword_at_0xc8,
        underwater_config_pointer_at_type_record_0xd8: underwater_config_pointer,
        underwater_config_window,
        underwater_response_enable_at_config_0x0c,
        environment_flags_at_entity_0xc8: record.environment_flags_at_0xc8,
        effective_environment_flags,
        behavior,
        wind,
        component_chain,
        linked_damage_state,
        warnings,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ModelDynamicCallbackPair {
    callback_address: u32,
    callback_data_value: u32,
}

fn decode_model_dynamic_callback_pair(bytes: &[u8]) -> Option<ModelDynamicCallbackPair> {
    Some(ModelDynamicCallbackPair {
        callback_address: bounded_u32(bytes, 0x00)?,
        callback_data_value: bounded_u32(bytes, 0x04)?,
    })
}

fn decode_behavior_callbacks(bytes: &[u8]) -> (Option<u32>, Option<u32>, Option<u32>) {
    (
        bounded_u32(bytes, 0x0C),
        bounded_u32(bytes, 0x18),
        bounded_u32(bytes, 0x2C),
    )
}

fn decode_pair_contact_sound_words(low: u32, high: u32) -> [u16; 4] {
    [
        low as u16,
        (low >> 16) as u16,
        high as u16,
        (high >> 16) as u16,
    ]
}

fn decode_type_vtable(
    pointer: Option<u32>,
    bounded_window: Option<RemoteWindow>,
    bytes: Option<Vec<u8>>,
) -> EntityTypeVtableEvidence {
    EntityTypeVtableEvidence {
        pointer_at_type_record_0x7c: pointer,
        bounded_window,
        callback_at_vtable_0x20: bytes.as_deref().and_then(|bytes| bounded_u32(bytes, 0x20)),
        callback_at_vtable_0x24: bytes.as_deref().and_then(|bytes| bounded_u32(bytes, 0x24)),
        callback_at_vtable_0x28: bytes.as_deref().and_then(|bytes| bounded_u32(bytes, 0x28)),
        callback_at_vtable_0x2c: bytes.as_deref().and_then(|bytes| bounded_u32(bytes, 0x2C)),
        hit_callback_at_vtable_0x30: bytes.as_deref().and_then(|bytes| bounded_u32(bytes, 0x30)),
        pair_dispatch_callback_at_vtable_0x38: bytes
            .as_deref()
            .and_then(|bytes| bounded_u32(bytes, 0x38)),
        release_dispatch_callback_at_vtable_0x48: bytes
            .as_deref()
            .and_then(|bytes| bounded_u32(bytes, 0x48)),
    }
}

fn apply_behavior_environment_masks(base: u32, or_mask: u32, clear_mask: u32) -> u32 {
    (base | or_mask) & !clear_mask
}

fn read_wind_runtime_evidence(process: &Process) -> WindRuntimeEvidence {
    let (bounded_window, bytes) =
        read_fixed_memory_window(process, WIND_STATE_ADDRESS, WIND_STATE_BYTES);
    let values = decode_wind_state(bytes.as_deref().unwrap_or_default());
    WindRuntimeEvidence {
        bounded_window,
        values,
    }
}

fn decode_wind_state(bytes: &[u8]) -> WindStateValues {
    WindStateValues {
        height_limit_raw_at_0x004f7194: bounded_u32(bytes, 0x00).map(|value| value as i32),
        mode_at_0x004f7198: bounded_u32(bytes, 0x04),
        configured_vector_raw_at_0x004f719c: bounded_i16_vector3(bytes, 0x08),
        drag_strength_at_0x004f71a4: bounded_u32(bytes, 0x10),
        active_vector_raw_at_0x004f71a8: bounded_i16_vector3(bytes, 0x14),
    }
}

fn read_remote_window(
    process: &Process,
    pointer: u32,
    requested_bytes: usize,
) -> (RemoteWindow, Option<Vec<u8>>) {
    if pointer == 0 {
        return (
            RemoteWindow {
                pointer,
                requested_bytes,
                raw_hex: None,
                read_error: None,
            },
            None,
        );
    }
    if !plausible_heap_pointer(pointer as usize) {
        let error = format!("pointer {pointer:08X} is outside the bounded aligned read range");
        return (
            RemoteWindow {
                pointer,
                requested_bytes,
                raw_hex: None,
                read_error: Some(error),
            },
            None,
        );
    }
    match process.read_bytes(pointer as usize, requested_bytes) {
        Ok(bytes) => (
            RemoteWindow {
                pointer,
                requested_bytes,
                raw_hex: Some(encode_hex(&bytes)),
                read_error: None,
            },
            Some(bytes),
        ),
        Err(error) => (
            RemoteWindow {
                pointer,
                requested_bytes,
                raw_hex: None,
                read_error: Some(error),
            },
            None,
        ),
    }
}

fn read_fixed_memory_window(
    process: &Process,
    address: usize,
    requested_bytes: usize,
) -> (FixedMemoryWindow, Option<Vec<u8>>) {
    match process.read_bytes(address, requested_bytes) {
        Ok(bytes) => (
            FixedMemoryWindow {
                address: address as u32,
                requested_bytes,
                raw_hex: Some(encode_hex(&bytes)),
                read_error: None,
            },
            Some(bytes),
        ),
        Err(error) => (
            FixedMemoryWindow {
                address: address as u32,
                requested_bytes,
                raw_hex: None,
                read_error: Some(error),
            },
            None,
        ),
    }
}

fn bounded_i16(bytes: &[u8], offset: usize) -> Option<i16> {
    let bytes = bytes.get(offset..offset.checked_add(2)?)?;
    Some(i16::from_le_bytes(bytes.try_into().ok()?))
}

fn bounded_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    let bytes = bytes.get(offset..offset.checked_add(2)?)?;
    Some(u16::from_le_bytes(bytes.try_into().ok()?))
}

fn bounded_i16_vector3(bytes: &[u8], offset: usize) -> Option<[i16; 3]> {
    Some([
        bounded_i16(bytes, offset)?,
        bounded_i16(bytes, offset.checked_add(2)?)?,
        bounded_i16(bytes, offset.checked_add(4)?)?,
    ])
}

fn bounded_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let bytes = bytes.get(offset..offset.checked_add(4)?)?;
    Some(u32::from_le_bytes(bytes.try_into().ok()?))
}

fn bounded_i32(bytes: &[u8], offset: usize) -> Option<i32> {
    bounded_u32(bytes, offset).map(|value| value as i32)
}

fn decode_entity_pair_runtime_scalars(bytes: &[u8]) -> Option<EntityPairRuntimeScalars> {
    Some(EntityPairRuntimeScalars {
        damage_modifier_callback_at_0x44: bounded_u32(bytes, 0x44)?,
        callback_scheduler_accumulator_us_at_0x6c: bounded_u32(bytes, 0x6C)?,
        animation_offset_at_0xb2: bounded_u16(bytes, 0xB2)?,
        scheduler_unit_delta_flag_at_0xb6: *bytes.get(0xB6)?,
    })
}

fn decode_entity_lifecycle_scalars(bytes: &[u8]) -> Option<EntityLifecycleScalars> {
    Some(EntityLifecycleScalars {
        remaining_lifetime_us_at_0x74: bounded_u32(bytes, 0x74)?,
        raw_dword_at_0x80: bounded_u32(bytes, 0x80)?,
        anchor_raw_8_8_at_0x90: bounded_i16_vector3(bytes, 0x90)?,
    })
}

/// Decode the two dwords consumed by `FUN_00401290`. A zero callback is an
/// exact observed no-op, while `None` means the bounded remote read did not
/// contain that dword.
fn decode_component_pair_contact(bytes: &[u8]) -> (Option<u32>, Option<u32>) {
    (bounded_u32(bytes, 0x18), bounded_u32(bytes, 0x1C))
}

pub fn write_text(
    process: &Process,
    snapshot: &EntitySnapshot,
    output: &mut dyn std::io::Write,
) -> std::io::Result<()> {
    writeln!(output, "[V2000 ORIGINAL LIVE ENTITY SNAPSHOT]")?;
    writeln!(
        output,
        "PROCESS: {}  PID: {}  RETAIL BUILD: {}",
        process.exe_name, process.process_id, process.build.verified_retail_build
    )?;
    writeln!(
        output,
        "TICK: {} -> {}  HEAD: {:08X} -> {:08X}  TAIL: {:08X} -> {:08X}  MODEL_POOL: {:08X}",
        snapshot.tick_before,
        snapshot.tick_after,
        snapshot.head_before,
        snapshot.head_after,
        snapshot.tail_before,
        snapshot.tail_after,
        snapshot.model_pool_pointer
    )?;
    writeln!(
        output,
        "FIELDS: PTR NEXT FLAGS TYPE HANDLE POS(raw 8.8 / wrapped) VEL(raw 8.8) ROT MODEL[0..3] SLOT ACTIVE_MODEL MASS BEHAVIOR ENV+C8"
    )?;
    for (index, record) in snapshot.records.iter().enumerate() {
        writeln!(
            output,
            "#{index:03} PTR {:08X} NEXT {:08X} FLAGS {:08X} TYPE {:3} HANDLE {:08X} \
             POS [{:+7.3},{:+7.3},{:+7.3}] RAW [{:+6},{:+6},{:+6}] WRAP [{:7.3},{:+7.3},{:7.3}] \
             VEL [{:+6},{:+6},{:+6}] ROT [{:04X},{:04X},{:04X}] MODEL [{:3},{:3},{:3},{:3}] SLOT {} ACTIVE {:3}@{:08X} MASS {:3} BEHAVIOR {:08X} ENV+C8 {:08X}",
            record.pointer,
            record.next,
            record.flags,
            record.entity_type,
            record.handle,
            record.position[0],
            record.position[1],
            record.position[2],
            record.position_raw_8_8[0],
            record.position_raw_8_8[1],
            record.position_raw_8_8[2],
            record.position_wrapped[0],
            record.position_wrapped[1],
            record.position_wrapped[2],
            record.velocity_raw_8_8[0],
            record.velocity_raw_8_8[1],
            record.velocity_raw_8_8[2],
            record.rotation[0],
            record.rotation[1],
            record.rotation[2],
            record.models[0],
            record.models[1],
            record.models[2],
            record.models[3],
            record.active_slot,
            record.active_model,
            record.active_model_resource,
            record.self_mass,
            record.behavior,
            record.environment_flags_at_0xc8,
        )?;
    }
    writeln!(
        output,
        "SNAPSHOT: tick={} topology={}",
        if snapshot.tick_stable {
            "STABLE"
        } else {
            "CHANGED"
        },
        if snapshot.topology_stable {
            "STABLE"
        } else {
            "CHANGED"
        }
    )?;
    for warning in &snapshot.warnings {
        writeln!(output, "WARNING: {warning}")?;
    }
    writeln!(output, "ENTITIES READ: {}", snapshot.records.len())?;
    Ok(())
}

pub fn changed_offsets(previous_hex: &str, current_hex: &str) -> Vec<u16> {
    let previous = decode_hex(previous_hex);
    let current = decode_hex(current_hex);
    previous
        .iter()
        .zip(&current)
        .enumerate()
        .filter_map(|(offset, (before, after))| (before != after).then_some(offset as u16))
        .collect()
}

fn encode_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    for &byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0xF) as usize] as char);
    }
    output
}

fn decode_hex(hex: &str) -> Vec<u8> {
    hex.as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = hex_digit(pair[0]);
            let low = hex_digit(pair[1]);
            high << 4 | low
        })
        .collect()
}

fn hex_digit(digit: u8) -> u8 {
    match digit {
        b'0'..=b'9' => digit - b'0',
        b'A'..=b'F' => digit - b'A' + 10,
        b'a'..=b'f' => digit - b'a' + 10,
        _ => 0,
    }
}

fn wrap(value: f32) -> f32 {
    value.rem_euclid(256.0)
}

#[cfg(test)]
mod tests {
    use super::{
        apply_behavior_environment_masks, bounded_u32, changed_offsets, decode_behavior_callbacks,
        decode_component_pair_contact, decode_entity_lifecycle_scalars,
        decode_entity_pair_runtime_scalars, decode_model_dynamic_callback_pair,
        decode_pair_contact_sound_words, decode_type_vtable, decode_wind_state,
        wrapped_axis_delta_raw, EntityLifecycleScalars, EntityPairRuntimeScalars,
        ModelDynamicCallbackPair, PowerUpPayload, RemoteWindow, TYPE_VTABLE_WINDOW_BYTES,
        WIND_STATE_BYTES,
    };

    #[test]
    fn raw_change_offsets_are_byte_offsets() {
        assert_eq!(changed_offsets("001122334455", "0011FF3344AA"), vec![2, 5]);
    }

    #[test]
    fn power_up_payload_uses_low_selector_and_signed_shifted_amount() {
        assert_eq!(
            PowerUpPayload::decode(0x0000_C802),
            PowerUpPayload {
                packed_at_entity_0x88: 0x0000_C802,
                selector: 2,
                amount: 200,
            }
        );
        assert_eq!(PowerUpPayload::decode(0xFFFF_FF3C).selector, 0x3C);
        assert_eq!(PowerUpPayload::decode(0xFFFF_FF3C).amount, -1);
    }

    #[test]
    fn nearby_axis_delta_uses_the_retail_world_wrap() {
        assert_eq!(wrapped_axis_delta_raw(-32760, 32760), 16);
        assert_eq!(wrapped_axis_delta_raw(32760, -32760), -16);
        assert_eq!(wrapped_axis_delta_raw(1000, 750), 250);
    }

    #[test]
    fn bounded_u32_rejects_truncated_remote_windows() {
        assert_eq!(bounded_u32(&[0x78, 0x56, 0x34, 0x12], 0), Some(0x1234_5678));
        assert_eq!(bounded_u32(&[0x78, 0x56, 0x34], 0), None);
        assert_eq!(bounded_u32(&[0; 8], usize::MAX), None);
    }

    #[test]
    fn pair_runtime_scalars_use_exact_entity_offsets_and_widths() {
        let mut bytes = [0_u8; 0xCC];
        bytes[0x44..0x48].copy_from_slice(&0x0044_7D70_u32.to_le_bytes());
        bytes[0x6C..0x70].copy_from_slice(&0x1234_5678_u32.to_le_bytes());
        bytes[0xB2..0xB4].copy_from_slice(&0xABCD_u16.to_le_bytes());
        bytes[0xB6] = 1;
        assert_eq!(
            decode_entity_pair_runtime_scalars(&bytes),
            Some(EntityPairRuntimeScalars {
                damage_modifier_callback_at_0x44: 0x0044_7D70,
                callback_scheduler_accumulator_us_at_0x6c: 0x1234_5678,
                animation_offset_at_0xb2: 0xABCD,
                scheduler_unit_delta_flag_at_0xb6: 1,
            })
        );
        assert_eq!(decode_entity_pair_runtime_scalars(&bytes[..0xB6]), None);
    }

    #[test]
    fn lifecycle_scalars_use_exact_entity_offsets_and_widths() {
        let mut bytes = [0_u8; 0xCC];
        bytes[0x74..0x78].copy_from_slice(&3_500_000_u32.to_le_bytes());
        bytes[0x80..0x84].copy_from_slice(&0x04B7_0001_u32.to_le_bytes());
        bytes[0x90..0x92].copy_from_slice(&(-1024_i16).to_le_bytes());
        bytes[0x92..0x94].copy_from_slice(&256_i16.to_le_bytes());
        bytes[0x94..0x96].copy_from_slice(&2047_i16.to_le_bytes());

        assert_eq!(
            decode_entity_lifecycle_scalars(&bytes),
            Some(EntityLifecycleScalars {
                remaining_lifetime_us_at_0x74: 3_500_000,
                raw_dword_at_0x80: 0x04B7_0001,
                anchor_raw_8_8_at_0x90: [-1024, 256, 2047],
            })
        );
        assert_eq!(decode_entity_lifecycle_scalars(&bytes[..0x95]), None);
    }

    #[test]
    fn behavior_masks_apply_to_live_entity_environment_flags() {
        assert_eq!(
            apply_behavior_environment_masks(0x4008, 0x0020, 0x0008),
            0x4020
        );
    }

    #[test]
    fn model_dynamic_callback_pair_requires_both_exact_dwords() {
        let bytes = [0x45, 0x23, 0x41, 0x00, 0x00, 0x60, 0x45, 0x23];
        assert_eq!(
            decode_model_dynamic_callback_pair(&bytes),
            Some(ModelDynamicCallbackPair {
                callback_address: 0x0041_2345,
                callback_data_value: 0x2345_6000,
            })
        );
        assert_eq!(decode_model_dynamic_callback_pair(&bytes[..7]), None);
    }

    #[test]
    fn component_pair_contact_uses_state_offsets_0x18_and_0x1c() {
        let mut bytes = [0_u8; 0x20];
        bytes[0x18..0x1C].copy_from_slice(&0x0040_A900_u32.to_le_bytes());
        bytes[0x1C..0x20].copy_from_slice(&0x1234_5678_u32.to_le_bytes());
        assert_eq!(
            decode_component_pair_contact(&bytes),
            (Some(0x0040_A900), Some(0x1234_5678))
        );
        assert_eq!(decode_component_pair_contact(&bytes[..0x1A]), (None, None));
    }

    #[test]
    fn behavior_callbacks_use_release_pair_and_death_slots() {
        let mut bytes = [0_u8; 0x30];
        bytes[0x0C..0x10].copy_from_slice(&0x0040_D1C0_u32.to_le_bytes());
        bytes[0x18..0x1C].copy_from_slice(&0x0042_5850_u32.to_le_bytes());
        bytes[0x2C..0x30].copy_from_slice(&0x0041_9750_u32.to_le_bytes());
        assert_eq!(
            decode_behavior_callbacks(&bytes),
            (Some(0x0040_D1C0), Some(0x0042_5850), Some(0x0041_9750))
        );
        assert_eq!(
            decode_behavior_callbacks(&bytes[..0x2F]),
            (Some(0x0040_D1C0), Some(0x0042_5850), None)
        );
    }

    #[test]
    fn pair_contact_sound_words_unpack_section_12_type_record_dwords() {
        assert_eq!(
            decode_pair_contact_sound_words(0x0016_000B, 0x002C_0021),
            [11, 22, 33, 44]
        );
    }

    #[test]
    fn type_vtable_callbacks_use_exact_recovered_offsets() {
        let mut bytes = vec![0_u8; TYPE_VTABLE_WINDOW_BYTES];
        bytes[0x20..0x24].copy_from_slice(&0x0041_1111_u32.to_le_bytes());
        bytes[0x24..0x28].copy_from_slice(&0x0041_2222_u32.to_le_bytes());
        bytes[0x28..0x2C].copy_from_slice(&0x0041_3333_u32.to_le_bytes());
        bytes[0x2C..0x30].copy_from_slice(&0x0041_4444_u32.to_le_bytes());
        bytes[0x30..0x34].copy_from_slice(&0x0041_4E90_u32.to_le_bytes());
        bytes[0x38..0x3C].copy_from_slice(&0x0040_D8D0_u32.to_le_bytes());
        bytes[0x48..0x4C].copy_from_slice(&0x0040_DC50_u32.to_le_bytes());
        let window = RemoteWindow {
            pointer: 0x004C_0000,
            requested_bytes: TYPE_VTABLE_WINDOW_BYTES,
            raw_hex: None,
            read_error: None,
        };
        let evidence = decode_type_vtable(Some(window.pointer), Some(window), Some(bytes));
        assert_eq!(evidence.pointer_at_type_record_0x7c, Some(0x004C_0000));
        assert_eq!(evidence.callback_at_vtable_0x20, Some(0x0041_1111));
        assert_eq!(evidence.callback_at_vtable_0x24, Some(0x0041_2222));
        assert_eq!(evidence.callback_at_vtable_0x28, Some(0x0041_3333));
        assert_eq!(evidence.callback_at_vtable_0x2c, Some(0x0041_4444));
        assert_eq!(evidence.hit_callback_at_vtable_0x30, Some(0x0041_4E90));
        assert_eq!(
            evidence.pair_dispatch_callback_at_vtable_0x38,
            Some(0x0040_D8D0)
        );
        assert_eq!(
            evidence.release_dispatch_callback_at_vtable_0x48,
            Some(0x0040_DC50)
        );

        let truncated = decode_type_vtable(
            Some(0x004C_0000),
            None,
            Some(vec![0_u8; TYPE_VTABLE_WINDOW_BYTES - 1]),
        );
        assert_eq!(truncated.hit_callback_at_vtable_0x30, Some(0));
        assert_eq!(truncated.pair_dispatch_callback_at_vtable_0x38, Some(0));
        assert_eq!(truncated.release_dispatch_callback_at_vtable_0x48, None);
    }

    #[test]
    fn wind_state_uses_the_bounded_retail_global_layout() {
        let mut bytes = [0_u8; WIND_STATE_BYTES];
        bytes[0x00..0x04].copy_from_slice(&(-75_i32).to_le_bytes());
        bytes[0x04..0x08].copy_from_slice(&2_u32.to_le_bytes());
        bytes[0x08..0x0A].copy_from_slice(&1500_i16.to_le_bytes());
        bytes[0x0A..0x0C].copy_from_slice(&(-400_i16).to_le_bytes());
        bytes[0x0C..0x0E].copy_from_slice(&(-3000_i16).to_le_bytes());
        bytes[0x10..0x14].copy_from_slice(&3_u32.to_le_bytes());
        bytes[0x14..0x16].copy_from_slice(&750_i16.to_le_bytes());
        bytes[0x16..0x18].copy_from_slice(&(-200_i16).to_le_bytes());
        bytes[0x18..0x1A].copy_from_slice(&(-1500_i16).to_le_bytes());

        let values = decode_wind_state(&bytes);
        assert_eq!(values.height_limit_raw_at_0x004f7194, Some(-75));
        assert_eq!(values.mode_at_0x004f7198, Some(2));
        assert_eq!(
            values.configured_vector_raw_at_0x004f719c,
            Some([1500, -400, -3000])
        );
        assert_eq!(values.drag_strength_at_0x004f71a4, Some(3));
        assert_eq!(
            values.active_vector_raw_at_0x004f71a8,
            Some([750, -200, -1500])
        );
        assert_eq!(
            decode_wind_state(&bytes[..WIND_STATE_BYTES - 1]).active_vector_raw_at_0x004f71a8,
            None
        );
    }
}
