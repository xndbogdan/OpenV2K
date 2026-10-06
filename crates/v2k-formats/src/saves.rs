//! V2000 native save-slot parser.
//!
//! Each playable save is a 2,048-byte `Slot00` through `Slot13` file. The
//! retail menu displays one additional, non-selectable row (id 14) labelled
//! "Used for game settings"; it is not backed by an ordinary `Slot14` file.
//!
//! Native files are eight CRC-framed records. A frame stores its payload,
//! its CRC, then padding to the next *strict* 0x80-byte boundary:
//!
//! ```text
//! 0x000, 0x080, 0x100, 0x180   four 4-byte header copies
//! 0x200, 0x480                 two complete 0x248-byte state copies
//! 0x700, 0x780                 two opaque 4-byte tail copies
//! ```
//!
//! `FUN_00448A60` tries copies in file order and accepts the first one whose
//! stored CRC is valid. Later copies are skipped once recovery succeeds. The
//! records are neither encrypted nor required to be identical.

use crate::levels::CAMPAIGN_LOGICAL_TO_GLOBAL_LEVEL_OFFSET;
use crate::ovl::read_u32;
use v2k_core::{Result, V2kError};

/// Number of rows displayed by the retail save/load screen.
pub const SLOT_ROW_COUNT: usize = 15;
/// The final retail row is informational and reserved for game settings.
pub const SETTINGS_SLOT_INDEX: usize = SLOT_ROW_COUNT - 1;
/// Number of rows backed by ordinary `SlotXX` game-save files.
pub const SAVE_SLOT_COUNT: usize = SETTINGS_SLOT_INDEX;
/// Exact size of a native retail slot file.
pub const NATIVE_SLOT_SIZE: usize = 0x800;
/// Complete state payload copied by the retail save/load routines.
pub const STATE_PAYLOAD_SIZE: usize = 0x248;
/// Retail's one-based campaign controller slots backed by gameplay overlays
/// 13 through 50.
pub const FIRST_CAMPAIGN_LOGICAL_LEVEL_ID: u32 = 1;
pub const LAST_CAMPAIGN_LOGICAL_LEVEL_ID: u32 = 38;
/// Last campaign slot that represents an interactive, saveable world. Logical
/// ids 37 and 38 resolve to the Intro1/Intro2 cinematic overlays and must not
/// be launched through the save-game path.
pub const LAST_SAVEABLE_CAMPAIGN_LOGICAL_LEVEL_ID: u32 = 36;

const HEADER_COPIES: usize = 4;
const STATE_COPIES: usize = 2;
const TAIL_COPIES: usize = 2;
const HEADER_PAYLOAD_SIZE: usize = 4;
const TAIL_PAYLOAD_SIZE: usize = 4;
const HEADER_STRIDE: usize = 0x80;
const STATE_OFFSET: usize = 0x200;
const STATE_STRIDE: usize = 0x280;
const TAIL_OFFSET: usize = 0x700;
const TAIL_STRIDE: usize = 0x80;
const UNMAPPED_STATE_OFFSET: usize = 0x1c0;

#[derive(Debug, Clone, Copy)]
struct RecordRef<'a> {
    payload: &'a [u8],
    stored_crc: u32,
    crc_valid: bool,
}

/// Parsed V2000 native save slot.
#[derive(Debug, Clone)]
pub struct SaveSlot {
    /// Magic/version word selected through retail's CRC fallback (expected 5).
    pub magic: u32,
    /// Decoded fields from the selected state payload.
    pub game_state: GameState,
    /// Complete selected 0x248-byte state payload, retained for RE work.
    pub state_payload: [u8; STATE_PAYLOAD_SIZE],
    /// Opaque four-byte value selected from the final record group.
    pub tail_value: u32,
    /// Stored CRC associated with the selected state copy.
    pub checksum: u32,
    /// Per-copy CRC results in physical file order.
    pub header_crc_valid: [bool; HEADER_COPIES],
    pub state_crc_valid: [bool; STATE_COPIES],
    pub tail_crc_valid: [bool; TAIL_COPIES],
    /// Copy indexes selected by retail's first-valid fallback, if any.
    pub selected_header_copy: Option<usize>,
    pub selected_state_copy: Option<usize>,
    pub selected_tail_copy: Option<usize>,
    /// Diagnostic equality only; mismatched redundant copies do not make a
    /// slot invalid when the first selected copy passes its CRC.
    pub states_match: bool,
    pub checksums_match: bool,
    pub headers_match: bool,
    pub blocks_cd_match: bool,
    /// Non-zero, not-yet-mapped u32 values in state bytes +0x1C0..+0x247.
    /// Offsets use the canonical first-state location (0x200 + relative).
    pub trailing_fields: Vec<(usize, u32)>,
}

/// Partially decoded fields from the native 0x248-byte state snapshot.
#[derive(Debug, Clone)]
pub struct GameState {
    /// Editable save-row label (32-byte field; retail probes at most 31
    /// characters). This commonly starts as the world name, but is not the
    /// authoritative name of the referenced level.
    pub display_name: String,
    /// One-based campaign controller slot used by the retail loader.
    ///
    /// This is not a global OVL id. `FUN_0042E570` loads the corresponding
    /// gameplay OVL at `logical_level_id + 12`.
    pub logical_level_id: u32,
    /// Exact player-controller snapshot copied by `FUN_00443440` and restored
    /// by `FUN_00443560` after the persistent type-46 allocation is rebuilt.
    pub player: NativePlayerSnapshot,
    pub field_38: u32,
    pub field_40: u32,
    pub field_44: u32,
    pub field_48: u32,
    pub field_4c: u32,
    pub field_50: u32,
    pub field_54: u32,
    pub field_60: u32,
    pub field_64: u32,
    pub field_68: u32,
}

/// Proven player fields within the native 0x248-byte session snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativePlayerSnapshot {
    /// Signed 8.8 world coordinates at state `+0x24..+0x29`.
    pub position_raw: [i16; 3],
    /// Signed 8.8 velocity at state `+0x2A..+0x2F`.
    pub velocity_raw: [i16; 3],
    /// Unsigned binary angles at state `+0x30..+0x35`.
    pub heading_raw: u16,
    pub pitch_raw: u16,
    pub roll_raw: u16,
    /// Signed raw hull health at state `+0x3C`.
    pub health_raw: i32,
}

impl GameState {
    /// Resolve the native campaign slot to its global world-overlay id.
    /// Values outside retail's 38 world/cinematic slots are rejected rather
    /// than being allowed to address a system OVL.
    pub fn global_level_id(&self) -> Option<u32> {
        (FIRST_CAMPAIGN_LOGICAL_LEVEL_ID..=LAST_CAMPAIGN_LOGICAL_LEVEL_ID)
            .contains(&self.logical_level_id)
            .then(|| self.logical_level_id + CAMPAIGN_LOGICAL_TO_GLOBAL_LEVEL_OFFSET)
    }

    /// Resolve only an interactive world that the compatibility save loader
    /// may enter. Intro1/Intro2 are valid world-overlay ids, but their runtime
    /// contracts are cinematic and cannot be treated as `Playing`.
    pub fn saveable_global_level_id(&self) -> Option<u32> {
        (FIRST_CAMPAIGN_LOGICAL_LEVEL_ID..=LAST_SAVEABLE_CAMPAIGN_LOGICAL_LEVEL_ID)
            .contains(&self.logical_level_id)
            .then(|| self.logical_level_id + CAMPAIGN_LOGICAL_TO_GLOBAL_LEVEL_OFFSET)
    }
}

impl SaveSlot {
    /// Parse a complete native slot while retaining corruption diagnostics.
    ///
    /// Structurally complete but CRC-invalid data still produces a `SaveSlot`
    /// so diagnostic tools can explain the failure. Call [`Self::is_valid`]
    /// before using the selected state as retail-compatible input.
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() != NATIVE_SLOT_SIZE {
            return Err(V2kError::OvlFormat(format!(
                "Save slot must be exactly {} bytes, got {}",
                NATIVE_SLOT_SIZE,
                data.len()
            )));
        }

        let headers: [RecordRef<'_>; HEADER_COPIES] = std::array::from_fn(|copy| {
            parse_record(data, copy * HEADER_STRIDE, HEADER_PAYLOAD_SIZE)
        });
        let states: [RecordRef<'_>; STATE_COPIES] = std::array::from_fn(|copy| {
            parse_record(data, STATE_OFFSET + copy * STATE_STRIDE, STATE_PAYLOAD_SIZE)
        });
        let tails: [RecordRef<'_>; TAIL_COPIES] = std::array::from_fn(|copy| {
            parse_record(data, TAIL_OFFSET + copy * TAIL_STRIDE, TAIL_PAYLOAD_SIZE)
        });

        let selected_header_copy = first_valid_copy(&headers);
        let selected_state_copy = first_valid_copy(&states);
        let selected_tail_copy = first_valid_copy(&tails);

        // Retain the first physical record for diagnostics when an entire
        // group is corrupt. Such a slot remains invalid and is never imported.
        let header = headers[selected_header_copy.unwrap_or(0)];
        let state = states[selected_state_copy.unwrap_or(0)];
        let tail = tails[selected_tail_copy.unwrap_or(0)];

        let magic = read_u32(header.payload, 0)?;
        let mut state_payload = [0_u8; STATE_PAYLOAD_SIZE];
        state_payload.copy_from_slice(state.payload);
        let game_state = parse_game_state(&state_payload)?;
        let tail_value = read_u32(tail.payload, 0)?;

        let headers_match = headers
            .iter()
            .skip(1)
            .all(|record| record.payload == headers[0].payload);
        let states_match = states[0].payload == states[1].payload;
        let checksums_match = states[0].stored_crc == states[1].stored_crc;
        let blocks_cd_match = tails[0].payload == tails[1].payload;

        let mut trailing_fields = Vec::new();
        for relative in (UNMAPPED_STATE_OFFSET..STATE_PAYLOAD_SIZE).step_by(4) {
            let value = read_u32(&state_payload, relative)?;
            if value != 0 {
                trailing_fields.push((STATE_OFFSET + relative, value));
            }
        }

        Ok(Self {
            magic,
            game_state,
            state_payload,
            tail_value,
            checksum: state.stored_crc,
            header_crc_valid: headers.map(|record| record.crc_valid),
            state_crc_valid: states.map(|record| record.crc_valid),
            tail_crc_valid: tails.map(|record| record.crc_valid),
            selected_header_copy,
            selected_state_copy,
            selected_tail_copy,
            states_match,
            checksums_match,
            headers_match,
            blocks_cd_match,
            trailing_fields,
        })
    }

    /// Whether retail's save-menu probe can recover the header and state rows
    /// it uses for status and label presentation.
    pub fn menu_probe_is_valid(&self) -> bool {
        self.magic == 5 && self.selected_header_copy.is_some() && self.selected_state_copy.is_some()
    }

    /// Whether the complete native save, including the opaque tail record,
    /// can be recovered for loading.
    pub fn full_load_is_valid(&self) -> bool {
        self.menu_probe_is_valid() && self.selected_tail_copy.is_some()
    }

    /// Whether the complete native save can be recovered for loading.
    pub fn is_valid(&self) -> bool {
        self.full_load_is_valid()
    }

    /// Encode this save slot into the canonical 2,048-byte native layout.
    pub fn encode(&self) -> [u8; NATIVE_SLOT_SIZE] {
        encode_native_save_slot(&self.state_payload, self.magic, self.tail_value)
    }
}

/// Encode a 2,048-byte native retail save slot with redundant CRC-protected
/// header, state, and tail records matching `FUN_00448CF0`.
pub fn encode_native_save_slot(
    state_payload: &[u8; STATE_PAYLOAD_SIZE],
    magic: u32,
    tail_value: u32,
) -> [u8; NATIVE_SLOT_SIZE] {
    let mut data = [0_u8; NATIVE_SLOT_SIZE];
    let header_bytes = magic.to_le_bytes();
    let header_crc = retail_crc32(&header_bytes).to_le_bytes();
    for copy in 0..HEADER_COPIES {
        let offset = copy * HEADER_STRIDE;
        data[offset..offset + 4].copy_from_slice(&header_bytes);
        data[offset + 4..offset + 8].copy_from_slice(&header_crc);
    }

    let state_crc = retail_crc32(state_payload).to_le_bytes();
    for copy in 0..STATE_COPIES {
        let offset = STATE_OFFSET + copy * STATE_STRIDE;
        data[offset..offset + STATE_PAYLOAD_SIZE].copy_from_slice(state_payload);
        data[offset + STATE_PAYLOAD_SIZE..offset + STATE_PAYLOAD_SIZE + 4]
            .copy_from_slice(&state_crc);
    }

    let tail_bytes = tail_value.to_le_bytes();
    let tail_crc = retail_crc32(&tail_bytes).to_le_bytes();
    for copy in 0..TAIL_COPIES {
        let offset = TAIL_OFFSET + copy * TAIL_STRIDE;
        data[offset..offset + 4].copy_from_slice(&tail_bytes);
        data[offset + 4..offset + 8].copy_from_slice(&tail_crc);
    }

    data
}

fn parse_record(data: &[u8], offset: usize, payload_size: usize) -> RecordRef<'_> {
    let payload = &data[offset..offset + payload_size];
    let stored_crc = u32::from_le_bytes(
        data[offset + payload_size..offset + payload_size + 4]
            .try_into()
            .expect("fixed native save record"),
    );
    RecordRef {
        payload,
        stored_crc,
        crc_valid: retail_crc32(payload) == stored_crc,
    }
}

fn first_valid_copy<const N: usize>(records: &[RecordRef<'_>; N]) -> Option<usize> {
    records.iter().position(|record| record.crc_valid)
}

/// CRC generated by `FUN_00448950`/`FUN_00448A20`: reflected IEEE polynomial,
/// zero seed, and no final complement.
pub fn retail_crc32(payload: &[u8]) -> u32 {
    let mut crc = 0_u32;
    for &byte in payload {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }
    crc
}

fn parse_game_state(data: &[u8; STATE_PAYLOAD_SIZE]) -> Result<GameState> {
    let name_raw = &data[..32];
    let null_pos = name_raw.iter().position(|&byte| byte == 0).unwrap_or(32);
    let display_name = String::from_utf8_lossy(&name_raw[..null_pos]).into_owned();

    let read_i16 = |offset: usize| i16::from_le_bytes([data[offset], data[offset + 1]]);
    let read_u16 = |offset: usize| u16::from_le_bytes([data[offset], data[offset + 1]]);

    Ok(GameState {
        display_name,
        logical_level_id: read_u32(data, 0x20)?,
        player: NativePlayerSnapshot {
            position_raw: [read_i16(0x24), read_i16(0x26), read_i16(0x28)],
            velocity_raw: [read_i16(0x2a), read_i16(0x2c), read_i16(0x2e)],
            heading_raw: read_u16(0x30),
            pitch_raw: read_u16(0x32),
            roll_raw: read_u16(0x34),
            health_raw: i32::from_le_bytes(
                data[0x3c..0x40]
                    .try_into()
                    .expect("fixed native player-health field"),
            ),
        },
        field_38: read_u32(data, 0x38)?,
        field_40: read_u32(data, 0x40)?,
        field_44: read_u32(data, 0x44)?,
        field_48: read_u32(data, 0x48)?,
        field_4c: read_u32(data, 0x4c)?,
        field_50: read_u32(data, 0x50)?,
        field_54: read_u32(data, 0x54)?,
        field_60: read_u32(data, 0x60)?,
        field_64: read_u32(data, 0x64)?,
        field_68: read_u32(data, 0x68)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_record(buf: &mut [u8], offset: usize, payload: &[u8]) {
        buf[offset..offset + payload.len()].copy_from_slice(payload);
        let crc = retail_crc32(payload);
        buf[offset + payload.len()..offset + payload.len() + 4].copy_from_slice(&crc.to_le_bytes());
    }

    fn make_slot(name: &str, level_id: u32, score: u32) -> Vec<u8> {
        let mut buf = vec![0_u8; NATIVE_SLOT_SIZE];
        let header = 5_u32.to_le_bytes();
        for copy in 0..HEADER_COPIES {
            write_record(&mut buf, copy * HEADER_STRIDE, &header);
        }

        let mut state = [0_u8; STATE_PAYLOAD_SIZE];
        let name_bytes = name.as_bytes();
        let len = name_bytes.len().min(31);
        state[..len].copy_from_slice(&name_bytes[..len]);
        state[0x20..0x24].copy_from_slice(&level_id.to_le_bytes());
        state[0x30..0x34].copy_from_slice(&0x4000_u32.to_le_bytes());
        state[0x3c..0x40].copy_from_slice(&40_000_i32.to_le_bytes());
        state[0x44..0x48].copy_from_slice(&score.to_le_bytes());
        state[0x54..0x58].copy_from_slice(&score.to_le_bytes());
        for copy in 0..STATE_COPIES {
            write_record(&mut buf, STATE_OFFSET + copy * STATE_STRIDE, &state);
        }

        let tail = 0x0020_1000_u32.to_le_bytes();
        for copy in 0..TAIL_COPIES {
            write_record(&mut buf, TAIL_OFFSET + copy * TAIL_STRIDE, &tail);
        }
        buf
    }

    fn rewrite_record_crc(buf: &mut [u8], offset: usize, payload_size: usize) {
        let crc = retail_crc32(&buf[offset..offset + payload_size]);
        buf[offset + payload_size..offset + payload_size + 4].copy_from_slice(&crc.to_le_bytes());
    }

    #[test]
    fn crc_matches_retail_magic_record() {
        assert_eq!(retail_crc32(&5_u32.to_le_bytes()), 0x37de_f032);
    }

    #[test]
    fn parses_complete_synthetic_slot() {
        let data = make_slot("A level name longer than 16", 7, 1234);
        let slot = SaveSlot::parse(&data).unwrap();

        assert_eq!(slot.magic, 5);
        assert_eq!(slot.game_state.display_name, "A level name longer than 16");
        assert_eq!(slot.game_state.logical_level_id, 7);
        assert_eq!(slot.game_state.global_level_id(), Some(19));
        assert_eq!(slot.game_state.field_44, 1234);
        assert_eq!(slot.game_state.field_54, 1234);
        assert_eq!(slot.game_state.player.heading_raw, 0x4000);
        assert_eq!(slot.game_state.player.pitch_raw, 0);
        assert_eq!(slot.game_state.player.roll_raw, 0);
        assert_eq!(slot.game_state.player.health_raw, 40_000);
        assert_eq!(slot.tail_value, 0x0020_1000);
        assert_eq!(slot.selected_header_copy, Some(0));
        assert_eq!(slot.selected_state_copy, Some(0));
        assert_eq!(slot.selected_tail_copy, Some(0));
        assert!(slot.header_crc_valid.iter().all(|valid| *valid));
        assert!(slot.state_crc_valid.iter().all(|valid| *valid));
        assert!(slot.tail_crc_valid.iter().all(|valid| *valid));
        assert!(slot.is_valid());
        assert!(slot.trailing_fields.is_empty());
    }

    #[test]
    fn retail_falls_back_from_a_corrupt_copy() {
        let mut data = make_slot("Fallback", 9, 77);
        data[4] ^= 0xff;
        data[STATE_OFFSET + STATE_PAYLOAD_SIZE] ^= 0xff;
        data[TAIL_OFFSET + TAIL_PAYLOAD_SIZE] ^= 0xff;

        let slot = SaveSlot::parse(&data).unwrap();
        assert_eq!(slot.selected_header_copy, Some(1));
        assert_eq!(slot.selected_state_copy, Some(1));
        assert_eq!(slot.selected_tail_copy, Some(1));
        assert!(slot.is_valid());
    }

    #[test]
    fn menu_probe_remains_valid_when_both_tail_copies_fail_crc() {
        let mut data = make_slot("Tail failure", 9, 77);
        for copy in 0..TAIL_COPIES {
            data[TAIL_OFFSET + copy * TAIL_STRIDE + TAIL_PAYLOAD_SIZE] ^= 0xff;
        }

        let slot = SaveSlot::parse(&data).unwrap();
        assert!(slot.menu_probe_is_valid());
        assert!(!slot.full_load_is_valid());
        assert!(!slot.is_valid());
        assert_eq!(slot.selected_tail_copy, None);
    }

    #[test]
    fn valid_copies_may_disagree_without_defeating_first_valid_selection() {
        let mut data = make_slot("First", 1, 100);
        let second = STATE_OFFSET + STATE_STRIDE;
        data[second..second + 6].copy_from_slice(b"Second");
        rewrite_record_crc(&mut data, second, STATE_PAYLOAD_SIZE);

        let slot = SaveSlot::parse(&data).unwrap();
        assert_eq!(slot.game_state.display_name, "First");
        assert!(!slot.states_match);
        assert!(!slot.checksums_match);
        assert!(slot.is_valid());
    }

    #[test]
    fn invalid_when_every_copy_in_a_group_fails_crc() {
        let mut data = make_slot("Broken", 1, 100);
        for copy in 0..STATE_COPIES {
            data[STATE_OFFSET + copy * STATE_STRIDE + STATE_PAYLOAD_SIZE] ^= 0xff;
        }
        let slot = SaveSlot::parse(&data).unwrap();
        assert_eq!(slot.selected_state_copy, None);
        assert!(!slot.is_valid());
    }

    #[test]
    fn invalid_when_crc_valid_header_has_wrong_magic() {
        let mut data = make_slot("Wrong magic", 1, 100);
        for copy in 0..HEADER_COPIES {
            let offset = copy * HEADER_STRIDE;
            data[offset..offset + 4].copy_from_slice(&4_u32.to_le_bytes());
            rewrite_record_crc(&mut data, offset, HEADER_PAYLOAD_SIZE);
        }
        let slot = SaveSlot::parse(&data).unwrap();
        assert_eq!(slot.magic, 4);
        assert!(!slot.is_valid());
    }

    #[test]
    fn wrong_size() {
        assert!(SaveSlot::parse(&vec![0_u8; 1024]).is_err());
        assert!(SaveSlot::parse(&vec![0_u8; 4096]).is_err());
    }

    #[test]
    fn native_level_field_separates_saveable_worlds_from_cinematics_and_system_overlays() {
        let zero = SaveSlot::parse(&make_slot("Zero", 0, 0)).unwrap();
        let last_world = SaveSlot::parse(&make_slot("Last world", 36, 0)).unwrap();
        let intro1 = SaveSlot::parse(&make_slot("Intro1", 37, 0)).unwrap();
        let intro2 = SaveSlot::parse(&make_slot("Intro2", 38, 0)).unwrap();
        let past_campaign = SaveSlot::parse(&make_slot("Past", 39, 0)).unwrap();
        assert_eq!(zero.game_state.global_level_id(), None);
        assert_eq!(zero.game_state.saveable_global_level_id(), None);
        assert_eq!(last_world.game_state.global_level_id(), Some(48));
        assert_eq!(last_world.game_state.saveable_global_level_id(), Some(48));
        assert_eq!(intro1.game_state.global_level_id(), Some(49));
        assert_eq!(intro1.game_state.saveable_global_level_id(), None);
        assert_eq!(intro2.game_state.global_level_id(), Some(50));
        assert_eq!(intro2.game_state.saveable_global_level_id(), None);
        assert_eq!(past_campaign.game_state.global_level_id(), None);
        assert_eq!(past_campaign.game_state.saveable_global_level_id(), None);
    }

    #[test]
    fn encoded_native_slot_round_trips_through_parser() {
        let original_data = make_slot("Round trip test", 13, 9876);
        let parsed = SaveSlot::parse(&original_data).unwrap();
        let encoded = parsed.encode();
        assert_eq!(encoded.len(), NATIVE_SLOT_SIZE);
        assert_eq!(&encoded[..], &original_data[..]);

        let reparsed = SaveSlot::parse(&encoded).unwrap();
        assert_eq!(reparsed.magic, parsed.magic);
        assert_eq!(
            reparsed.game_state.display_name,
            parsed.game_state.display_name
        );
        assert_eq!(
            reparsed.game_state.logical_level_id,
            parsed.game_state.logical_level_id
        );
        assert_eq!(reparsed.tail_value, parsed.tail_value);
        assert_eq!(reparsed.state_payload, parsed.state_payload);
        assert!(reparsed.is_valid());
    }
}
