use serde::Serialize;

use crate::controller::{locate_controller, SESSION_POINTER_GLOBAL};
use crate::entity::{EntityRecord, MODEL_POOL_PTR, TICK_50HZ};
use crate::process::{i16_at, plausible_heap_pointer, u16_at, u32_at, Process};

mod ride_surface;
pub use ride_surface::{
    read_nearby_static_collision_candidates, read_world_surface_probe, VtolRideSurfaceEvidence,
    WorldSurfaceProbeEvidence,
};

const ENTITY_TYPE_TABLE_PTR: usize = 0x004F_E650;
const WAVE_ENABLE: usize = 0x004F_ECE4;
const GLOBAL_DELTA_US: usize = 0x004D_04E4;
const SESSION_BYTE_OFFSET: usize = 0x296;
const TYPE_SUB_G_POINTER_OFFSET: usize = 0xE8;
const STATIC_SUB_G_BYTES: usize = 0x68;
const COMPONENT_TABLE_POINTER_OFFSET: usize = 0x0C;
const RUNTIME_SUB_G_POINTER_OFFSET: usize = 0x18;
const RUNTIME_SUB_G_BYTES: usize = 0x44;
const BODY_CONFIG_SELECTOR_OFFSET: usize = 0x198;
const MODE_CHANGE_REQUEST_OFFSET: usize = 0x20D;
const MODEL_RADIUS_OFFSET: usize = 0x08;
const MODEL_COLLISION_RADIUS_OFFSET: usize = 0x0A;
const FOCUSED_ENTITY_BYTES: usize = 0xB0;
const MAX_ATTACHMENTS: u32 = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct VtolControllerAddresses {
    body_config_selector: u32,
    mode_change_request: u32,
    fuel: u32,
    turbo_capability: u32,
}

fn vtol_controller_addresses(controller_pointer: u32) -> VtolControllerAddresses {
    VtolControllerAddresses {
        body_config_selector: controller_pointer.wrapping_add(BODY_CONFIG_SELECTOR_OFFSET as u32),
        mode_change_request: controller_pointer.wrapping_add(MODE_CHANGE_REQUEST_OFFSET as u32),
        fuel: controller_pointer.wrapping_add(0x88),
        turbo_capability: controller_pointer.wrapping_add(0x197),
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct HoverControllerConfig {
    pub pointer: u32,
    pub base_clearance_raw: u16,
    pub lift_range_raw: u16,
    pub strength_raw: u32,
    pub near_boost_range_raw: u16,
    pub damping_range_raw: u16,
    pub wave_flag: u8,
    pub body_offset_flag: u8,
    pub reserved: u16,
    pub raw_hex: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Attachment {
    pub handle: u32,
    pub entity_pointer: Option<u32>,
    pub entity_type: Option<u32>,
    pub mass: Option<u16>,
}

/// One bounded passive read used by the VTOL evidence probe. The address and
/// exact requested length travel with the bytes so a capture can prove which
/// allocation was sampled without assigning semantics to unknown fields.
#[derive(Debug, Clone, Serialize)]
pub struct VtolReadWindow {
    pub address: u32,
    pub requested_bytes: usize,
    pub raw_hex: Option<String>,
    pub read_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct VtolRuntimeValues {
    /// Post near-surface/turbo manual lift component consumed and attenuated
    /// by `FUN_0041B210`.
    pub manual_lift_at_0x20: Option<i32>,
    /// Altitude-assist magnitude projected after the attenuated manual-lift
    /// branch in `FUN_0041B210`; retail uses full body-up Y and one-eighth
    /// body-up X/Z while runtime `+0x43 == 0`.
    pub altitude_assist_at_0x24: Option<i32>,
    /// Runtime phase counter used by the lift-force branch gate.
    pub phase_counter_at_0x34: Option<i32>,
    /// `FUN_0041B8C0` seeds this dword from `Random_Next()` and authored
    /// Sub-G `+0x0C`; `FUN_0041B940` can replace it later.
    pub random_sample_at_0x38: Option<i32>,
    /// Turbo/blur state that selects the 0xC00 rather than 0x800 altitude
    /// attenuation threshold in `FUN_0041B210`.
    pub turbo_height_limit_flag_at_0x3d: Option<u8>,
    pub byte_at_0x3e: Option<u8>,
    pub byte_at_0x3f: Option<u8>,
    pub byte_at_0x40: Option<u8>,
    pub self_right_mode_at_0x41: Option<u8>,
    pub byte_at_0x42: Option<u8>,
    pub byte_at_0x43: Option<u8>,
}

/// Read-only evidence for the retail VTOL/heli integration path.
///
/// `FUN_00409A80` proves the runtime linkage as
/// `entity+0x4C -> root`, `root+0x0C -> component table`, and
/// `table+0x18 -> 0x44-byte Sub-G runtime allocation`. The authored 0x68-byte
/// Sub-G at `type_record+0xE8` is retained separately and must not be mistaken
/// for mutable per-entity state.
#[derive(Debug, Clone, Serialize)]
pub struct VtolRuntimeEvidence {
    pub tick_address: u32,
    pub tick_before: Option<u32>,
    pub tick_after: Option<u32>,
    pub tick_stable: Option<bool>,
    pub entity_pointer: u32,
    pub entity_component_root_slot_address: u32,
    pub entity_component_root_pointer: Option<u32>,
    pub component_root_window: Option<VtolReadWindow>,
    pub component_table_pointer_slot_address: Option<u32>,
    pub component_table_pointer: Option<u32>,
    pub component_table_window: Option<VtolReadWindow>,
    pub runtime_sub_g_pointer_slot_address: Option<u32>,
    pub runtime_sub_g_pointer: Option<u32>,
    pub runtime_sub_g_window: Option<VtolReadWindow>,
    #[serde(flatten)]
    pub runtime_values: VtolRuntimeValues,

    pub static_sub_g_pointer_slot_address: u32,
    pub static_sub_g_pointer: Option<u32>,
    pub static_sub_g_window: Option<VtolReadWindow>,
    pub authored_random_base_raw_at_static_sub_g_0x0c: Option<i16>,

    pub session_pointer_address: u32,
    pub session_pointer: Option<u32>,
    pub session_byte_address_at_0x296: Option<u32>,
    pub session_byte_at_0x296: Option<u8>,
    pub global_delta_us_address: u32,
    pub global_delta_us: Option<u32>,
    /// `FUN_00446640::param_3` is a transient stack argument. A passive
    /// ReadProcessMemory probe has no proven stable address from which to read
    /// it, so keep the gate explicit instead of copying the global delta and
    /// calling that proof.
    pub dispatch_delta_us: Option<u32>,
    pub dispatch_delta_status: &'static str,

    /// Player-controller allocation selected by `FUN_00443AF0`. The fly
    /// callback receives this object as param_1; these offsets do not belong
    /// to the separate 0xCC-byte world entity reached through controller+0x68.
    pub controller_pointer: Option<u32>,
    pub body_config_selector_address_at_controller_0x198: Option<u32>,
    pub body_config_selector_at_controller_0x198: Option<u8>,
    pub mode_change_request_address_at_controller_0x20d: Option<u32>,
    pub mode_change_request_at_controller_0x20d: Option<u8>,
    pub fuel_address_at_controller_0x88: Option<u32>,
    pub fuel_at_controller_0x88: Option<i32>,
    pub type_flags_address_at_entity_0x64: u32,
    pub type_flags_at_entity_0x64: Option<u32>,
    pub turbo_capability_address_at_controller_0x197: Option<u32>,
    pub turbo_capability_flags_at_controller_0x197: Option<u8>,
    pub turbo_capability_bit_1: Option<bool>,
    pub model_pool_pointer_address: u32,
    pub model_pool_pointer: Option<u32>,
    pub focused_active_model_slot: Option<usize>,
    pub focused_active_model_id: Option<u16>,
    pub focused_active_model_resource_pointer: Option<u32>,
    /// Section-8 `radius` at +0x08. `FUN_00445310` subtracts half this word
    /// from the sampled ride surface before deriving its altitude error.
    pub active_model_radius_raw_at_0x08: Option<u16>,
    pub vtol_clearance_half_radius_raw: Option<u16>,
    /// Separate Section-8 broad-phase collision radius at +0x0A.
    pub active_model_collision_radius_raw_at_0x0a: Option<u16>,
    /// Narrow tick-bracketed terrain/water sample used to measure ascent
    /// ceilings and ground/water penetration without invoking game callbacks.
    pub ride_surface: VtolRideSurfaceEvidence,
    /// Basis re-read inside this evidence window's tick bracket.
    pub focused_body_basis_raw: Option<[i32; 9]>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlayerPhysicsContext {
    pub wave_enable_global: u32,
    pub entity_type_table: u32,
    pub type_record: u32,
    pub hover_controller: HoverControllerConfig,
    pub attachment_owner: u32,
    pub attachment_state: u32,
    pub attachment_list: u32,
    pub attachment_entries: u32,
    pub attachments: Vec<Attachment>,
    pub self_mass: u16,
    pub attached_mass: u32,
    pub total_mass: u32,
    pub hover_targets_wave: bool,
    pub full_buoyancy_regime: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vtol_runtime: Option<VtolRuntimeEvidence>,
    pub warnings: Vec<String>,
}

pub fn read(
    process: &Process,
    player: &EntityRecord,
    entities: &[EntityRecord],
    capture_vtol_evidence: bool,
) -> Result<PlayerPhysicsContext, String> {
    let wave_enable_global = process.read_u32(WAVE_ENABLE)?;
    let entity_type_table = process.read_u32(ENTITY_TYPE_TABLE_PTR)?;
    require_pointer(entity_type_table, "entity type table")?;
    let type_record = process.read_u32(
        entity_type_table as usize + player.entity_type as usize * std::mem::size_of::<u32>(),
    )?;
    require_pointer(type_record, "player type record")?;
    let hover_pointer = process.read_u32(type_record as usize + 0xD8)?;
    require_pointer(hover_pointer, "Section-12 Sub-C hover controller")?;
    let hover_bytes = process.read_bytes(hover_pointer as usize, 16)?;
    let hover_controller = HoverControllerConfig {
        pointer: hover_pointer,
        base_clearance_raw: u16_at(&hover_bytes, 0x00),
        lift_range_raw: u16_at(&hover_bytes, 0x02),
        strength_raw: u32_at(&hover_bytes, 0x04),
        near_boost_range_raw: u16_at(&hover_bytes, 0x08),
        damping_range_raw: u16_at(&hover_bytes, 0x0A),
        wave_flag: hover_bytes[0x0C],
        body_offset_flag: hover_bytes[0x0D],
        reserved: u16_at(&hover_bytes, 0x0E),
        raw_hex: encode_hex(&hover_bytes),
    };

    let mut warnings = Vec::new();
    let mut attachments = Vec::new();
    let attachment_owner = process.read_u32(player.pointer as usize + 0x4C)?;
    let mut attachment_state = 0;
    let mut attachment_list = 0;
    let mut attachment_entries = 0;
    if attachment_owner != 0 {
        if plausible_heap_pointer(attachment_owner as usize) {
            attachment_state = process.read_u32(attachment_owner as usize + 0x0C)?;
        } else {
            warnings.push(format!(
                "implausible attachment owner {attachment_owner:08X}"
            ));
        }
    }
    if attachment_state != 0 {
        if plausible_heap_pointer(attachment_state as usize) {
            attachment_list = process.read_u32(attachment_state as usize + 0x14)?;
        } else {
            warnings.push(format!(
                "implausible attachment state {attachment_state:08X}"
            ));
        }
    }
    if attachment_list != 0 {
        if plausible_heap_pointer(attachment_list as usize) {
            attachment_entries = process.read_u32(attachment_list as usize)?;
            let count = process.read_u32(attachment_list as usize + 4)?;
            if count > MAX_ATTACHMENTS {
                warnings.push(format!(
                    "attachment count {count} exceeds safety limit {MAX_ATTACHMENTS}"
                ));
            } else if count != 0 && plausible_heap_pointer(attachment_entries as usize) {
                for index in 0..count {
                    let handle =
                        process.read_u32(attachment_entries as usize + index as usize * 12 + 8)?;
                    let entity = entities.iter().find(|entity| entity.handle == handle);
                    if entity.is_none() {
                        warnings.push(format!(
                            "attachment handle {handle:08X} has no entity-list match"
                        ));
                    }
                    attachments.push(Attachment {
                        handle,
                        entity_pointer: entity.map(|entity| entity.pointer),
                        entity_type: entity.map(|entity| entity.entity_type),
                        mass: entity.map(|entity| entity.self_mass),
                    });
                }
            } else if count != 0 {
                warnings.push(format!(
                    "implausible attachment entries pointer {attachment_entries:08X}"
                ));
            }
        } else {
            warnings.push(format!("implausible attachment list {attachment_list:08X}"));
        }
    }

    let attached_mass: u32 = attachments
        .iter()
        .filter_map(|attachment| attachment.mass)
        .map(u32::from)
        .sum();
    let total_mass = u32::from(player.self_mass) + attached_mass;
    let vtol_runtime =
        capture_vtol_evidence.then(|| read_vtol_runtime(process, player, type_record));
    Ok(PlayerPhysicsContext {
        wave_enable_global,
        entity_type_table,
        type_record,
        hover_targets_wave: wave_enable_global != 0
            && hover_controller.wave_flag != 0
            && attached_mass <= 99,
        full_buoyancy_regime: total_mass < 151,
        vtol_runtime,
        hover_controller,
        attachment_owner,
        attachment_state,
        attachment_list,
        attachment_entries,
        attachments,
        self_mass: player.self_mass,
        attached_mass,
        total_mass,
        warnings,
    })
}

fn read_vtol_runtime(
    process: &Process,
    player: &EntityRecord,
    type_record: u32,
) -> VtolRuntimeEvidence {
    let mut warnings = Vec::new();
    let tick_before = read_u32_optional(
        process,
        TICK_50HZ as u32,
        "VTOL evidence start tick",
        &mut warnings,
    );
    let focused_entity_bytes =
        match process.read_bytes(player.pointer as usize, FOCUSED_ENTITY_BYTES) {
            Ok(bytes) => Some(bytes),
            Err(error) => {
                warnings.push(format!(
                    "focused entity window at {:08X} is unreadable: {error}",
                    player.pointer
                ));
                None
            }
        };
    let focused_entity = focused_entity_bytes
        .as_deref()
        .and_then(decode_focused_entity);

    let static_sub_g_pointer_slot_address =
        type_record.wrapping_add(TYPE_SUB_G_POINTER_OFFSET as u32);
    let static_sub_g_pointer = read_u32_optional(
        process,
        static_sub_g_pointer_slot_address,
        "static Sub-G pointer",
        &mut warnings,
    );
    let (static_sub_g_window, static_sub_g_bytes) = static_sub_g_pointer
        .map(|pointer| {
            read_pointer_window(
                process,
                pointer,
                STATIC_SUB_G_BYTES,
                "static Sub-G",
                &mut warnings,
            )
        })
        .unwrap_or((None, None));
    let authored_random_base_raw_at_static_sub_g_0x0c = static_sub_g_bytes
        .as_deref()
        .and_then(|bytes| bytes.get(0x0C..0x0E))
        .map(|bytes| i16_at(bytes, 0));

    let entity_component_root_pointer = focused_entity.map(|state| state.component_root);
    let component_root_window =
        entity_component_root_pointer.map(|pointer| read_window(process, pointer, 0x18));
    let component_table_pointer_slot_address = entity_component_root_pointer
        .filter(|pointer| plausible_heap_pointer(*pointer as usize))
        .map(|pointer| pointer.wrapping_add(COMPONENT_TABLE_POINTER_OFFSET as u32));
    let component_table_pointer = component_table_pointer_slot_address.and_then(|address| {
        read_u32_optional(process, address, "component table pointer", &mut warnings)
    });
    let (component_table_window, component_table_bytes) = component_table_pointer
        .map(|pointer| {
            read_pointer_window(process, pointer, 0x40, "component table", &mut warnings)
        })
        .unwrap_or((None, None));
    let runtime_sub_g_pointer_slot_address = component_table_pointer
        .map(|pointer| pointer.wrapping_add(RUNTIME_SUB_G_POINTER_OFFSET as u32));
    let runtime_sub_g_pointer = component_table_bytes
        .as_deref()
        .and_then(|bytes| bytes.get(RUNTIME_SUB_G_POINTER_OFFSET..RUNTIME_SUB_G_POINTER_OFFSET + 4))
        .map(|bytes| u32_at(bytes, 0));
    let (runtime_sub_g_window, runtime_sub_g_bytes) = runtime_sub_g_pointer
        .map(|pointer| {
            read_pointer_window(
                process,
                pointer,
                RUNTIME_SUB_G_BYTES,
                "runtime Sub-G",
                &mut warnings,
            )
        })
        .unwrap_or((None, None));
    let runtime_values = decode_runtime_sub_g(runtime_sub_g_bytes.as_deref().unwrap_or_default());

    let session_pointer = read_u32_optional(
        process,
        SESSION_POINTER_GLOBAL as u32,
        "session pointer",
        &mut warnings,
    );
    let session_byte_address_at_0x296 = session_pointer
        .filter(|pointer| plausible_heap_pointer(*pointer as usize))
        .map(|pointer| pointer.wrapping_add(SESSION_BYTE_OFFSET as u32));
    let session_byte_at_0x296 = session_byte_address_at_0x296.and_then(|address| {
        read_u8_optional(process, address, "session byte +0x296", &mut warnings)
    });
    let global_delta_us = read_u32_optional(
        process,
        GLOBAL_DELTA_US as u32,
        "global delta microseconds",
        &mut warnings,
    );

    // FUN_00446640 resolves this allocation with FUN_00443AF0, then passes it
    // as FUN_00445310::param_1. Reading the same offsets from the world entity
    // produced plausible-looking but unrelated pointer/state bytes in earlier
    // captures (most visibly entity+0x88 equalling the controller pointer).
    let controller = locate_controller(process, Some(player.handle), &mut warnings);
    let controller_pointer = controller.map(|location| location.controller_pointer);
    let controller_addresses = controller_pointer.map(vtol_controller_addresses);
    let body_config_selector_address_at_controller_0x198 =
        controller_addresses.map(|addresses| addresses.body_config_selector);
    let body_config_selector_at_controller_0x198 = body_config_selector_address_at_controller_0x198
        .and_then(|address| {
            read_u8_optional(
                process,
                address,
                "controller body/config selector",
                &mut warnings,
            )
        });
    let mode_change_request_address_at_controller_0x20d =
        controller_addresses.map(|addresses| addresses.mode_change_request);
    let mode_change_request_at_controller_0x20d = mode_change_request_address_at_controller_0x20d
        .and_then(|address| {
            read_u8_optional(
                process,
                address,
                "controller mode-change request",
                &mut warnings,
            )
        });
    let fuel_address_at_controller_0x88 = controller_addresses.map(|addresses| addresses.fuel);
    let fuel_at_controller_0x88 = fuel_address_at_controller_0x88
        .and_then(|address| {
            read_u32_optional(process, address, "VTOL controller fuel", &mut warnings)
        })
        .map(|value| value as i32);
    let type_flags_address_at_entity_0x64 = player.pointer.wrapping_add(0x64);
    let type_flags_at_entity_0x64 = read_u32_optional(
        process,
        type_flags_address_at_entity_0x64,
        "VTOL type flags",
        &mut warnings,
    );
    let turbo_capability_address_at_controller_0x197 =
        controller_addresses.map(|addresses| addresses.turbo_capability);
    let turbo_capability_flags_at_controller_0x197 = turbo_capability_address_at_controller_0x197
        .and_then(|address| {
            read_u8_optional(
                process,
                address,
                "VTOL controller turbo flags",
                &mut warnings,
            )
        });
    let model_pool_pointer = read_u32_optional(
        process,
        MODEL_POOL_PTR as u32,
        "model pool pointer",
        &mut warnings,
    );
    let focused_active_model_resource_pointer = focused_entity.and_then(|state| {
        let pool =
            model_pool_pointer.filter(|pointer| plausible_heap_pointer(*pointer as usize))?;
        read_u32_optional(
            process,
            pool.wrapping_add(u32::from(state.active_model_id) * 4),
            "focused active model resource pointer",
            &mut warnings,
        )
    });
    let (active_model_radius_raw_at_0x08, active_model_collision_radius_raw_at_0x0a) =
        if let Some(resource) = focused_active_model_resource_pointer
            .filter(|pointer| plausible_heap_pointer(*pointer as usize))
        {
            match process.read_bytes(resource as usize, 0x0C) {
                Ok(header) => decode_model_radii(&header),
                Err(error) => {
                    warnings.push(format!(
                        "active model radii at {:08X}+0x08/+0x0A are unreadable: {error}",
                        resource
                    ));
                    (None, None)
                }
            }
        } else {
            warnings
                .push("focused active model resource pointer is absent or implausible".to_owned());
            (None, None)
        };
    let mut ride_surface = ride_surface::read(ride_surface::VtolRideSurfaceRequest {
        process,
        entity_pointer: player.pointer,
        active_model_half_radius_raw: active_model_radius_raw_at_0x08.map(|radius| radius >> 1),
        active_model_collision_radius_raw: active_model_collision_radius_raw_at_0x0a,
        type_flags_at_entity_0x64,
        runtime: &runtime_values,
        game_mode_signed_at_session_0x296: session_byte_at_0x296.map(|byte| byte as i8),
        warnings: &mut warnings,
    });
    let tick_after = read_u32_optional(
        process,
        TICK_50HZ as u32,
        "VTOL evidence end tick",
        &mut warnings,
    );
    let tick_stable = tick_before
        .zip(tick_after)
        .map(|(before, after)| before == after);
    ride_surface::set_outer_tick_stability(&mut ride_surface, tick_stable);

    VtolRuntimeEvidence {
        tick_address: TICK_50HZ as u32,
        tick_before,
        tick_after,
        tick_stable,
        entity_pointer: player.pointer,
        entity_component_root_slot_address: player.pointer.wrapping_add(0x4C),
        entity_component_root_pointer,
        component_root_window,
        component_table_pointer_slot_address,
        component_table_pointer,
        component_table_window,
        runtime_sub_g_pointer_slot_address,
        runtime_sub_g_pointer,
        runtime_sub_g_window,
        runtime_values,
        static_sub_g_pointer_slot_address,
        static_sub_g_pointer,
        static_sub_g_window,
        authored_random_base_raw_at_static_sub_g_0x0c,
        session_pointer_address: SESSION_POINTER_GLOBAL as u32,
        session_pointer,
        session_byte_address_at_0x296,
        session_byte_at_0x296,
        global_delta_us_address: GLOBAL_DELTA_US as u32,
        global_delta_us,
        dispatch_delta_us: None,
        dispatch_delta_status: "unavailable_read_only: FUN_00446640::param_3 is a transient stack argument with no proven passive address",
        controller_pointer,
        body_config_selector_address_at_controller_0x198,
        body_config_selector_at_controller_0x198,
        mode_change_request_address_at_controller_0x20d,
        mode_change_request_at_controller_0x20d,
        fuel_address_at_controller_0x88,
        fuel_at_controller_0x88,
        type_flags_address_at_entity_0x64,
        type_flags_at_entity_0x64,
        turbo_capability_address_at_controller_0x197,
        turbo_capability_flags_at_controller_0x197,
        turbo_capability_bit_1: turbo_capability_flags_at_controller_0x197
            .map(|flags| flags & 2 != 0),
        model_pool_pointer_address: MODEL_POOL_PTR as u32,
        model_pool_pointer,
        focused_active_model_slot: focused_entity.map(|state| state.active_slot),
        focused_active_model_id: focused_entity.map(|state| state.active_model_id),
        focused_active_model_resource_pointer,
        active_model_radius_raw_at_0x08,
        vtol_clearance_half_radius_raw: active_model_radius_raw_at_0x08
            .map(|radius| radius >> 1),
        active_model_collision_radius_raw_at_0x0a,
        ride_surface,
        focused_body_basis_raw: focused_entity.map(|state| state.body_basis_raw),
        warnings,
    }
}

fn decode_runtime_sub_g(bytes: &[u8]) -> VtolRuntimeValues {
    VtolRuntimeValues {
        manual_lift_at_0x20: bounded_i32(bytes, 0x20),
        altitude_assist_at_0x24: bounded_i32(bytes, 0x24),
        phase_counter_at_0x34: bounded_i32(bytes, 0x34),
        random_sample_at_0x38: bounded_i32(bytes, 0x38),
        turbo_height_limit_flag_at_0x3d: bytes.get(0x3D).copied(),
        byte_at_0x3e: bytes.get(0x3E).copied(),
        byte_at_0x3f: bytes.get(0x3F).copied(),
        byte_at_0x40: bytes.get(0x40).copied(),
        self_right_mode_at_0x41: bytes.get(0x41).copied(),
        byte_at_0x42: bytes.get(0x42).copied(),
        byte_at_0x43: bytes.get(0x43).copied(),
    }
}

fn bounded_i32(bytes: &[u8], offset: usize) -> Option<i32> {
    let bytes = bytes.get(offset..offset.checked_add(4)?)?;
    Some(i32::from_le_bytes(bytes.try_into().ok()?))
}

fn decode_model_radii(header: &[u8]) -> (Option<u16>, Option<u16>) {
    let decode = |offset: usize| {
        header
            .get(offset..offset + 2)
            .and_then(|bytes| bytes.try_into().ok())
            .map(u16::from_le_bytes)
    };
    (
        decode(MODEL_RADIUS_OFFSET),
        decode(MODEL_COLLISION_RADIUS_OFFSET),
    )
}

fn decode_body_basis(bytes: &[u8]) -> Option<[i32; 9]> {
    (bytes.len() >= 9 * 4).then(|| {
        std::array::from_fn(|index| {
            let offset = index * 4;
            i32::from_le_bytes(bytes[offset..offset + 4].try_into().expect("bounded basis"))
        })
    })
}

#[derive(Clone, Copy)]
struct FocusedEntityState {
    component_root: u32,
    body_basis_raw: [i32; 9],
    active_slot: usize,
    active_model_id: u16,
}

fn decode_focused_entity(bytes: &[u8]) -> Option<FocusedEntityState> {
    (bytes.len() >= FOCUSED_ENTITY_BYTES).then(|| {
        let flags = u32_at(bytes, 0x08);
        let active_slot = usize::from(flags & 0x2000 != 0) * 2 + usize::from(flags & 0x4000 != 0);
        FocusedEntityState {
            component_root: u32_at(bytes, 0x4C),
            body_basis_raw: decode_body_basis(&bytes[0x0C..0x0C + 9 * 4])
                .expect("bounded focused entity basis"),
            active_slot,
            active_model_id: u16_at(bytes, 0xA8 + active_slot * 2),
        }
    })
}

fn read_window(process: &Process, address: u32, requested_bytes: usize) -> VtolReadWindow {
    read_window_with_bytes(process, address, requested_bytes).0
}

fn read_window_with_bytes(
    process: &Process,
    address: u32,
    requested_bytes: usize,
) -> (VtolReadWindow, Option<Vec<u8>>) {
    if address == 0 || !plausible_heap_pointer(address as usize) {
        return (
            VtolReadWindow {
                address,
                requested_bytes,
                raw_hex: None,
                read_error: Some(format!(
                    "pointer {address:08X} is outside the bounded aligned read range"
                )),
            },
            None,
        );
    }
    match process.read_bytes(address as usize, requested_bytes) {
        Ok(bytes) => {
            let raw_hex = encode_hex(&bytes);
            (
                VtolReadWindow {
                    address,
                    requested_bytes,
                    raw_hex: Some(raw_hex),
                    read_error: None,
                },
                Some(bytes),
            )
        }
        Err(error) => (
            VtolReadWindow {
                address,
                requested_bytes,
                raw_hex: None,
                read_error: Some(error),
            },
            None,
        ),
    }
}

fn read_pointer_window(
    process: &Process,
    pointer: u32,
    requested_bytes: usize,
    label: &str,
    warnings: &mut Vec<String>,
) -> (Option<VtolReadWindow>, Option<Vec<u8>>) {
    let (window, bytes) = read_window_with_bytes(process, pointer, requested_bytes);
    if let Some(error) = &window.read_error {
        warnings.push(format!("{label} {pointer:08X} is unreadable: {error}"));
    }
    (Some(window), bytes)
}

fn read_u32_optional(
    process: &Process,
    address: u32,
    label: &str,
    warnings: &mut Vec<String>,
) -> Option<u32> {
    match process.read_u32(address as usize) {
        Ok(value) => Some(value),
        Err(error) => {
            warnings.push(format!("{label} at {address:08X} is unreadable: {error}"));
            None
        }
    }
}

fn read_u8_optional(
    process: &Process,
    address: u32,
    label: &str,
    warnings: &mut Vec<String>,
) -> Option<u8> {
    match process.read_bytes(address as usize, 1) {
        Ok(bytes) => bytes.first().copied(),
        Err(error) => {
            warnings.push(format!("{label} at {address:08X} is unreadable: {error}"));
            None
        }
    }
}

fn require_pointer(pointer: u32, label: &str) -> Result<(), String> {
    if plausible_heap_pointer(pointer as usize) {
        Ok(())
    } else {
        Err(format!("{label} pointer {pointer:08X} is implausible"))
    }
}

fn encode_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    use std::fmt::Write as _;
    for byte in bytes {
        let _ = write!(output, "{byte:02X}");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::{
        decode_body_basis, decode_focused_entity, decode_model_radii, decode_runtime_sub_g,
        vtol_controller_addresses, FOCUSED_ENTITY_BYTES, RUNTIME_SUB_G_BYTES,
    };

    #[test]
    fn vtol_controller_fields_are_based_on_the_controller_allocation() {
        let addresses = vtol_controller_addresses(0x1000_0000);
        assert_eq!(addresses.fuel, 0x1000_0088);
        assert_eq!(addresses.turbo_capability, 0x1000_0197);
        assert_eq!(addresses.body_config_selector, 0x1000_0198);
        assert_eq!(addresses.mode_change_request, 0x1000_020D);
    }

    #[test]
    fn model_radius_and_collision_radius_remain_distinct() {
        let mut header = [0_u8; 0x0C];
        header[0x08..0x0A].copy_from_slice(&111_u16.to_le_bytes());
        header[0x0A..0x0C].copy_from_slice(&222_u16.to_le_bytes());
        assert_eq!(decode_model_radii(&header), (Some(111), Some(222)));
        assert_eq!(decode_model_radii(&header[..0x0B]), (Some(111), None));
    }

    #[test]
    fn focused_basis_decode_requires_all_nine_dwords() {
        let mut bytes = [0_u8; 9 * 4];
        for index in 0..9 {
            bytes[index * 4..index * 4 + 4].copy_from_slice(&(index as i32 - 4).to_le_bytes());
        }
        assert_eq!(
            decode_body_basis(&bytes),
            Some([-4, -3, -2, -1, 0, 1, 2, 3, 4])
        );
        assert_eq!(decode_body_basis(&bytes[..35]), None);
    }

    #[test]
    fn focused_entity_selects_the_active_model_inside_the_tick_bracket() {
        let mut bytes = [0_u8; FOCUSED_ENTITY_BYTES];
        bytes[0x08..0x0C].copy_from_slice(&0x4000_u32.to_le_bytes());
        bytes[0x4C..0x50].copy_from_slice(&0x1234_5000_u32.to_le_bytes());
        bytes[0xAA..0xAC].copy_from_slice(&67_u16.to_le_bytes());
        for index in 0..9 {
            bytes[0x0C + index * 4..0x10 + index * 4]
                .copy_from_slice(&(index as i32).to_le_bytes());
        }

        let state = decode_focused_entity(&bytes).expect("complete entity window");
        assert_eq!(state.component_root, 0x1234_5000);
        assert_eq!(state.active_slot, 1);
        assert_eq!(state.active_model_id, 67);
        assert_eq!(state.body_basis_raw, [0, 1, 2, 3, 4, 5, 6, 7, 8]);
        assert!(decode_focused_entity(&bytes[..FOCUSED_ENTITY_BYTES - 1]).is_none());
    }

    #[test]
    fn runtime_sub_g_decode_requires_the_exact_tail_bytes() {
        let mut bytes = [0_u8; RUNTIME_SUB_G_BYTES];
        bytes[0x20..0x24].copy_from_slice(&12_345_i32.to_le_bytes());
        bytes[0x24..0x28].copy_from_slice(&(-678_i32).to_le_bytes());
        bytes[0x34..0x38].copy_from_slice(&199_i32.to_le_bytes());
        bytes[0x38..0x3C].copy_from_slice(&(-12_345_i32).to_le_bytes());
        bytes[0x3D] = 1;
        bytes[0x3E..0x43].copy_from_slice(&[1, 2, 3, 4, 5]);

        let values = decode_runtime_sub_g(&bytes);
        assert_eq!(values.manual_lift_at_0x20, Some(12_345));
        assert_eq!(values.altitude_assist_at_0x24, Some(-678));
        assert_eq!(values.phase_counter_at_0x34, Some(199));
        assert_eq!(values.random_sample_at_0x38, Some(-12_345));
        assert_eq!(values.turbo_height_limit_flag_at_0x3d, Some(1));
        assert_eq!(values.byte_at_0x3e, Some(1));
        assert_eq!(values.byte_at_0x3f, Some(2));
        assert_eq!(values.byte_at_0x40, Some(3));
        assert_eq!(values.self_right_mode_at_0x41, Some(4));
        assert_eq!(values.byte_at_0x42, Some(5));
        assert_eq!(values.byte_at_0x43, Some(0));

        let truncated = decode_runtime_sub_g(&bytes[..0x3B]);
        assert_eq!(truncated.manual_lift_at_0x20, Some(12_345));
        assert_eq!(truncated.random_sample_at_0x38, None);
        assert_eq!(truncated.byte_at_0x3e, None);
    }
}
