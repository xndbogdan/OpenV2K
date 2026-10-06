//! Read-only access to the retail player-controller chain.
//!
//! Both the fire probe and the hover-input probe reach the controller through
//! `*(0x004F72C8) + 0x27C`. Keeping that pointer validation here prevents the
//! two capture paths from drifting onto different interpretations of the same
//! live object.

use serde::Serialize;

use crate::process::{i16_at, plausible_heap_pointer, u32_at, Process};

pub const SESSION_POINTER_GLOBAL: usize = 0x004F_72C8;
pub const SESSION_CONTROLLER_OFFSET: usize = 0x27C;
pub const CONTROLLER_ENTITY_HANDLE_OFFSET: usize = 0x68;
pub const CONTROLLER_MOTION_VECTOR_OFFSET: usize = 0x280;
pub const CONTROLLER_MOTION_VECTOR_BYTES: usize = 0x10;
const TICK_50HZ: usize = 0x004F_ED60;

/// The validated session/controller pointers used for one bounded read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ControllerLocation {
    pub session_pointer: u32,
    pub controller_pointer: u32,
    pub entity_handle_at_0x68: Option<u32>,
    pub entity_handle_matches_expected: Option<bool>,
}

/// Offset-named values from the exact controller `+0x280..+0x28F` bytes.
///
/// These names intentionally describe representation only. In particular,
/// signed word `+0x06` has no recovered gameplay meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ControllerMotionRawFields {
    pub signed_word_at_00: i16,
    pub signed_word_at_02: i16,
    pub signed_word_at_04: i16,
    pub signed_word_at_06: i16,
    pub signed_dword_at_08: i32,
    pub signed_dword_at_0c: i32,
}

/// Only the channel meanings established by `FUN_004445E0` and its callers.
/// Values remain in retail raw units; the probe does not normalize them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ControllerMotionDecodedChannels {
    pub turn_raw_at_00: i16,
    pub pitch_raw_at_02: i16,
    pub vertical_raw_at_04: i16,
    pub throttle_raw_at_08: i32,
    pub fire_raw_at_0c: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ControllerMotionVector {
    pub address: u32,
    pub byte_length: usize,
    pub raw_bytes: [u8; CONTROLLER_MOTION_VECTOR_BYTES],
    pub raw_hex: String,
    pub raw_fields: ControllerMotionRawFields,
    pub decoded_channels: ControllerMotionDecodedChannels,
}

/// A non-fatal evidence record. Pointer/read failures stay in `warnings` so a
/// transient controller transition does not abort the surrounding timeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ControllerMotionEvidence {
    pub tick_before: Option<u32>,
    pub tick_after: Option<u32>,
    pub tick_stable: Option<bool>,
    pub session_pointer_global: u32,
    pub session_controller_offset: u32,
    pub motion_vector_offset: u32,
    pub location: Option<ControllerLocation>,
    pub motion_vector: Option<ControllerMotionVector>,
    pub warnings: Vec<String>,
}

pub fn locate_controller(
    process: &Process,
    expected_entity_handle: Option<u32>,
    warnings: &mut Vec<String>,
) -> Option<ControllerLocation> {
    let session_pointer =
        read_heap_pointer(process, SESSION_POINTER_GLOBAL, "session pointer", warnings)?;
    let controller_pointer = read_heap_pointer(
        process,
        session_pointer as usize + SESSION_CONTROLLER_OFFSET,
        "player controller at session +0x27C",
        warnings,
    )?;
    let entity_handle_at_0x68 =
        match process.read_u32(controller_pointer as usize + CONTROLLER_ENTITY_HANDLE_OFFSET) {
            Ok(handle) => Some(handle),
            Err(error) => {
                warnings.push(format!(
                "could not read controller entity handle at {controller_pointer:08X}+0x68: {error}"
            ));
                None
            }
        };
    let entity_handle_matches_expected = expected_entity_handle
        .zip(entity_handle_at_0x68)
        .map(|(expected, actual)| expected == actual);
    if entity_handle_matches_expected == Some(false) {
        warnings.push(format!(
            "controller entity handle {:08X} does not match expected player handle {:08X}",
            entity_handle_at_0x68.unwrap(),
            expected_entity_handle.unwrap()
        ));
    }

    Some(ControllerLocation {
        session_pointer,
        controller_pointer,
        entity_handle_at_0x68,
        entity_handle_matches_expected,
    })
}

pub fn read_motion_evidence(
    process: &Process,
    expected_entity_handle: Option<u32>,
) -> ControllerMotionEvidence {
    let mut warnings = Vec::new();
    let tick_before = read_optional_u32(
        process,
        TICK_50HZ,
        "50 Hz tick before motion read",
        &mut warnings,
    );
    let location = locate_controller(process, expected_entity_handle, &mut warnings);
    let motion_vector = location.and_then(|location| {
        let address = location.controller_pointer as usize + CONTROLLER_MOTION_VECTOR_OFFSET;
        match process.read_bytes(address, CONTROLLER_MOTION_VECTOR_BYTES) {
            Ok(bytes) => Some(parse_motion_vector(address as u32, &bytes)),
            Err(error) => {
                warnings.push(format!(
                    "could not read controller motion vector at {:08X}+0x280: {error}",
                    location.controller_pointer
                ));
                None
            }
        }
    });
    let tick_after = read_optional_u32(
        process,
        TICK_50HZ,
        "50 Hz tick after motion read",
        &mut warnings,
    );

    ControllerMotionEvidence {
        tick_before,
        tick_after,
        tick_stable: tick_before
            .zip(tick_after)
            .map(|(before, after)| before == after),
        session_pointer_global: SESSION_POINTER_GLOBAL as u32,
        session_controller_offset: SESSION_CONTROLLER_OFFSET as u32,
        motion_vector_offset: CONTROLLER_MOTION_VECTOR_OFFSET as u32,
        location,
        motion_vector,
        warnings,
    }
}

fn read_optional_u32(
    process: &Process,
    address: usize,
    label: &str,
    warnings: &mut Vec<String>,
) -> Option<u32> {
    match process.read_u32(address) {
        Ok(value) => Some(value),
        Err(error) => {
            warnings.push(format!("could not read {label} at {address:08X}: {error}"));
            None
        }
    }
}

fn parse_motion_vector(address: u32, bytes: &[u8]) -> ControllerMotionVector {
    debug_assert!(bytes.len() >= CONTROLLER_MOTION_VECTOR_BYTES);
    let raw_bytes: [u8; CONTROLLER_MOTION_VECTOR_BYTES] = bytes[..CONTROLLER_MOTION_VECTOR_BYTES]
        .try_into()
        .expect("bounded controller motion read");
    let raw_fields = ControllerMotionRawFields {
        signed_word_at_00: i16_at(&raw_bytes, 0x00),
        signed_word_at_02: i16_at(&raw_bytes, 0x02),
        signed_word_at_04: i16_at(&raw_bytes, 0x04),
        signed_word_at_06: i16_at(&raw_bytes, 0x06),
        signed_dword_at_08: u32_at(&raw_bytes, 0x08) as i32,
        signed_dword_at_0c: u32_at(&raw_bytes, 0x0C) as i32,
    };
    ControllerMotionVector {
        address,
        byte_length: CONTROLLER_MOTION_VECTOR_BYTES,
        raw_bytes,
        raw_hex: encode_hex(&raw_bytes),
        raw_fields,
        decoded_channels: ControllerMotionDecodedChannels {
            turn_raw_at_00: raw_fields.signed_word_at_00,
            pitch_raw_at_02: raw_fields.signed_word_at_02,
            vertical_raw_at_04: raw_fields.signed_word_at_04,
            throttle_raw_at_08: raw_fields.signed_dword_at_08,
            fire_raw_at_0c: raw_fields.signed_dword_at_0c,
        },
    }
}

fn read_heap_pointer(
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
    use super::*;

    #[test]
    fn preserves_exact_motion_bytes_and_only_proven_channel_decodes() {
        let mut bytes = [0u8; CONTROLLER_MOTION_VECTOR_BYTES];
        bytes[0x00..0x02].copy_from_slice(&(-2303i16).to_le_bytes());
        bytes[0x02..0x04].copy_from_slice(&(0x0D80i16).to_le_bytes());
        bytes[0x04..0x06].copy_from_slice(&(-0x0A00i16).to_le_bytes());
        bytes[0x06..0x08].copy_from_slice(&(0x1234i16).to_le_bytes());
        bytes[0x08..0x0C].copy_from_slice(&(0x0001_0000i32).to_le_bytes());
        bytes[0x0C..0x10].copy_from_slice(&(-1i32).to_le_bytes());

        let vector = parse_motion_vector(0x0123_4280, &bytes);
        assert_eq!(vector.address, 0x0123_4280);
        assert_eq!(vector.raw_bytes, bytes);
        assert_eq!(vector.raw_hex.len(), CONTROLLER_MOTION_VECTOR_BYTES * 2);
        assert_eq!(vector.raw_fields.signed_word_at_06, 0x1234);
        assert_eq!(vector.decoded_channels.turn_raw_at_00, -2303);
        assert_eq!(vector.decoded_channels.pitch_raw_at_02, 0x0D80);
        assert_eq!(vector.decoded_channels.vertical_raw_at_04, -0x0A00);
        assert_eq!(vector.decoded_channels.throttle_raw_at_08, 0x0001_0000);
        assert_eq!(vector.decoded_channels.fire_raw_at_0c, -1);
    }
}
