//! Section 13 level descriptor decoder.
//!
//! 208-byte (0xD0) base descriptor per level:
//!   +0x00  64 bytes  Level name (null-padded ASCII)
//!   +0x40  36 bytes  Game parameters, including:
//!   +0x48  u32       world_style (1-6, selects system resource OVL 6-11)
//!   +0x4C  u32       terrain_sprite_base (GLOBAL sprite-pool index: 120
//!                    canonical opaque transition tiles, then special tiles;
//!                    shoreline frames are base+125..+129 via FUN_00433180)
//!   +0x54  u16       fullscreen sky/background colour (master palette index)
//!   +0x56  u16       optional auxiliary sky model id
//!   +0x58  u16       Main Base abort sky/background colour
//!   +0x5A  u16       Main Base abort optional sky model id
//!   +0x64  u32       campaign_record_count (0x20-byte trailing records)
//!   +0x68  u32       campaign_records_rel (relative pointer, patched at load)
//!   +0x6C  28 bytes  More parameters
//!   +0x88  u32       terrain draw depth and world fog far plane in cells
//!   +0x8C  u32       world fog width in cells (FUN_0042E920)
//!   +0x90  56 bytes  More parameters
//!   +0xC8  u32       sub_count (entity spawn sub-entries)
//!   +0xCC  u32       sub_arr_rel (relative pointer to sub-entry ptr array)
//!
//! Sub-entry (0x44 = 68 bytes):
//!   +0x00  u32       reserved (always 0)
//!   +0x04  u32       entity_type (entity class ID, range 6-104)
//!   +0x08  [u8; 4]   signed 8.8 fixed X, Y (copied to entity+0x96/+0x98)
//!   +0x0C  [u8; 4]   signed 8.8 fixed Z in low half (entity+0x9A)
//!   +0x10  u32       param
//!   +0x14  40 bytes  extra (mostly zeros)
//!   +0x20  u16[3]    yaw/pitch/roll (copied to entity+0xA2/+0xA4/+0xA6)
//!   +0x2C  u32[4]    optional per-spawn model-slot overrides
//!   +0x3C  u32       anim_ptr (relative offset to animation block, 0 = none)
//!   +0x40  u32       config_ptr (relative offset to 0x58-byte config block, 0 = none)
//!
//! Campaign record: 32 bytes (0x20). Records whose +0x0C flags contain 0x10
//! describe authored terrain-marker transitions; other record kinds retain
//! their raw bytes until their semantics are independently recovered.
//!
//! Present in 38 game world levels (13-50).
//!
//! This Rust decoder is canonical; the earlier Python prototype is retired.

use crate::ovl::{read_u16, read_u32};
use v2k_core::{Result, V2kError};

/// Base descriptor size.
const BASE_SIZE: usize = 0xD0; // 208 bytes

/// Sub-entry size.
const SUB_ENTRY_SIZE: usize = 0x44; // 68 bytes

/// Campaign record size.
const CAMPAIGN_RECORD_SIZE: usize = 0x20; // 32 bytes

/// Section-13 campaign-record flag consumed by `FUN_0042DD10` for an authored
/// terrain-marker transition.
pub const CAMPAIGN_MARKER_TRANSITION_FLAG: u32 = 0x10;

/// Retail stores campaign level numbers as one-based logical ids. Gameplay
/// overlays begin at global level 13, so their global id is logical id + 12.
pub const CAMPAIGN_LOGICAL_TO_GLOBAL_LEVEL_OFFSET: u32 = 12;

/// An entity spawn sub-entry.
#[derive(Debug, Clone)]
pub struct EntitySpawn {
    /// Index within the level's sub-entry array.
    pub index: usize,
    /// Entity class ID (range 6-104).
    pub entity_type: u32,
    /// Signed little-endian 8.8 fixed X/Y pair.
    pub pos_data_1: [u8; 4],
    /// Signed little-endian 8.8 fixed Z in the low pair. The high pair is not
    /// copied by the base entity constructor.
    pub pos_data_2: [u8; 4],
    /// Varies (0 or 1 typically).
    pub param: u32,
    /// Authored yaw/pitch/roll at +0x20/+0x22/+0x24. Each is a 16-bit binary
    /// angle where 0x10000 is one complete turn; the first component is
    /// heading/yaw at runtime entity +0xA2.
    pub rotation: [u16; 3],
    /// Complete +0x14..+0x3B per-spawn payload. This includes behavior
    /// arguments and the four model overrides retained separately below.
    pub extra: [u8; 40],
    /// Signed constructor value at spawn +0x28. `FUN_004104B0` receives the
    /// record at +0x04, so `param_2[9]` copies this dword to runtime entity
    /// +0x50/+0x54. Generic damage consumes +0x50 before hull health.
    pub initial_damage_buffer_raw: i32,
    /// Per-instance global Section-8 model overrides. A zero slot falls back
    /// to the corresponding model id in the Section-12 type record.
    pub model_overrides: [u32; 4],
    /// Whether this sub-entry has an animation block.
    pub has_animation: bool,
    /// Number of animation frames (if has_animation).
    pub anim_frames: u32,
    /// Raw optional animation block and its 0x1C-byte frame records.
    pub animation: Option<EntityAnimation>,
    /// Whether this sub-entry has a 0x58-byte config block.
    pub has_config: bool,
    /// Raw optional 0x58-byte behavior/configuration block.
    pub config: Option<[u8; 0x58]>,
}

impl EntitySpawn {
    /// Signed little-endian 8.8 fixed X/Y/Z position.
    pub fn position_raw(&self) -> [i16; 3] {
        [
            i16::from_le_bytes([self.pos_data_1[0], self.pos_data_1[1]]),
            i16::from_le_bytes([self.pos_data_1[2], self.pos_data_1[3]]),
            i16::from_le_bytes([self.pos_data_2[0], self.pos_data_2[1]]),
        ]
    }
}

#[derive(Debug, Clone)]
pub struct EntityAnimation {
    pub header: [u8; 0x18],
    pub frames: Vec<[u8; 0x1C]>,
}

/// A raw Section-13 campaign/goal record.
///
/// Only records carrying [`CAMPAIGN_MARKER_TRANSITION_FLAG`] have the typed
/// transition interpretation exposed by [`CampaignRecord::marker_transition`].
/// The remaining bytes and record kinds deliberately stay unclassified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CampaignRecord {
    /// Raw 32 bytes.
    pub data: [u8; 32],
}

/// Evidence-backed view of a campaign record linked to one of the authored
/// terrain-marker subtypes 1..=5 (`FUN_00433BD0` kinds 0x16..=0x1A).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CampaignMarkerTransition {
    /// One-based campaign level written by `FUN_0042DD10` to the handoff.
    pub destination_logical_level: u32,
    /// Authored signed 8.8 X/Y/Z arrival copied by `FUN_0042E270`.
    pub arrival_raw: [i16; 3],
    /// Signed byte at record +0x1D. Authored transition records use 1..=5.
    pub marker_subtype: i8,
    /// Signed terrain-state selectors at +0x1E/+0x1F. `446440` matches
    /// bit3 then bit4 from the actual static contact; value2 is a wildcard.
    pub marker_args: [i8; 2],
}

impl CampaignMarkerTransition {
    /// Convert the one-based campaign id to the global gameplay-overlay id.
    pub fn destination_global_level(self) -> Option<u32> {
        self.destination_logical_level
            .checked_add(CAMPAIGN_LOGICAL_TO_GLOBAL_LEVEL_OFFSET)
    }
}

impl CampaignRecord {
    /// Raw flag dword at +0x0C.
    pub fn flags(&self) -> u32 {
        u32::from_le_bytes(
            self.data[0x0C..0x10]
                .try_into()
                .expect("fixed record slice"),
        )
    }

    /// Decode the fields consumed by the retail campaign-marker transition
    /// path. Records without flag 0x10 intentionally return `None` rather than
    /// assigning route semantics to other campaign/goal record kinds.
    pub fn marker_transition(&self) -> Option<CampaignMarkerTransition> {
        if self.flags() & CAMPAIGN_MARKER_TRANSITION_FLAG == 0 {
            return None;
        }
        Some(CampaignMarkerTransition {
            destination_logical_level: u32::from_le_bytes(
                self.data[0x00..0x04]
                    .try_into()
                    .expect("fixed record slice"),
            ),
            arrival_raw: [
                i16::from_le_bytes(
                    self.data[0x04..0x06]
                        .try_into()
                        .expect("fixed record slice"),
                ),
                i16::from_le_bytes(
                    self.data[0x06..0x08]
                        .try_into()
                        .expect("fixed record slice"),
                ),
                i16::from_le_bytes(
                    self.data[0x08..0x0A]
                        .try_into()
                        .expect("fixed record slice"),
                ),
            ],
            marker_subtype: self.data[0x1D] as i8,
            marker_args: [self.data[0x1E] as i8, self.data[0x1F] as i8],
        })
    }
}

/// Scene fog pair produced by `FUN_0042E920` and the successful
/// `FUN_00451710` world-load suffix. Each authored dword is shifted left eight
/// bits before the 32-bit subtraction; no terrain-scan cap or saturation runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LevelFogPlanes {
    pub near_raw: i32,
    pub far_raw: i32,
}

impl LevelFogPlanes {
    pub const fn from_authored_cells(far_cells: u32, width_cells: u32) -> Self {
        let far_raw = far_cells.wrapping_shl(8) as i32;
        let width_raw = width_cells.wrapping_shl(8) as i32;
        Self {
            near_raw: far_raw.wrapping_sub(width_raw),
            far_raw,
        }
    }
}

/// A parsed level descriptor from Section 13.
#[derive(Debug)]
pub struct LevelDescriptor {
    /// Exact authored 0xD0-byte descriptor header before pointer relocation.
    /// Unclassified level/render/behavior parameters remain available for
    /// targeted reverse engineering instead of being discarded by parsing.
    pub raw_header: [u8; BASE_SIZE],
    /// Level name (from first 64 bytes, null-terminated ASCII).
    pub name: String,
    /// World-resource selector (+0x48, values 1-6 on game levels). Retail
    /// loads system overlay `5 + world_style`, which supplies this world's
    /// terrain palettes, sprites, static-object descriptors, and model range.
    pub world_style: u32,
    /// Global sprite-pool index of the terrain transition block (+0x4C).
    /// `base..base+119` are the 120 canonical five-material opaque tiles;
    /// marching-squares shoreline frames are `base+125..base+129`.
    pub terrain_sprite_base: u32,
    /// Master-palette colour filling the framebuffer before the world pass
    /// (`FUN_0042F270` → `FUN_0047B9F0`).
    pub sky_color_index: u16,
    /// Optional global model drawn twice over the sky colour with distant
    /// parallax (`FUN_0042F270`; `sky1`/`sky2` on levels 13/14).
    pub sky_model: u16,
    /// Alternate master-palette background selected by the Main Base abort
    /// request (`FUN_0042F1A0`, Section 13 `+0x58`).
    pub main_base_abort_sky_color_index: u16,
    /// Optional global sky model paired with the Main Base abort background
    /// (Section 13 `+0x5A`). A zero value deliberately suppresses the normal
    /// sky model; it is not a missing-resource fallback.
    pub main_base_abort_sky_model: u16,
    /// Authored terrain scan depth. `FUN_00433130` derives the opaque terrain
    /// footprint from this value, capped to 52 columns by 30 rows. The same
    /// authored value is the uncapped world fog far plane in cells.
    pub terrain_draw_depth: u32,
    /// Number of entity spawn sub-entries.
    pub sub_count: u32,
    /// Number of trailing campaign/goal records.
    pub campaign_record_count: u32,
    /// Entity spawn sub-entries.
    pub entities: Vec<EntitySpawn>,
    /// Raw campaign/goal records. Marker-transition records expose their
    /// evidence-backed route fields through `CampaignRecord::marker_transition`.
    pub campaign_records: Vec<CampaignRecord>,
}

impl LevelDescriptor {
    /// Read an aligned or unaligned little-endian dword from the authored base
    /// descriptor. Structural pointer fields are returned in their on-disk
    /// relative form.
    pub fn raw_u32(&self, offset: usize) -> Option<u32> {
        let bytes = self.raw_header.get(offset..offset.checked_add(4)?)?;
        Some(u32::from_le_bytes(bytes.try_into().ok()?))
    }

    /// Authored world fog width in cells (Section 13 `+0x8C`). This word is
    /// also copied unchanged into the normal and Main Base abort frame request.
    pub fn fog_width_cells(&self) -> u32 {
        self.raw_u32(0x8C)
            .expect("Section 13 base header always contains +0x8C")
    }

    /// Raw scene planes installed after a successful world load. These do not
    /// authenticate transient model-local or identity-reset projector planes.
    pub fn fog_planes_raw(&self) -> LevelFogPlanes {
        LevelFogPlanes::from_authored_cells(self.terrain_draw_depth, self.fog_width_cells())
    }

    /// Authored time-trophy deadline in seconds (Section 13 `+0x5C`).
    ///
    /// `FUN_0042E570` consumes this value when it initializes the per-world
    /// campaign controller. Zero is meaningful: it immediately secures the
    /// world's independent time-trophy claim instead of starting a timer.
    pub fn time_trophy_deadline_seconds(&self) -> u32 {
        self.raw_u32(0x5C)
            .expect("Section 13 base header always contains +0x5C")
    }

    /// Runtime terrain-material response selectors built by `FUN_0042EA30`.
    ///
    /// Section-13 dwords `+0xA8..+0xC4` are copied first. Retail then replaces
    /// slot zero with authored slot one unless slot zero is selector 7. The
    /// terrain type byte's low three bits index this eight-entry table.
    pub fn ground_response_selectors(&self) -> [u8; 8] {
        let mut selectors =
            std::array::from_fn(|index| self.raw_u32(0xA8 + index * 4).unwrap_or(0) as u8);
        if selectors[0] != 7 {
            selectors[0] = selectors[1];
        }
        selectors
    }

    /// Runtime water-entry response selectors built by `FUN_0042EA30`.
    /// Retail fills all eight material slots from Section-13 dword `+0xA8`.
    pub fn water_response_selectors(&self) -> [u8; 8] {
        [self.raw_u32(0xA8).unwrap_or(0) as u8; 8]
    }
}

/// Parse Section 13 level descriptor.
///
/// Section 13 has count=1 per PRELOAD.DAT for all levels 13-50.
/// The block contains one base descriptor + sub-entries + campaign records.
pub fn parse_level(data: &[u8]) -> Result<LevelDescriptor> {
    if data.len() < BASE_SIZE {
        return Err(V2kError::section(
            13,
            format!(
                "section 13 too short: {} bytes, need {}",
                data.len(),
                BASE_SIZE
            ),
        ));
    }

    // Level name (64 bytes, null-terminated)
    let name_bytes = &data[..64];
    let name_end = name_bytes.iter().position(|&b| b == 0).unwrap_or(64);
    let name = String::from_utf8_lossy(&name_bytes[..name_end])
        .trim()
        .to_string();

    let world_style = read_u32(data, 0x48)?;
    let terrain_sprite_base = read_u32(data, 0x4C)?;
    let sky_color_index = read_u16(data, 0x54)?;
    let sky_model = read_u16(data, 0x56)?;
    let main_base_abort_sky_color_index = read_u16(data, 0x58)?;
    let main_base_abort_sky_model = read_u16(data, 0x5A)?;
    let terrain_draw_depth = read_u32(data, 0x88)?;
    let campaign_record_count = read_u32(data, 0x64)?;
    let campaign_records_rel = read_u32(data, 0x68)? as usize;
    let sub_count = read_u32(data, 0xC8)?;

    // Sanity checks
    if sub_count > 1000 || campaign_record_count > 1000 {
        return Err(V2kError::section(
            13,
            format!(
                "unreasonable counts: sub_count={}, campaign_record_count={}",
                sub_count, campaign_record_count
            ),
        ));
    }

    // Parse sub-entry pointer array (sub_count × u32, immediately after base)
    let ptr_array_offset = BASE_SIZE;
    let mut entities = Vec::with_capacity(sub_count as usize);

    for si in 0..sub_count as usize {
        let ptr_off = ptr_array_offset + si * 4;
        if ptr_off + 4 > data.len() {
            break;
        }
        let sub_rel = read_u32(data, ptr_off)? as usize;

        if sub_rel + SUB_ENTRY_SIZE > data.len() {
            entities.push(EntitySpawn {
                index: si,
                entity_type: 0,
                pos_data_1: [0; 4],
                pos_data_2: [0; 4],
                param: 0,
                rotation: [0; 3],
                extra: [0; 40],
                initial_damage_buffer_raw: 0,
                model_overrides: [0; 4],
                has_animation: false,
                anim_frames: 0,
                animation: None,
                has_config: false,
                config: None,
            });
            continue;
        }

        let entity_type = read_u32(data, sub_rel + 0x04)?;
        let pos_data_1 = [
            data[sub_rel + 0x08],
            data[sub_rel + 0x09],
            data[sub_rel + 0x0A],
            data[sub_rel + 0x0B],
        ];
        let pos_data_2 = [
            data[sub_rel + 0x0C],
            data[sub_rel + 0x0D],
            data[sub_rel + 0x0E],
            data[sub_rel + 0x0F],
        ];
        let param = read_u32(data, sub_rel + 0x10)?;
        // FUN_004104B0 receives `param_2` at spawn +0x04. Its param_2[7]
        // dword is copied to entity +0xA2/+0xA4 and param_2[8]'s low word to
        // +0xA6, so yaw/pitch/roll live at spawn +0x20/+0x22/+0x24. The old
        // +0x1C start happened to leave yaw in `rotation[2]`, masking the same
        // four-byte constructor-base error that shifted the model slots.
        let rotation = [
            read_u16(data, sub_rel + 0x20)?,
            read_u16(data, sub_rel + 0x22)?,
            read_u16(data, sub_rel + 0x24)?,
        ];
        let mut extra = [0u8; 40];
        extra.copy_from_slice(&data[sub_rel + 0x14..sub_rel + 0x3C]);
        let initial_damage_buffer_raw = read_u32(data, sub_rel + 0x28)? as i32;
        // FUN_004104B0 receives `param_2` at spawn +0x04, then consumes
        // param_2[10..=13]. The four constructor model slots therefore live
        // at spawn +0x2C..=+0x38. Reading from +0x28 shifts the slots left:
        // Intro2's meteors lose their slot-0 `grock`, and its authored peasant
        // hut incorrectly falls back to the type-66 `factory6` model.
        let model_overrides = [
            read_u32(data, sub_rel + 0x2C)?,
            read_u32(data, sub_rel + 0x30)?,
            read_u32(data, sub_rel + 0x34)?,
            read_u32(data, sub_rel + 0x38)?,
        ];
        let anim_ptr = read_u32(data, sub_rel + 0x3C)? as usize;
        let config_ptr = read_u32(data, sub_rel + 0x40)? as usize;

        let has_animation = anim_ptr != 0;
        let has_config = config_ptr != 0;

        let mut anim_frames = 0u32;
        let mut animation = None;
        if has_animation && anim_ptr + 0x14 <= data.len() {
            let anim_count = read_u32(data, anim_ptr + 0x10)?;
            if anim_count < 1000 {
                anim_frames = anim_count;
                if anim_ptr + 0x18 <= data.len() {
                    let mut header = [0u8; 0x18];
                    header.copy_from_slice(&data[anim_ptr..anim_ptr + 0x18]);
                    let frame_ptr = read_u32(data, anim_ptr + 0x14)? as usize;
                    let immediate = anim_ptr + 0x18;
                    let needed = anim_count as usize * 0x1C;
                    let frame_start = if frame_ptr != 0 && frame_ptr + needed <= data.len() {
                        frame_ptr
                    } else {
                        immediate
                    };
                    if frame_start + needed <= data.len() {
                        let mut frames = Vec::with_capacity(anim_count as usize);
                        for frame in 0..anim_count as usize {
                            let start = frame_start + frame * 0x1C;
                            let mut raw = [0u8; 0x1C];
                            raw.copy_from_slice(&data[start..start + 0x1C]);
                            frames.push(raw);
                        }
                        animation = Some(EntityAnimation { header, frames });
                    }
                }
            }
        }

        let config = if has_config && config_ptr + 0x58 <= data.len() {
            let mut raw = [0u8; 0x58];
            raw.copy_from_slice(&data[config_ptr..config_ptr + 0x58]);
            Some(raw)
        } else {
            None
        };

        entities.push(EntitySpawn {
            index: si,
            entity_type,
            pos_data_1,
            pos_data_2,
            param,
            rotation,
            extra,
            initial_damage_buffer_raw,
            model_overrides,
            has_animation,
            anim_frames,
            animation,
            has_config,
            config,
        });
    }

    // +0x68 is the authored relative pointer that FUN_0042D880 relocates for
    // the campaign controller. Entity payloads can contain alignment gaps, so
    // deriving this array from their apparent decoded sizes loses real routes.
    let campaign_bytes = campaign_record_count as usize * CAMPAIGN_RECORD_SIZE;
    if campaign_record_count != 0
        && (campaign_records_rel == 0
            || campaign_records_rel
                .checked_add(campaign_bytes)
                .is_none_or(|end| end > data.len()))
    {
        return Err(V2kError::section(
            13,
            format!(
                "campaign record array out of bounds: offset={}, count={}, data_len={}",
                campaign_records_rel,
                campaign_record_count,
                data.len()
            ),
        ));
    }
    let campaign_record_start = campaign_records_rel;
    let mut campaign_records = Vec::with_capacity(campaign_record_count as usize);
    for record_index in 0..campaign_record_count as usize {
        let record_offset = campaign_record_start + record_index * CAMPAIGN_RECORD_SIZE;
        if record_offset + CAMPAIGN_RECORD_SIZE <= data.len() {
            let mut record_data = [0u8; CAMPAIGN_RECORD_SIZE];
            record_data.copy_from_slice(&data[record_offset..record_offset + CAMPAIGN_RECORD_SIZE]);
            campaign_records.push(CampaignRecord { data: record_data });
        }
    }

    let raw_header = data[..BASE_SIZE]
        .try_into()
        .expect("validated Section 13 base header size");

    Ok(LevelDescriptor {
        raw_header,
        name,
        world_style,
        terrain_sprite_base,
        sky_color_index,
        sky_model,
        main_base_abort_sky_color_index,
        main_base_abort_sky_model,
        terrain_draw_depth,
        sub_count,
        campaign_record_count,
        entities,
        campaign_records,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fog_planes_preserve_native_shift_subtract_and_signed_bits() {
        for (far, width, expected) in [
            (21u32, 8u32, [3328, 5376]),
            (22, 6, [4096, 5632]),
            (40, 12, [7168, 10240]),
            (0, 0, [0, 0]),
            (5, 12, [-1792, 1280]),
            (0, 1, [-256, 0]),
            (0x0080_0000, 1, [0x7fff_ff00, i32::MIN]),
            (0x0100_0000, 0, [0, 0]),
            (u32::MAX, 1, [-512, -256]),
        ] {
            let mut data = vec![0u8; BASE_SIZE];
            data[0x88..0x8C].copy_from_slice(&far.to_le_bytes());
            data[0x8C..0x90].copy_from_slice(&width.to_le_bytes());
            let level = parse_level(&data).unwrap();
            assert_eq!(level.fog_width_cells(), width);
            assert_eq!(
                level.fog_planes_raw(),
                LevelFogPlanes {
                    near_raw: expected[0],
                    far_raw: expected[1],
                },
                "far={far:#x}, width={width:#x}"
            );
        }
    }

    #[test]
    fn parse_empty_level() {
        let mut data = vec![0u8; BASE_SIZE];
        // Name: "Test"
        data[0] = b'T';
        data[1] = b'e';
        data[2] = b's';
        data[3] = b't';
        // sub_count = 0, campaign_record_count = 0 (already zero)

        let level = parse_level(&data).unwrap();
        assert_eq!(level.name, "Test");
        assert_eq!(level.sub_count, 0);
        assert_eq!(level.campaign_record_count, 0);
        assert!(level.entities.is_empty());
        assert!(level.campaign_records.is_empty());
        assert_eq!(&level.raw_header[..4], b"Test");
    }

    #[test]
    fn decodes_only_flagged_campaign_marker_transitions() {
        let mut data = vec![0u8; BASE_SIZE + CAMPAIGN_RECORD_SIZE];
        data[0x64..0x68].copy_from_slice(&1u32.to_le_bytes());
        data[0x68..0x6C].copy_from_slice(&(BASE_SIZE as u32).to_le_bytes());

        let record = &mut data[BASE_SIZE..BASE_SIZE + CAMPAIGN_RECORD_SIZE];
        record[0x00..0x04].copy_from_slice(&18u32.to_le_bytes());
        record[0x04..0x06].copy_from_slice(&7424i16.to_le_bytes());
        record[0x06..0x08].copy_from_slice(&5120i16.to_le_bytes());
        record[0x08..0x0A].copy_from_slice(&9728i16.to_le_bytes());
        record[0x0C..0x10].copy_from_slice(&0x110u32.to_le_bytes());
        record[0x1D] = 2;
        record[0x1E] = 3;
        record[0x1F] = 4;

        let level = parse_level(&data).unwrap();
        assert_eq!(level.campaign_record_count, 1);
        assert_eq!(level.campaign_records.len(), 1);
        let campaign_record = &level.campaign_records[0];
        assert_eq!(campaign_record.flags(), 0x110);
        let transition = campaign_record.marker_transition().unwrap();
        assert_eq!(transition.destination_logical_level, 18);
        assert_eq!(transition.destination_global_level(), Some(30));
        assert_eq!(transition.arrival_raw, [7424, 5120, 9728]);
        assert_eq!(transition.marker_subtype, 2);
        assert_eq!(transition.marker_args, [3, 4]);

        let mut unclassified = campaign_record.clone();
        unclassified.data[0x0C..0x10].copy_from_slice(&0x100u32.to_le_bytes());
        assert_eq!(unclassified.flags(), 0x100);
        assert_eq!(unclassified.marker_transition(), None);
    }

    #[test]
    fn water_fields() {
        let mut data = vec![0u8; BASE_SIZE];
        // Water (level 22): style 6, base 3565 (real values from 0X22XX.OVL).
        data[0x48..0x4C].copy_from_slice(&6u32.to_le_bytes());
        data[0x4C..0x50].copy_from_slice(&3565u32.to_le_bytes());
        data[0x54..0x56].copy_from_slice(&25u16.to_le_bytes());
        data[0x58..0x5A].copy_from_slice(&32u16.to_le_bytes());
        data[0x5A..0x5C].copy_from_slice(&305u16.to_le_bytes());
        data[0x5C..0x60].copy_from_slice(&180u32.to_le_bytes());
        data[0x88..0x8C].copy_from_slice(&22u32.to_le_bytes());
        let level = parse_level(&data).unwrap();
        assert_eq!(level.world_style, 6);
        assert_eq!(level.terrain_sprite_base, 3565);
        assert_eq!(level.sky_color_index, 25);
        assert_eq!(level.sky_model, 0);
        assert_eq!(level.main_base_abort_sky_color_index, 32);
        assert_eq!(level.main_base_abort_sky_model, 305);
        assert_eq!(level.time_trophy_deadline_seconds(), 180);
        assert_eq!(level.terrain_draw_depth, 22);
        assert_eq!(level.raw_u32(0x48), Some(6));
        assert_eq!(level.raw_u32(0x4C), Some(3565));
        assert_eq!(level.raw_u32(BASE_SIZE - 3), None);
    }

    #[test]
    fn builds_retail_ground_and_water_response_selector_maps() {
        let mut data = vec![0u8; BASE_SIZE];
        for (index, selector) in [6u32, 2, 5, 2, 0, 0, 0, 0].into_iter().enumerate() {
            let offset = 0xA8 + index * 4;
            data[offset..offset + 4].copy_from_slice(&selector.to_le_bytes());
        }
        let level = parse_level(&data).unwrap();
        assert_eq!(level.ground_response_selectors(), [2, 2, 5, 2, 0, 0, 0, 0]);
        assert_eq!(level.water_response_selectors(), [6; 8]);

        data[0xA8..0xAC].copy_from_slice(&7u32.to_le_bytes());
        let selector_seven = parse_level(&data).unwrap();
        assert_eq!(selector_seven.ground_response_selectors()[0], 7);
        assert_eq!(selector_seven.water_response_selectors(), [7; 8]);
    }

    #[test]
    fn too_short() {
        assert!(parse_level(&[0u8; 100]).is_err());
    }

    #[test]
    fn unreasonable_counts() {
        let mut data = vec![0u8; BASE_SIZE];
        // sub_count = 9999
        data[0xC8..0xCC].copy_from_slice(&9999u32.to_le_bytes());
        assert!(parse_level(&data).is_err());
    }

    #[test]
    fn preserves_spawn_payload_animation_and_config() {
        let sub = BASE_SIZE + 4;
        let anim = sub + SUB_ENTRY_SIZE;
        let config = anim + 0x18 + 0x1C;
        let mut data = vec![0u8; config + 0x58];
        data[0xC8..0xCC].copy_from_slice(&1u32.to_le_bytes());
        data[BASE_SIZE..BASE_SIZE + 4].copy_from_slice(&(sub as u32).to_le_bytes());
        data[sub + 4..sub + 8].copy_from_slice(&67u32.to_le_bytes());
        for (i, byte) in data[sub + 0x14..sub + 0x3C].iter_mut().enumerate() {
            *byte = i as u8;
        }
        data[sub + 0x2C..sub + 0x3C].fill(0);
        data[sub + 0x2C..sub + 0x30].copy_from_slice(&321u32.to_le_bytes());
        data[sub + 0x3C..sub + 0x40].copy_from_slice(&(anim as u32).to_le_bytes());
        data[sub + 0x40..sub + 0x44].copy_from_slice(&(config as u32).to_le_bytes());
        data[anim + 0x10..anim + 0x14].copy_from_slice(&1u32.to_le_bytes());
        data[anim + 0x14..anim + 0x18].copy_from_slice(&((anim + 0x18) as u32).to_le_bytes());
        data[anim + 0x18..anim + 0x18 + 0x1C].fill(0xA5);
        data[config..config + 0x58].fill(0x5A);

        let level = parse_level(&data).unwrap();
        let spawn = &level.entities[0];
        assert_eq!(spawn.rotation, [0x0D0C, 0x0F0E, 0x1110]);
        assert_eq!(spawn.extra[0], 0);
        assert_eq!(spawn.extra[23], 23);
        assert_eq!(spawn.extra[39], 0);
        assert_eq!(spawn.initial_damage_buffer_raw, 0x1716_1514);
        assert_eq!(spawn.model_overrides[0], 321);
        assert_eq!(spawn.animation.as_ref().unwrap().frames[0], [0xA5; 0x1C]);
        assert_eq!(spawn.config.as_ref().unwrap(), &[0x5A; 0x58]);
    }
}
