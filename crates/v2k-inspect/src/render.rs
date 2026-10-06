use serde::Serialize;

use crate::controller::SESSION_POINTER_GLOBAL;
use crate::entity::TICK_50HZ;
use crate::process::{i16_at, plausible_heap_pointer, u32_at, Process};
use crate::resources::resolve_resource_handle;

const CONTEXT_FOG_NEAR: usize = 0x004D_055C;
const CONTEXT_FOG_FAR: usize = 0x004D_0560;
const ACTIVE_FOG_NEAR: usize = 0x004F_EEEC;
const ACTIVE_FOG_FAR: usize = 0x004F_EEF0;
const ACTIVE_FOG_SCALE: usize = 0x004F_EEE8;
const VIEW_TRANSFORM: usize = 0x004F_EEA0;
const TERRAIN_SCAN_COLUMNS: usize = 0x004C_AB70;
const TERRAIN_SCAN_ROWS: usize = 0x004C_AB74;
const WATER_SHORELINE_BASE: usize = 0x004D_B26C;
const FRAME_DELTA_US: usize = 0x004D_04E4;
const ACTIVE_CAMERA_SETTING: usize = 0x004C_B408;
const CAMERA_PARAM3_CONTROLLER: usize = 0x004F_71C0;
const CAMERA_PARAM4_CONTROLLER: usize = 0x004F_71C8;
const CONTROLLER_CURRENT_VALUE_OFFSET: usize = 0x18;
const SESSION_CAMERA_TARGET_HANDLE_OFFSET: usize = 0x308;
const CAMERA_TARGET_ENTITY_BYTES: usize = 0xA2;

/// Contiguous retail camera output, spring, and tracked-entity cache from
/// `DAT_004DAEE0` through the final word of the cached 3x3 body basis.
const CAMERA_BLOCK: usize = 0x004D_AEE0;
const CAMERA_BLOCK_BYTES: usize = 0xE4;

const OUTPUT_VIEW_BASIS_OFFSET: usize = 0x00;
const OUTPUT_EYE_OFFSET: usize = 0x30;
const SPRING_STATE_OFFSET: usize = 0x38;
const FOCUS_POSITION_OFFSET: usize = SPRING_STATE_OFFSET;
const FOCUS_VELOCITY_OFFSET: usize = SPRING_STATE_OFFSET + 0x06;
const FOCUS_INITIALIZED_OFFSET: usize = SPRING_STATE_OFFSET + 0x28;
const EYE_POSITION_OFFSET: usize = SPRING_STATE_OFFSET + 0x2C;
const EYE_VELOCITY_OFFSET: usize = EYE_POSITION_OFFSET + 0x06;
const EYE_INITIALIZED_OFFSET: usize = SPRING_STATE_OFFSET + 0x54;
const GENERATED_VIEW_BASIS_OFFSET: usize = 0x90;
const ATTACHMENT_HANDLE_OFFSET: usize = 0xB4;
const TRACKED_POSITION_OFFSET: usize = 0xB8;
const TRACKED_BODY_BASIS_OFFSET: usize = 0xC0;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CameraSpringSnapshot {
    /// Eye origin installed for the world renderer, signed 8.8 words.
    pub output_eye_raw: [i16; 3],
    /// View basis copied to `DAT_004DAEE0` after `FUN_0040F3A0`.
    pub output_view_basis_raw: [i32; 9],
    /// Lossless 0x7C-byte `FUN_0040F3A0` state as 31 little-endian dwords.
    pub spring_state_raw: [u32; 31],
    /// First `FUN_0040F3A0` spring: the look-at/focus point.
    pub focus_position_raw: [i16; 3],
    pub focus_velocity_raw: [i16; 3],
    pub focus_initialized: u32,
    /// Second `FUN_0040F3A0` spring: the camera eye.
    pub eye_position_raw: [i16; 3],
    pub eye_velocity_raw: [i16; 3],
    pub eye_initialized: u32,
    /// Basis generated from the two sprung points before the output copy.
    pub generated_view_basis_raw: [i32; 9],
    /// Handle transformed by the post-camera attachment tail, if any.
    pub attachment_handle: u32,
    /// Pose cached from the entity passed to `FUN_0040ED10` at callback entry.
    pub tracked_position_raw: [i16; 3],
    pub tracked_body_basis_raw: [i32; 9],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct CameraParameterSnapshot {
    pub controller_pointer: u32,
    /// `FUN_004715E0` returns controller `+0x18`. `None` means the controller
    /// pointer was null or transiently unreadable during a mode handoff.
    pub current_raw: Option<i32>,
}

/// Subject selected by Intro2's Section-2 camera operation. This is distinct
/// from `camera.tracked_position_raw`, which is the hidden class-1 proxy passed
/// to `FUN_0040ED10` after `FUN_004503C0` has begun following this subject.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CameraTargetSnapshot {
    pub session_pointer: Option<u32>,
    pub target_handle_at_context_0x308: Option<u32>,
    pub entity_pointer: Option<u32>,
    pub entity_type_at_0x58: Option<u32>,
    pub entity_handle_at_0x5c: Option<u32>,
    pub position_raw_8_8: Option<[i16; 3]>,
    pub velocity_raw_8_8: Option<[i16; 3]>,
    /// An absent target handle is valid outside Intro2. A non-null handle that
    /// cannot be resolved or validated remains explicit here without dropping
    /// the surrounding 200-Hz render sample.
    pub resolution_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RenderSnapshot {
    pub tick_before: u32,
    pub tick_after: u32,
    pub tick_stable: bool,
    /// Stored world render-context planes, signed 8.8 fixed point.
    pub context_fog_near_raw: i32,
    pub context_fog_far_raw: i32,
    /// Planes currently installed in FUN_00470A10.
    pub active_fog_near_raw: i32,
    pub active_fog_far_raw: i32,
    pub active_fog_scale_raw: i32,
    pub active_fog_near_cells: f32,
    pub active_fog_far_cells: f32,
    /// 3x3 fixed-point view basis followed by its three translations.
    pub view_transform_raw: [i32; 12],
    pub terrain_scan_columns: i32,
    pub terrain_scan_rows: i32,
    pub water_shoreline_base: i32,
    /// Incoming retail frame delta in microseconds. `FUN_0044FFA0` passes
    /// `min(frame_delta_us, 0x1E848)` to the camera callback.
    pub frame_delta_us: i32,
    /// Settings block `+0x30`, consumed by `FUN_0040ED10`'s distance formula.
    pub active_camera_setting: i32,
    /// True only when two complete reads of the camera block were identical.
    pub camera_duplicate_stable: bool,
    /// True only when duplicate reads of both dynamic ED10 parameters agree.
    pub camera_parameters_duplicate_stable: bool,
    /// True only when duplicate subject identity/pose reads agree. A false
    /// sample at an authored cut is retained rather than silently blended.
    pub camera_target_duplicate_stable: bool,
    /// Runtime values passed as `FUN_0040ED10` parameters 3 and 4.
    pub camera_param3: CameraParameterSnapshot,
    pub camera_param4: CameraParameterSnapshot,
    pub camera_target: CameraTargetSnapshot,
    pub camera: CameraSpringSnapshot,
}

pub fn read_snapshot(process: &Process) -> Result<RenderSnapshot, String> {
    let tick_before = process.read_u32(TICK_50HZ)?;
    let camera_before = process.read_bytes(CAMERA_BLOCK, CAMERA_BLOCK_BYTES)?;
    let camera_param3_before = read_camera_parameter(process, CAMERA_PARAM3_CONTROLLER)?;
    let camera_param4_before = read_camera_parameter(process, CAMERA_PARAM4_CONTROLLER)?;
    let camera_target_before = read_camera_target(process);
    let context_fog_near_raw = process.read_i32(CONTEXT_FOG_NEAR)?;
    let context_fog_far_raw = process.read_i32(CONTEXT_FOG_FAR)?;
    let active_fog_near_raw = process.read_i32(ACTIVE_FOG_NEAR)?;
    let active_fog_far_raw = process.read_i32(ACTIVE_FOG_FAR)?;
    let active_fog_scale_raw = process.read_i32(ACTIVE_FOG_SCALE)?;
    let mut view_transform_raw = [0; 12];
    for (index, value) in view_transform_raw.iter_mut().enumerate() {
        *value = process.read_i32(VIEW_TRANSFORM + index * 4)?;
    }
    let terrain_scan_columns = process.read_i32(TERRAIN_SCAN_COLUMNS)?;
    let terrain_scan_rows = process.read_i32(TERRAIN_SCAN_ROWS)?;
    let water_shoreline_base = process.read_i32(WATER_SHORELINE_BASE)?;
    let frame_delta_us = process.read_i32(FRAME_DELTA_US)?;
    let active_camera_setting = process.read_i32(ACTIVE_CAMERA_SETTING)?;
    let camera_param3 = read_camera_parameter(process, CAMERA_PARAM3_CONTROLLER)?;
    let camera_param4 = read_camera_parameter(process, CAMERA_PARAM4_CONTROLLER)?;
    let camera_target = read_camera_target(process);
    let camera_after = process.read_bytes(CAMERA_BLOCK, CAMERA_BLOCK_BYTES)?;
    let tick_after = process.read_u32(TICK_50HZ)?;
    let camera_duplicate_stable = camera_before == camera_after;
    let camera_parameters_duplicate_stable =
        camera_param3_before == camera_param3 && camera_param4_before == camera_param4;
    let camera_target_duplicate_stable = camera_target_before == camera_target;
    let camera = decode_camera_block(&camera_after);

    Ok(RenderSnapshot {
        tick_before,
        tick_after,
        tick_stable: tick_before == tick_after,
        context_fog_near_raw,
        context_fog_far_raw,
        active_fog_near_raw,
        active_fog_far_raw,
        active_fog_scale_raw,
        active_fog_near_cells: active_fog_near_raw as f32 / 256.0,
        active_fog_far_cells: active_fog_far_raw as f32 / 256.0,
        view_transform_raw,
        terrain_scan_columns,
        terrain_scan_rows,
        water_shoreline_base,
        frame_delta_us,
        active_camera_setting,
        camera_duplicate_stable,
        camera_parameters_duplicate_stable,
        camera_target_duplicate_stable,
        camera_param3,
        camera_param4,
        camera_target,
        camera,
    })
}

fn read_camera_target(process: &Process) -> CameraTargetSnapshot {
    let mut snapshot = CameraTargetSnapshot {
        session_pointer: None,
        target_handle_at_context_0x308: None,
        entity_pointer: None,
        entity_type_at_0x58: None,
        entity_handle_at_0x5c: None,
        position_raw_8_8: None,
        velocity_raw_8_8: None,
        resolution_error: None,
    };

    let session_pointer = match process.read_u32(SESSION_POINTER_GLOBAL) {
        Ok(pointer) if plausible_heap_pointer(pointer as usize) => pointer,
        Ok(pointer) => {
            snapshot.resolution_error =
                Some(format!("session pointer {pointer:08X} is implausible"));
            return snapshot;
        }
        Err(error) => {
            snapshot.resolution_error = Some(format!("could not read session pointer: {error}"));
            return snapshot;
        }
    };
    snapshot.session_pointer = Some(session_pointer);

    let target_handle =
        match process.read_u32(session_pointer as usize + SESSION_CAMERA_TARGET_HANDLE_OFFSET) {
            Ok(handle) => handle,
            Err(error) => {
                snapshot.resolution_error =
                    Some(format!("could not read camera target handle: {error}"));
                return snapshot;
            }
        };
    if target_handle == 0 {
        return snapshot;
    }
    snapshot.target_handle_at_context_0x308 = Some(target_handle);

    let resolution = match resolve_resource_handle(process, target_handle) {
        Ok(resolution) => resolution,
        Err(error) => {
            snapshot.resolution_error = Some(error);
            return snapshot;
        }
    };
    snapshot.entity_pointer = Some(resolution.object_pointer);
    let bytes = match process.read_bytes(
        resolution.object_pointer as usize,
        CAMERA_TARGET_ENTITY_BYTES,
    ) {
        Ok(bytes) => bytes,
        Err(error) => {
            snapshot.resolution_error =
                Some(format!("could not read camera target entity: {error}"));
            return snapshot;
        }
    };
    match decode_camera_target_entity(&bytes, target_handle) {
        Ok((entity_type, entity_handle, position, velocity)) => {
            snapshot.entity_type_at_0x58 = Some(entity_type);
            snapshot.entity_handle_at_0x5c = Some(entity_handle);
            snapshot.position_raw_8_8 = Some(position);
            snapshot.velocity_raw_8_8 = Some(velocity);
        }
        Err(error) => snapshot.resolution_error = Some(error),
    }
    snapshot
}

fn decode_camera_target_entity(
    bytes: &[u8],
    expected_handle: u32,
) -> Result<(u32, u32, [i16; 3], [i16; 3]), String> {
    if bytes.len() < CAMERA_TARGET_ENTITY_BYTES {
        return Err(format!(
            "camera target entity read has {} bytes; expected {CAMERA_TARGET_ENTITY_BYTES}",
            bytes.len()
        ));
    }
    let entity_type = u32_at(bytes, 0x58);
    if entity_type > 129 {
        return Err(format!(
            "camera target has implausible entity type {entity_type}"
        ));
    }
    let entity_handle = u32_at(bytes, 0x5C);
    if entity_handle != expected_handle {
        return Err(format!(
            "camera target entity stores handle {entity_handle:08X}, expected {expected_handle:08X}"
        ));
    }
    Ok((
        entity_type,
        entity_handle,
        i16_triplet(bytes, 0x96),
        i16_triplet(bytes, 0x9C),
    ))
}

fn read_camera_parameter(
    process: &Process,
    pointer_address: usize,
) -> Result<CameraParameterSnapshot, String> {
    let controller_pointer = process.read_u32(pointer_address)?;
    let current_raw = if plausible_heap_pointer(controller_pointer as usize) {
        process
            .read_i32(controller_pointer as usize + CONTROLLER_CURRENT_VALUE_OFFSET)
            .ok()
    } else {
        None
    };
    Ok(CameraParameterSnapshot {
        controller_pointer,
        current_raw,
    })
}

fn decode_camera_block(bytes: &[u8]) -> CameraSpringSnapshot {
    debug_assert_eq!(bytes.len(), CAMERA_BLOCK_BYTES);
    CameraSpringSnapshot {
        output_eye_raw: i16_triplet(bytes, OUTPUT_EYE_OFFSET),
        output_view_basis_raw: i32_matrix(bytes, OUTPUT_VIEW_BASIS_OFFSET),
        spring_state_raw: std::array::from_fn(|index| {
            u32_at(bytes, SPRING_STATE_OFFSET + index * 4)
        }),
        focus_position_raw: i16_triplet(bytes, FOCUS_POSITION_OFFSET),
        focus_velocity_raw: i16_triplet(bytes, FOCUS_VELOCITY_OFFSET),
        focus_initialized: u32_at(bytes, FOCUS_INITIALIZED_OFFSET),
        eye_position_raw: i16_triplet(bytes, EYE_POSITION_OFFSET),
        eye_velocity_raw: i16_triplet(bytes, EYE_VELOCITY_OFFSET),
        eye_initialized: u32_at(bytes, EYE_INITIALIZED_OFFSET),
        generated_view_basis_raw: i32_matrix(bytes, GENERATED_VIEW_BASIS_OFFSET),
        attachment_handle: u32_at(bytes, ATTACHMENT_HANDLE_OFFSET),
        tracked_position_raw: i16_triplet(bytes, TRACKED_POSITION_OFFSET),
        tracked_body_basis_raw: i32_matrix(bytes, TRACKED_BODY_BASIS_OFFSET),
    }
}

fn i16_triplet(bytes: &[u8], offset: usize) -> [i16; 3] {
    std::array::from_fn(|index| i16_at(bytes, offset + index * 2))
}

fn i32_matrix(bytes: &[u8], offset: usize) -> [i32; 9] {
    std::array::from_fn(|index| u32_at(bytes, offset + index * 4) as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put_i16(bytes: &mut [u8], offset: usize, value: i16) {
        bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn put_i32(bytes: &mut [u8], offset: usize, value: i32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    #[test]
    fn camera_block_layout_matches_fun_0040f3a0_globals() {
        let mut bytes = vec![0; CAMERA_BLOCK_BYTES];
        for (index, value) in [11, 12, 13].into_iter().enumerate() {
            put_i16(&mut bytes, OUTPUT_EYE_OFFSET + index * 2, value);
        }
        for (index, value) in [21, 22, 23].into_iter().enumerate() {
            put_i16(&mut bytes, FOCUS_POSITION_OFFSET + index * 2, value);
        }
        for (index, value) in [31, 32, 33].into_iter().enumerate() {
            put_i16(&mut bytes, FOCUS_VELOCITY_OFFSET + index * 2, value);
        }
        put_i32(&mut bytes, FOCUS_INITIALIZED_OFFSET, 1);
        for (index, value) in [41, 42, 43].into_iter().enumerate() {
            put_i16(&mut bytes, EYE_POSITION_OFFSET + index * 2, value);
        }
        for (index, value) in [51, 52, 53].into_iter().enumerate() {
            put_i16(&mut bytes, EYE_VELOCITY_OFFSET + index * 2, value);
        }
        put_i32(&mut bytes, EYE_INITIALIZED_OFFSET, 1);
        for index in 0..9 {
            put_i32(
                &mut bytes,
                OUTPUT_VIEW_BASIS_OFFSET + index * 4,
                100 + index as i32,
            );
            put_i32(
                &mut bytes,
                GENERATED_VIEW_BASIS_OFFSET + index * 4,
                200 + index as i32,
            );
            put_i32(
                &mut bytes,
                TRACKED_BODY_BASIS_OFFSET + index * 4,
                300 + index as i32,
            );
        }
        put_i32(&mut bytes, ATTACHMENT_HANDLE_OFFSET, 0x047F_0001);
        for (index, value) in [61, 62, 63].into_iter().enumerate() {
            put_i16(&mut bytes, TRACKED_POSITION_OFFSET + index * 2, value);
        }

        let camera = decode_camera_block(&bytes);
        assert_eq!(camera.output_eye_raw, [11, 12, 13]);
        assert_eq!(
            camera.output_view_basis_raw,
            [100, 101, 102, 103, 104, 105, 106, 107, 108]
        );
        assert_eq!(camera.spring_state_raw[0], 0x0016_0015);
        assert_eq!(camera.spring_state_raw[30], 208);
        assert_eq!(camera.focus_position_raw, [21, 22, 23]);
        assert_eq!(camera.focus_velocity_raw, [31, 32, 33]);
        assert_eq!(camera.focus_initialized, 1);
        assert_eq!(camera.eye_position_raw, [41, 42, 43]);
        assert_eq!(camera.eye_velocity_raw, [51, 52, 53]);
        assert_eq!(camera.eye_initialized, 1);
        assert_eq!(
            camera.generated_view_basis_raw,
            [200, 201, 202, 203, 204, 205, 206, 207, 208]
        );
        assert_eq!(camera.attachment_handle, 0x047F_0001);
        assert_eq!(camera.tracked_position_raw, [61, 62, 63]);
        assert_eq!(
            camera.tracked_body_basis_raw,
            [300, 301, 302, 303, 304, 305, 306, 307, 308]
        );
    }

    #[test]
    fn camera_target_entity_retains_signed_pose_and_rejects_recycled_handles() {
        let expected_handle = 0x04FF_0001_u32;
        let mut bytes = vec![0; CAMERA_TARGET_ENTITY_BYTES];
        bytes[0x58..0x5C].copy_from_slice(&52_u32.to_le_bytes());
        bytes[0x5C..0x60].copy_from_slice(&expected_handle.to_le_bytes());
        for (index, value) in [-29_149_i16, -468, 32_762].into_iter().enumerate() {
            put_i16(&mut bytes, 0x96 + index * 2, value);
        }
        for (index, value) in [4_212_i16, 0, -645].into_iter().enumerate() {
            put_i16(&mut bytes, 0x9C + index * 2, value);
        }

        let decoded = decode_camera_target_entity(&bytes, expected_handle).unwrap();
        assert_eq!(decoded.0, 52);
        assert_eq!(decoded.1, expected_handle);
        assert_eq!(decoded.2, [-29_149, -468, 32_762]);
        assert_eq!(decoded.3, [4_212, 0, -645]);

        let error = decode_camera_target_entity(&bytes, 0x0401_0001).unwrap_err();
        assert!(error.contains("stores handle 04FF0001"));
    }
}
