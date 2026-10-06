//! Retail weapon inventory descriptors and power-up acquisition.
//!
//! `FUN_00445A90` stores 32 fixed-size weapon records at controller `+0x2A8`.
//! A weapon power-up copies its canonical 24-byte descriptor into the first
//! empty/matching slot, replaces descriptor `+0x04` with the greater of the
//! previous and incoming resource counts, and selects the slot only when that
//! count strictly increases. The default selector-1 record and the Level-1
//! selector-2 record below are byte-exact values recovered from retail memory
//! in the `20260720-143552-powerups-projectiles` capture.
//!
//! Targetter selector `0x3C` and Turbo selector `0x3E` are not weapon records.
//! Retail sets controller capability bits and rejects duplicate acquisition,
//! so both remain separate state machines here rather than consuming one of
//! the 32 weapon slots.

/// Number of fixed-size weapon descriptor slots in the retail controller.
pub const WEAPON_SLOT_COUNT: usize = 32;

/// Default infinite-ammunition machine-gun selector.
pub const DEFAULT_WEAPON_SELECTOR: u8 = 1;
/// Level-1 finite-ammunition tube/flare-gun selector.
pub const LEVEL_ONE_WEAPON_SELECTOR: u8 = 2;
/// Working Factory type-61 payload `0x0001F412` low byte. The incoming count
/// is 500, but master descriptor flags `0x07` make this rapid gun infinite.
pub const FACTORY_LEVEL_ONE_WEAPON_SELECTOR: u8 = 0x12;
/// Retail `FUN_00445A90` weapon branch: selectors below this are table lookups.
pub const WEAPON_SELECTOR_LIMIT: u8 = 0x32;
/// Targetter capability selector handled outside the weapon slots.
pub const TARGETTER_SELECTOR: u8 = 0x3c;
/// Turbo capability selector handled outside the weapon slots.
pub const TURBO_SELECTOR: u8 = 0x3e;

const WEAPON_DESCRIPTOR_BYTES: usize = 0x18;
const AMMO_FIELD_OFFSET: usize = 0x04;
const CALLBACK_SELECTOR_OFFSET: usize = 0x08;
const CADENCE_TICKS_OFFSET: usize = 0x0a;
const FLAGS_OFFSET: usize = 0x0c;
const SOUND_ID_OFFSET: usize = 0x0e;
const HUD_RESOURCE_OFFSET: usize = 0x10;
const PICKUP_TEXT_ARGUMENT_OFFSET: usize = 0x14;
const INFINITE_AMMO_FLAG: u8 = 0x01;
const RETAIL_TICK_MICROS: u64 = 20_000;
const EMPTY_AMMO_DEFAULT_SUCCESSOR: u32 = 0x0c;
const EMPTY_AMMO_SUCCESSORS: [(u32, u32); 8] = [
    (0x12, 0x01),
    (0x0e, 0x12),
    (0x0d, 0x0e),
    (0x0c, 0x0d),
    (0x0a, 0x16),
    (0x13, 0x0a),
    (0x1d, 0x07),
    (0x1c, 0x08),
];

/// Byte-exact runtime descriptor installed in the default weapon slot.
pub const DEFAULT_WEAPON_DESCRIPTOR_RAW: [u8; WEAPON_DESCRIPTOR_BYTES] = [
    0x01, 0x00, 0x00, 0x00, 0x78, 0x03, 0x00, 0x00, 0x00, 0x01, 0x08, 0x05, 0x07, 0x00, 0x58, 0x00,
    0x73, 0x00, 0x06, 0x02, 0x0e, 0x01, 0x00, 0x00,
];

/// Byte-exact selector-1 master descriptor before new-game initialization
/// installs retail's authored infinite-ammunition sentinel (`888`).
pub const DEFAULT_WEAPON_MASTER_DESCRIPTOR_RAW: [u8; WEAPON_DESCRIPTOR_BYTES] = [
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x08, 0x05, 0x07, 0x00, 0x58, 0x00,
    0x73, 0x00, 0x06, 0x02, 0x0e, 0x01, 0x00, 0x00,
];

/// Byte-exact selector-2 master descriptor before acquisition supplies ammo.
pub const LEVEL_ONE_WEAPON_DESCRIPTOR_RAW: [u8; WEAPON_DESCRIPTOR_BYTES] = [
    0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x02, 0x0f, 0x05, 0x04, 0x00, 0x3c, 0x00,
    0x00, 0x00, 0x07, 0x02, 0x04, 0x01, 0x00, 0x00,
];

/// Canonical master table `DAT_004CDC08` .. `DAT_004CDE48` (24 × 0x18).
///
/// `FUN_00445A90` scans these records in order for a matching selector dword.
/// A walk that reaches `DAT_004CDE48` (`iVar7 == 0x18`) is not found.
pub const WEAPON_MASTER_TABLE_RAW: [[u8; WEAPON_DESCRIPTOR_BYTES]; 24] = [
    [
        0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x08, 0x05, 0x07, 0x00, 0x58,
        0x00, 0x73, 0x00, 0x06, 0x02, 0x0e, 0x01, 0x00, 0x00,
    ],
    [
        0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x02, 0x0f, 0x05, 0x04, 0x00, 0x3c,
        0x00, 0x00, 0x00, 0x07, 0x02, 0x04, 0x01, 0x00, 0x00,
    ],
    [
        0x19, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0c, 0x02, 0x50, 0x05, 0x00, 0x00, 0x40,
        0x00, 0x00, 0x00, 0x06, 0x02, 0x00, 0x00, 0x00, 0x00,
    ],
    [
        0x0c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x05, 0x05, 0x03, 0x00, 0x00, 0x4c,
        0x00, 0x75, 0x00, 0x00, 0x00, 0xfe, 0x00, 0x00, 0x00,
    ],
    [
        0x0d, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x07, 0x05, 0x05, 0x03, 0x00, 0x00, 0x4d,
        0x00, 0x77, 0x00, 0x00, 0x00, 0xfd, 0x00, 0x00, 0x00,
    ],
    [
        0x0e, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x06, 0x05, 0x05, 0x03, 0x00, 0x00, 0x4e,
        0x00, 0x76, 0x00, 0x00, 0x00, 0xfc, 0x00, 0x00, 0x00,
    ],
    [
        0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x64, 0x32, 0x03, 0x04, 0x00, 0x4f,
        0x00, 0x00, 0x00, 0x08, 0x02, 0x01, 0x01, 0x00, 0x00,
    ],
    [
        0x0a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x32, 0x14, 0x03, 0x00, 0x00, 0x44,
        0x00, 0x00, 0x00, 0x0b, 0x02, 0x06, 0x01, 0x00, 0x00,
    ],
    [
        0x16, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x32, 0x14, 0x03, 0x00, 0x00, 0x44,
        0x00, 0x00, 0x00, 0x0a, 0x02, 0x05, 0x01, 0x00, 0x00,
    ],
    [
        0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x32, 0x32, 0x02, 0x04, 0x00, 0x3d,
        0x00, 0x78, 0x00, 0x00, 0x00, 0x03, 0x01, 0x00, 0x00,
    ],
    [
        0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x09, 0x32, 0x32, 0x02, 0x04, 0x00, 0x58,
        0x00, 0x79, 0x00, 0x00, 0x00, 0x02, 0x01, 0x00, 0x00,
    ],
    [
        0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x64, 0x14, 0x02, 0x00, 0x00, 0x58,
        0x00, 0x00, 0x00, 0x11, 0x02, 0x07, 0x01, 0x00, 0x00,
    ],
    [
        0x13, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x64, 0x14, 0x02, 0x00, 0x00, 0x58,
        0x00, 0x00, 0x00, 0x0f, 0x02, 0x08, 0x01, 0x00, 0x00,
    ],
    [
        0x09, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0a, 0x64, 0x32, 0x02, 0x00, 0x00, 0x3c,
        0x00, 0x00, 0x00, 0x0d, 0x02, 0x09, 0x01, 0x00, 0x00,
    ],
    [
        0x07, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x02, 0x04, 0x05, 0x00, 0x00, 0x40,
        0x00, 0x00, 0x00, 0x9a, 0x03, 0x0b, 0x01, 0x00, 0x00,
    ],
    [
        0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x02, 0x04, 0x05, 0x00, 0x00, 0x40,
        0x00, 0x00, 0x00, 0x8f, 0x03, 0x0a, 0x01, 0x00, 0x00,
    ],
    [
        0x1d, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0b, 0x02, 0x14, 0x03, 0x00, 0x00, 0x58,
        0x00, 0x00, 0x00, 0x18, 0x02, 0x1b, 0x01, 0x00, 0x00,
    ],
    [
        0x1c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0b, 0x02, 0x14, 0x03, 0x00, 0x00, 0x58,
        0x00, 0x00, 0x00, 0x17, 0x02, 0x1a, 0x01, 0x00, 0x00,
    ],
    [
        0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0b, 0x02, 0x0a, 0x05, 0x00, 0x00, 0x58,
        0x00, 0x00, 0x00, 0x19, 0x03, 0x0c, 0x01, 0x00, 0x00,
    ],
    [
        0x11, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x64, 0x32, 0x03, 0x04, 0x00, 0x4f,
        0x00, 0x00, 0x00, 0x09, 0x02, 0x00, 0x01, 0x00, 0x00,
    ],
    [
        0x12, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0f, 0x01, 0x02, 0x05, 0x07, 0x00, 0x58,
        0x00, 0x74, 0x00, 0x00, 0x00, 0x0e, 0x01, 0x00, 0x00,
    ],
    [
        0x1a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0e, 0x01, 0x02, 0x05, 0x00, 0x00, 0x40,
        0x00, 0x00, 0x00, 0x12, 0x02, 0x0d, 0x01, 0x00, 0x00,
    ],
    [
        0x1b, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x01, 0x14, 0x05, 0x01, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x10, 0x02, 0xff, 0x00, 0x00, 0x00,
    ],
    [
        0x1f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0c, 0x05, 0x0c, 0x03, 0x00, 0x00, 0x44,
        0x00, 0x00, 0x00, 0x13, 0x02, 0x1c, 0x01, 0x00, 0x00,
    ],
];

const EMPTY_WEAPON_DESCRIPTOR_RAW: [u8; WEAPON_DESCRIPTOR_BYTES] = [0; WEAPON_DESCRIPTOR_BYTES];

/// The two retail values packed into a type-61 power-up runtime word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PowerUpPayload {
    /// Low byte of the packed runtime word.
    pub selector: u8,
    /// Arithmetic signed shift of the complete packed word by eight bits.
    pub amount: i32,
}

impl PowerUpPayload {
    /// Decode the value copied from Section-13 extra `+0x1C` to the live
    /// type-61 entity's `+0x88` field.
    ///
    /// Retail effectively performs `(packed as i32) >> 8`; retaining signed
    /// arithmetic matters for malformed or sentinel-bearing source data.
    pub const fn from_runtime_word(packed: u32) -> Self {
        Self {
            selector: (packed & 0xff) as u8,
            amount: (packed as i32) >> 8,
        }
    }
}

/// One byte-exact controller weapon record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeaponDescriptor {
    raw: [u8; WEAPON_DESCRIPTOR_BYTES],
}

impl WeaponDescriptor {
    /// Construct a descriptor without losing fields whose meaning is not yet
    /// proven. Runtime integration should prefer accessors over raw offsets.
    pub const fn from_raw(raw: [u8; WEAPON_DESCRIPTOR_BYTES]) -> Self {
        Self { raw }
    }

    /// Complete 24-byte controller representation.
    pub const fn raw(&self) -> &[u8; WEAPON_DESCRIPTOR_BYTES] {
        &self.raw
    }

    /// Weapon/projectile selector at descriptor `+0x00`.
    pub fn selector(&self) -> u32 {
        read_u32(&self.raw, 0)
    }

    /// Whether this slot is unused.
    pub fn is_empty(&self) -> bool {
        self.selector() == 0
    }

    /// Raw resource/ammunition word at descriptor `+0x04`.
    pub fn stored_resource_count(&self) -> u32 {
        read_u32(&self.raw, AMMO_FIELD_OFFSET)
    }

    /// Selector written to the weapon component's callback/output word.
    pub const fn callback_selector(&self) -> u8 {
        self.raw[CALLBACK_SELECTOR_OFFSET]
    }

    /// Authored 20-ms repeat ticks at descriptor `+0x0A`.
    pub const fn cadence_ticks(&self) -> u8 {
        self.raw[CADENCE_TICKS_OFFSET]
    }

    /// `FUN_00424E70` joint-pulse decay multiplier at descriptor `+0x0B`.
    pub const fn joint_decay_rate(&self) -> u8 {
        self.raw[0x0b]
    }

    /// `FUN_00444FA0` writes `flags >> 2 & 1` into emitter `+0x17`.
    pub const fn emits_auxiliary_command(&self) -> bool {
        self.raw[FLAGS_OFFSET] & 0x04 != 0
    }

    /// Exact held-trigger repeat interval used by `FUN_00444FA0`.
    pub const fn cadence_micros(&self) -> u64 {
        self.cadence_ticks() as u64 * RETAIL_TICK_MICROS
    }

    /// Global sound resource played for one firing event.
    pub fn firing_sound_id(&self) -> u16 {
        read_u16(&self.raw, SOUND_ID_OFFSET)
    }

    /// Low 16 bits of descriptor `+0x10`, when a 3D HUD model is authored.
    pub fn hud_model_id(&self) -> Option<u16> {
        nonzero_u16(read_u16(&self.raw, HUD_RESOURCE_OFFSET))
    }

    /// High 16 bits of descriptor `+0x10`, when a HUD sprite is authored.
    pub fn hud_sprite_id(&self) -> Option<u16> {
        nonzero_u16(read_u16(&self.raw, HUD_RESOURCE_OFFSET + 2))
    }

    /// Retail-preferred HUD presentation: a model when the low word exists,
    /// otherwise the authored sprite in the high word.
    pub fn hud_resource(&self) -> Option<WeaponHudResource> {
        self.hud_model_id()
            .map(WeaponHudResource::Model)
            .or_else(|| self.hud_sprite_id().map(WeaponHudResource::Sprite))
    }

    /// Global Section-2 string supplied as the `%s` argument to direct
    /// pickup text `0xCD`. Retail reads this word from descriptor `+0x14`
    /// after installing/updating the controller slot.
    pub fn pickup_text_argument_id(&self) -> u16 {
        read_u16(&self.raw, PICKUP_TEXT_ARGUMENT_OFFSET)
    }

    /// Deduplicated resource event selected before acquisition. Infinite-
    /// ammunition descriptors use event 6; finite descriptors use event 7.
    pub const fn pickup_resource_event_id(&self) -> u8 {
        if self.raw[FLAGS_OFFSET] & INFINITE_AMMO_FLAG != 0 {
            6
        } else {
            7
        }
    }

    /// Typed ammunition state. Flag bit zero makes the stored `888` value a
    /// retail sentinel rather than a finite round count.
    pub fn ammunition(&self) -> Ammunition {
        if self.raw[FLAGS_OFFSET] & INFINITE_AMMO_FLAG != 0 {
            Ammunition::Infinite
        } else {
            Ammunition::Finite(self.stored_resource_count())
        }
    }

    fn has_usable_ammunition(&self) -> bool {
        match self.ammunition() {
            Ammunition::Infinite => true,
            Ammunition::Finite(rounds) => rounds != 0,
        }
    }

    fn set_stored_resource_count(&mut self, count: u32) {
        self.raw[AMMO_FIELD_OFFSET..AMMO_FIELD_OFFSET + 4].copy_from_slice(&count.to_le_bytes());
    }

    fn commit_round(&mut self) -> bool {
        match self.ammunition() {
            Ammunition::Infinite => true,
            Ammunition::Finite(0) => false,
            Ammunition::Finite(count) => {
                self.set_stored_resource_count(count - 1);
                true
            }
        }
    }
}

impl Default for WeaponDescriptor {
    fn default() -> Self {
        Self::from_raw(EMPTY_WEAPON_DESCRIPTOR_RAW)
    }
}

/// HUD resource policy encoded in descriptor `+0x10`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeaponHudResource {
    Model(u16),
    Sprite(u16),
}

/// Ammunition semantics encoded by descriptor `+0x04/+0x0C`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ammunition {
    Infinite,
    Finite(u32),
}

/// Direction used by the retail next/previous weapon controls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeaponCycleDirection {
    Forward,
    Backward,
}

/// Why a weapon power-up could not mutate the inventory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeaponAcquisitionRejection {
    UnsupportedSelector,
    InventoryFull,
}

/// Result of the max-not-add weapon acquisition transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeaponAcquisition {
    Accepted {
        slot: usize,
        previous_resource_count: u32,
        resource_count: u32,
        auto_selected: bool,
    },
    Rejected(WeaponAcquisitionRejection),
}

/// Result of committing one round from the selected descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmmoCommit {
    Fired {
        slot: usize,
        selector: u32,
        remaining: Ammunition,
        /// Slot selected by retail's empty-ammunition successor graph after
        /// this shot consumed the final finite round.
        automatic_successor_slot: Option<usize>,
    },
    Empty {
        slot: usize,
        selector: u32,
    },
}

/// Fixed 32-slot retail weapon inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeaponInventory {
    slots: [WeaponDescriptor; WEAPON_SLOT_COUNT],
    selected_slot: usize,
    last_fired_slot: usize,
}

impl WeaponInventory {
    /// Construct the retail new-game inventory with selector 1 in slot zero.
    pub fn new() -> Self {
        let mut slots = [WeaponDescriptor::default(); WEAPON_SLOT_COUNT];
        slots[0] = WeaponDescriptor::from_raw(DEFAULT_WEAPON_DESCRIPTOR_RAW);
        Self {
            slots,
            selected_slot: 0,
            last_fired_slot: 0,
        }
    }

    /// Restore 32 inventory slots from the 0x248-byte native retail save state
    /// payload matching `FUN_00443560`.
    pub fn restore_from_native_payload(payload: &[u8; 0x248]) -> Option<Self> {
        let mut slots = [WeaponDescriptor::default(); WEAPON_SLOT_COUNT];
        let weapons_offset = 0x40;
        let mut slot_index = 0;
        while slot_index < WEAPON_SLOT_COUNT {
            let offset = weapons_offset + slot_index * 8;
            let selector = u32::from_le_bytes([
                payload[offset],
                payload[offset + 1],
                payload[offset + 2],
                payload[offset + 3],
            ]);
            if selector == 0 {
                break;
            }
            let ammo = u32::from_le_bytes([
                payload[offset + 4],
                payload[offset + 5],
                payload[offset + 6],
                payload[offset + 7],
            ]);
            let mut descriptor = master_descriptor(u8::try_from(selector).ok()?)?;
            descriptor.set_stored_resource_count(ammo);
            slots[slot_index] = descriptor;
            slot_index += 1;
        }

        let selected_byte = payload[0x146] as usize;
        // Retail restores the exact selected byte. Invalid/empty selections
        // and unknown descriptors would require stale controller state; reject
        // them instead of inventing a default gun or selecting another slot.
        if selected_byte >= WEAPON_SLOT_COUNT || slots[selected_byte].is_empty() {
            return None;
        }
        let selected_slot = selected_byte;

        Some(Self {
            slots,
            selected_slot,
            last_fired_slot: selected_slot,
        })
    }

    /// Encode the 32 inventory slots and selected weapon slot into the 0x248-byte
    /// native retail save state payload matching `FUN_00443260`.
    pub fn encode_into_native_payload(&self, payload: &mut [u8; 0x248]) {
        let weapons_offset = 0x40;
        for (index, descriptor) in self.slots.iter().enumerate() {
            let offset = weapons_offset + index * 8;
            if descriptor.is_empty() {
                payload[offset..offset + 8].fill(0);
            } else {
                let selector = descriptor.selector();
                let ammo = descriptor.stored_resource_count();
                payload[offset..offset + 4].copy_from_slice(&selector.to_le_bytes());
                payload[offset + 4..offset + 8].copy_from_slice(&ammo.to_le_bytes());
            }
        }
        payload[0x146] = self.selected_slot as u8;
    }

    /// Immutable access to all controller slots in retail order.
    pub const fn slots(&self) -> &[WeaponDescriptor; WEAPON_SLOT_COUNT] {
        &self.slots
    }

    /// One descriptor by controller slot.
    pub fn slot(&self, index: usize) -> Option<&WeaponDescriptor> {
        self.slots.get(index)
    }

    /// Currently selected controller slot.
    pub const fn selected_slot(&self) -> usize {
        self.selected_slot
    }

    /// Slot whose ammunition was most recently committed successfully.
    pub const fn last_fired_slot(&self) -> usize {
        self.last_fired_slot
    }

    /// Descriptor currently selected for firing and HUD presentation.
    pub const fn selected_descriptor(&self) -> &WeaponDescriptor {
        &self.slots[self.selected_slot]
    }

    /// Typed ammunition state for the selected descriptor.
    pub fn selected_ammunition(&self) -> Ammunition {
        self.selected_descriptor().ammunition()
    }

    /// Number of non-empty descriptor slots.
    pub fn occupied_slot_count(&self) -> usize {
        self.slots.iter().filter(|slot| !slot.is_empty()).count()
    }

    /// Apply the weapon branch of `FUN_00445A90`.
    ///
    /// Selectors below `0x32` are accepted when `DAT_004CDC08` contains a
    /// matching master record. The operation is transactional: unsupported
    /// or full acquisitions leave every slot and selection field unchanged.
    pub fn acquire_weapon(&mut self, payload: PowerUpPayload) -> WeaponAcquisition {
        let Some(master) = master_descriptor(payload.selector) else {
            return WeaponAcquisition::Rejected(WeaponAcquisitionRejection::UnsupportedSelector);
        };

        let selector = u32::from(payload.selector);
        let Some(slot_index) = self
            .slots
            .iter()
            .position(|slot| slot.is_empty() || slot.selector() == selector)
        else {
            return WeaponAcquisition::Rejected(WeaponAcquisitionRejection::InventoryFull);
        };

        let was_empty = self.slots[slot_index].is_empty();
        if was_empty {
            self.slots[slot_index] = master;
        }

        let previous = self.slots[slot_index].stored_resource_count();
        let incoming_increases = (previous as i32) < payload.amount;
        if (previous as i32) <= payload.amount {
            self.slots[slot_index].set_stored_resource_count(payload.amount as u32);
        }
        if incoming_increases {
            self.selected_slot = slot_index;
        }

        WeaponAcquisition::Accepted {
            slot: slot_index,
            previous_resource_count: previous,
            resource_count: self.slots[slot_index].stored_resource_count(),
            auto_selected: incoming_increases,
        }
    }

    /// Move to the next/previous usable occupied slot with wraparound.
    ///
    /// `FUN_004440D0` calls the next/previous slot callback, then rejects a
    /// finite zero count at `0x00444129..0x00444145`. The descriptor remains
    /// owned for HUD transitions, persistence and later reacquisition; its
    /// selector is never cleared by this selection path. Flag-bit-0 infinite
    /// descriptors remain usable even when their stored count is zero.
    pub fn cycle(&mut self, direction: WeaponCycleDirection) -> usize {
        for distance in 1..=WEAPON_SLOT_COUNT {
            let index = match direction {
                WeaponCycleDirection::Forward => {
                    (self.selected_slot + distance) % WEAPON_SLOT_COUNT
                }
                WeaponCycleDirection::Backward => {
                    (self.selected_slot + WEAPON_SLOT_COUNT - distance) % WEAPON_SLOT_COUNT
                }
            };
            if !self.slots[index].is_empty() && self.slots[index].has_usable_ammunition() {
                self.selected_slot = index;
                break;
            }
        }
        self.selected_slot
    }

    /// Consume one finite round, or acknowledge an infinite-ammo fire.
    ///
    /// `FUN_00444FA0` calls `FUN_00445290` only after a successful shot leaves
    /// a finite descriptor at zero. The latter walks the authored selector
    /// graph at executable VA `0x004CDE48`, skipping missing and empty owned
    /// slots until it finds usable ammunition. A restored finite-zero
    /// selection remains selected when firing is rejected; a manual cycle
    /// subsequently skips it through the separate `FUN_004440D0` policy.
    pub fn commit_selected_round(&mut self) -> AmmoCommit {
        let slot = self.selected_slot;
        let selector = self.slots[slot].selector();
        if self.slots[slot].commit_round() {
            self.last_fired_slot = slot;
            let remaining = self.slots[slot].ammunition();
            let automatic_successor_slot = if remaining == Ammunition::Finite(0) {
                self.first_usable_successor_slot(selector)
            } else {
                None
            };
            if let Some(successor_slot) = automatic_successor_slot {
                self.selected_slot = successor_slot;
            }
            AmmoCommit::Fired {
                slot,
                selector,
                remaining,
                automatic_successor_slot,
            }
        } else {
            AmmoCommit::Empty { slot, selector }
        }
    }

    fn first_usable_successor_slot(&self, exhausted_selector: u32) -> Option<usize> {
        let mut selector = exhausted_selector;
        let mut visited = [false; u8::MAX as usize + 1];

        loop {
            selector = empty_ammo_successor(selector);
            let selector_index = selector as usize;
            if visited[selector_index] {
                // Retail assumes selector 1's infinite descriptor is always
                // installed. A malformed port-side inventory must fail closed
                // instead of reproducing retail's otherwise endless loop.
                return None;
            }
            visited[selector_index] = true;

            if let Some(slot) = self
                .slots
                .iter()
                .position(|descriptor| descriptor.selector() == selector)
            {
                if self.slots[slot].has_usable_ammunition() {
                    return Some(slot);
                }
            }
        }
    }
}

impl Default for WeaponInventory {
    fn default() -> Self {
        Self::new()
    }
}

/// Player capabilities acquired outside the weapon descriptor array.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PlayerCapabilities {
    targetter: bool,
    turbo: bool,
}

impl PlayerCapabilities {
    /// 443560 tests these two bits in saved-profile+123 (state+147).
    pub const fn from_native_flags(flags: u8) -> Self {
        Self {
            targetter: flags & 1 != 0,
            turbo: flags & 2 != 0,
        }
    }
    pub const fn has_targetter(&self) -> bool {
        self.targetter
    }

    pub const fn has_turbo(&self) -> bool {
        self.turbo
    }

    /// Acquire selector `0x3C` exactly once. Duplicates and every other
    /// selector leave capability state unchanged.
    pub fn acquire_targetter(&mut self, payload: PowerUpPayload) -> TargetterAcquisition {
        if payload.selector != TARGETTER_SELECTOR {
            return TargetterAcquisition::RejectedUnsupportedSelector;
        }
        if self.targetter {
            return TargetterAcquisition::AlreadyAcquired;
        }
        self.targetter = true;
        TargetterAcquisition::Acquired
    }

    /// Acquire selector `0x3E` exactly once. Duplicates and every other
    /// selector leave capability state unchanged.
    pub fn acquire_turbo(&mut self, payload: PowerUpPayload) -> TurboAcquisition {
        if payload.selector != TURBO_SELECTOR {
            return TurboAcquisition::RejectedUnsupportedSelector;
        }
        if self.turbo {
            return TurboAcquisition::AlreadyAcquired;
        }
        self.turbo = true;
        TurboAcquisition::Acquired
    }
}

/// Result of the separate targetter-capability transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetterAcquisition {
    Acquired,
    AlreadyAcquired,
    RejectedUnsupportedSelector,
}

/// Result of the separate Turbo-capability transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurboAcquisition {
    Acquired,
    AlreadyAcquired,
    RejectedUnsupportedSelector,
}

fn master_descriptor(selector: u8) -> Option<WeaponDescriptor> {
    if selector >= WEAPON_SELECTOR_LIMIT {
        return None;
    }
    WEAPON_MASTER_TABLE_RAW.iter().find_map(|raw| {
        let descriptor = WeaponDescriptor::from_raw(*raw);
        (descriptor.selector() == u32::from(selector)).then_some(descriptor)
    })
}

fn empty_ammo_successor(selector: u32) -> u32 {
    EMPTY_AMMO_SUCCESSORS
        .iter()
        .find_map(|&(from, to)| (from == selector).then_some(to))
        .unwrap_or(EMPTY_AMMO_DEFAULT_SUCCESSOR)
}

fn read_u16(raw: &[u8; WEAPON_DESCRIPTOR_BYTES], offset: usize) -> u16 {
    u16::from_le_bytes([raw[offset], raw[offset + 1]])
}

fn read_u32(raw: &[u8; WEAPON_DESCRIPTOR_BYTES], offset: usize) -> u32 {
    u32::from_le_bytes([
        raw[offset],
        raw[offset + 1],
        raw[offset + 2],
        raw[offset + 3],
    ])
}

const fn nonzero_u16(value: u16) -> Option<u16> {
    if value == 0 {
        None
    } else {
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn selector_two(amount: i32) -> PowerUpPayload {
        PowerUpPayload {
            selector: LEVEL_ONE_WEAPON_SELECTOR,
            amount,
        }
    }

    #[test]
    fn runtime_payload_uses_low_selector_and_signed_arithmetic_amount() {
        assert_eq!(
            PowerUpPayload::from_runtime_word(0x0000_c802),
            selector_two(200)
        );
        assert_eq!(
            PowerUpPayload::from_runtime_word(0xffff_ff02),
            selector_two(-1)
        );
    }

    #[test]
    fn new_inventory_contains_exact_infinite_selector_one_descriptor() {
        let inventory = WeaponInventory::new();
        let descriptor = inventory.selected_descriptor();

        assert_eq!(inventory.occupied_slot_count(), 1);
        assert_eq!(inventory.selected_slot(), 0);
        assert_eq!(inventory.last_fired_slot(), 0);
        assert_eq!(descriptor.raw(), &DEFAULT_WEAPON_DESCRIPTOR_RAW);
        assert_eq!(descriptor.selector(), 1);
        assert_eq!(descriptor.stored_resource_count(), 888);
        assert_eq!(descriptor.ammunition(), Ammunition::Infinite);
        assert_eq!(descriptor.callback_selector(), 0);
        assert_eq!(descriptor.cadence_micros(), 160_000);
        assert_eq!(descriptor.firing_sound_id(), 88);
        assert_eq!(descriptor.hud_model_id(), Some(115));
        assert_eq!(descriptor.hud_sprite_id(), Some(518));
        assert_eq!(
            descriptor.hud_resource(),
            Some(WeaponHudResource::Model(115))
        );
    }

    #[test]
    fn master_table_starts_with_the_recovered_selector_one_and_two_records() {
        assert_eq!(
            WEAPON_MASTER_TABLE_RAW[0],
            DEFAULT_WEAPON_MASTER_DESCRIPTOR_RAW
        );
        assert_eq!(WEAPON_MASTER_TABLE_RAW[1], LEVEL_ONE_WEAPON_DESCRIPTOR_RAW);
        assert_eq!(
            master_descriptor(FACTORY_LEVEL_ONE_WEAPON_SELECTOR)
                .expect("factory selector 0x12 is in DAT_004CDC08")
                .selector(),
            u32::from(FACTORY_LEVEL_ONE_WEAPON_SELECTOR)
        );
        assert_eq!(master_descriptor(0x32), None);
        assert_eq!(master_descriptor(0x15), None);
    }

    #[test]
    fn factory_level_one_payload_installs_selector_eighteen() {
        let mut inventory = WeaponInventory::new();
        let payload = PowerUpPayload::from_runtime_word(0x0001_f412);
        assert_eq!(
            payload,
            PowerUpPayload {
                selector: FACTORY_LEVEL_ONE_WEAPON_SELECTOR,
                amount: 500,
            }
        );
        assert_eq!(
            inventory.acquire_weapon(payload),
            WeaponAcquisition::Accepted {
                slot: 1,
                previous_resource_count: 0,
                resource_count: 500,
                auto_selected: true,
            }
        );
        let descriptor = inventory.selected_descriptor();
        assert_eq!(descriptor.selector(), 0x12);
        assert_eq!(descriptor.stored_resource_count(), 500);
        assert_eq!(descriptor.callback_selector(), 0x0f);
        assert_eq!(descriptor.cadence_micros(), 40_000);
        assert_eq!(descriptor.firing_sound_id(), 88);
        assert_eq!(descriptor.ammunition(), Ammunition::Infinite);
        assert!(descriptor.emits_auxiliary_command());
        assert_eq!(descriptor.joint_decay_rate(), 5);
        assert_eq!(descriptor.hud_model_id(), Some(116));
    }

    #[test]
    fn selector_two_acquisition_installs_exact_descriptor_and_auto_selects() {
        let mut inventory = WeaponInventory::new();

        assert_eq!(
            inventory.acquire_weapon(selector_two(200)),
            WeaponAcquisition::Accepted {
                slot: 1,
                previous_resource_count: 0,
                resource_count: 200,
                auto_selected: true,
            }
        );

        let descriptor = inventory.selected_descriptor();
        let mut expected = LEVEL_ONE_WEAPON_DESCRIPTOR_RAW;
        expected[4..8].copy_from_slice(&200_u32.to_le_bytes());
        assert_eq!(inventory.selected_slot(), 1);
        assert_eq!(inventory.occupied_slot_count(), 2);
        assert_eq!(descriptor.raw(), &expected);
        assert_eq!(descriptor.callback_selector(), 4);
        assert_eq!(descriptor.cadence_micros(), 300_000);
        assert_eq!(descriptor.firing_sound_id(), 60);
        assert_eq!(descriptor.ammunition(), Ammunition::Finite(200));
        assert_eq!(descriptor.hud_model_id(), None);
        assert_eq!(descriptor.hud_sprite_id(), Some(519));
        assert_eq!(
            descriptor.hud_resource(),
            Some(WeaponHudResource::Sprite(519))
        );
    }

    #[test]
    fn repeated_selector_two_acquisition_takes_max_without_adding() {
        let mut inventory = WeaponInventory::new();
        inventory.acquire_weapon(selector_two(200));
        inventory.cycle(WeaponCycleDirection::Backward);
        assert_eq!(inventory.selected_slot(), 0);

        assert_eq!(
            inventory.acquire_weapon(selector_two(150)),
            WeaponAcquisition::Accepted {
                slot: 1,
                previous_resource_count: 200,
                resource_count: 200,
                auto_selected: false,
            }
        );
        assert_eq!(inventory.selected_slot(), 0);

        assert_eq!(
            inventory.acquire_weapon(selector_two(200)),
            WeaponAcquisition::Accepted {
                slot: 1,
                previous_resource_count: 200,
                resource_count: 200,
                auto_selected: false,
            }
        );
        assert_eq!(inventory.selected_slot(), 0);

        assert_eq!(
            inventory.acquire_weapon(selector_two(250)),
            WeaponAcquisition::Accepted {
                slot: 1,
                previous_resource_count: 200,
                resource_count: 250,
                auto_selected: true,
            }
        );
        assert_eq!(inventory.selected_slot(), 1);
    }

    #[test]
    fn cycling_wraps_in_both_directions_and_skips_empty_slots() {
        let mut inventory = WeaponInventory::new();
        inventory.acquire_weapon(selector_two(200));

        assert_eq!(inventory.cycle(WeaponCycleDirection::Forward), 0);
        assert_eq!(inventory.cycle(WeaponCycleDirection::Forward), 1);
        assert_eq!(inventory.cycle(WeaponCycleDirection::Backward), 0);
        assert_eq!(inventory.cycle(WeaponCycleDirection::Backward), 1);
    }

    #[test]
    fn ammo_commit_preserves_infinite_and_decrements_finite_to_empty() {
        let mut inventory = WeaponInventory::new();
        assert_eq!(
            inventory.commit_selected_round(),
            AmmoCommit::Fired {
                slot: 0,
                selector: 1,
                remaining: Ammunition::Infinite,
                automatic_successor_slot: None,
            }
        );
        assert_eq!(inventory.selected_descriptor().stored_resource_count(), 888);

        inventory.acquire_weapon(selector_two(2));
        assert_eq!(
            inventory.commit_selected_round(),
            AmmoCommit::Fired {
                slot: 1,
                selector: 2,
                remaining: Ammunition::Finite(1),
                automatic_successor_slot: None,
            }
        );
        assert_eq!(inventory.last_fired_slot(), 1);
        assert_eq!(
            inventory.commit_selected_round(),
            AmmoCommit::Fired {
                slot: 1,
                selector: 2,
                remaining: Ammunition::Finite(0),
                automatic_successor_slot: Some(0),
            }
        );
        assert_eq!(inventory.selected_slot(), 0);
        assert_eq!(inventory.selected_ammunition(), Ammunition::Infinite);

        // Retail retains the descriptor but both manual directions skip it.
        assert_eq!(inventory.slot(1).unwrap().selector(), 2);
        assert_eq!(
            inventory.slot(1).unwrap().ammunition(),
            Ammunition::Finite(0)
        );
        assert_eq!(inventory.cycle(WeaponCycleDirection::Forward), 0);
        assert_eq!(inventory.cycle(WeaponCycleDirection::Backward), 0);

        // A native saved selection can still point to the retained zero count;
        // an unfireable visit does not run the post-shot successor graph.
        inventory.selected_slot = 1;
        assert_eq!(
            inventory.commit_selected_round(),
            AmmoCommit::Empty {
                slot: 1,
                selector: 2,
            }
        );
        assert_eq!(inventory.selected_ammunition(), Ammunition::Finite(0));
    }

    #[test]
    fn manual_cycles_skip_exhausted_slots_between_usable_finite_weapons() {
        let mut inventory = WeaponInventory::new();
        inventory.acquire_weapon(PowerUpPayload {
            selector: 7,
            amount: 2,
        });
        inventory.acquire_weapon(selector_two(1));
        assert!(matches!(
            inventory.commit_selected_round(),
            AmmoCommit::Fired { .. }
        ));
        inventory.acquire_weapon(PowerUpPayload {
            selector: 8,
            amount: 5,
        });

        assert_eq!(inventory.cycle(WeaponCycleDirection::Forward), 0);
        assert_eq!(inventory.cycle(WeaponCycleDirection::Forward), 1);
        assert_eq!(inventory.cycle(WeaponCycleDirection::Forward), 3);
        assert_eq!(inventory.cycle(WeaponCycleDirection::Backward), 1);
        assert_eq!(inventory.cycle(WeaponCycleDirection::Backward), 0);
        assert_eq!(inventory.occupied_slot_count(), 4);
        assert_eq!(inventory.slot(2).unwrap().selector(), 2);
        assert_eq!(
            inventory.slot(2).unwrap().ammunition(),
            Ammunition::Finite(0)
        );
    }

    #[test]
    fn exhausted_descriptor_reacquisition_reuses_its_slot_and_only_an_increase_selects() {
        let mut inventory = WeaponInventory::new();
        inventory.acquire_weapon(selector_two(1));
        inventory.commit_selected_round();
        assert_eq!(inventory.selected_slot(), 0);
        let exhausted = *inventory.slot(1).unwrap();

        assert_eq!(
            inventory.acquire_weapon(selector_two(0)),
            WeaponAcquisition::Accepted {
                slot: 1,
                previous_resource_count: 0,
                resource_count: 0,
                auto_selected: false,
            }
        );
        assert_eq!(inventory.selected_slot(), 0);
        assert_eq!(inventory.slot(1), Some(&exhausted));
        assert_eq!(
            inventory.acquire_weapon(selector_two(5)),
            WeaponAcquisition::Accepted {
                slot: 1,
                previous_resource_count: 0,
                resource_count: 5,
                auto_selected: true,
            }
        );
        assert_eq!(inventory.selected_slot(), 1);
        assert_eq!(inventory.occupied_slot_count(), 2);
        let replenished = *inventory.slot(1).unwrap();
        assert_eq!(&replenished.raw()[8..], &exhausted.raw()[8..]);
        inventory.cycle(WeaponCycleDirection::Backward);
        assert_eq!(
            inventory.acquire_weapon(selector_two(3)),
            WeaponAcquisition::Accepted {
                slot: 1,
                previous_resource_count: 5,
                resource_count: 5,
                auto_selected: false,
            }
        );
        assert_eq!(inventory.selected_slot(), 0);
    }

    #[test]
    fn all_authored_infinite_weapons_cycle_and_fire_with_a_zero_stored_count() {
        let infinite_selectors: Vec<_> = WEAPON_MASTER_TABLE_RAW
            .iter()
            .map(|raw| WeaponDescriptor::from_raw(*raw))
            .filter(|descriptor| descriptor.ammunition() == Ammunition::Infinite)
            .map(|descriptor| descriptor.selector())
            .collect();
        assert_eq!(infinite_selectors, [1, 0x12, 0x1b]);
        for selector in infinite_selectors {
            let mut inventory = WeaponInventory::new();
            inventory.slots[0].set_stored_resource_count(0);
            if selector != 1 {
                inventory.acquire_weapon(PowerUpPayload {
                    selector: selector as u8,
                    amount: 0,
                });
                assert_eq!(inventory.cycle(WeaponCycleDirection::Forward), 1);
            }
            for _ in 0..3 {
                assert!(
                    matches!(inventory.commit_selected_round(), AmmoCommit::Fired {
                    selector: fired_selector, remaining: Ammunition::Infinite,
                    automatic_successor_slot: None, ..
                } if fired_selector == selector)
                );
            }
            assert_eq!(inventory.selected_descriptor().stored_resource_count(), 0);
        }
    }

    #[test]
    fn every_authored_finite_weapon_retains_its_descriptor_and_skips_manual_reselection() {
        let mut finite_count = 0;
        for raw in WEAPON_MASTER_TABLE_RAW {
            let descriptor = WeaponDescriptor::from_raw(raw);
            if descriptor.ammunition() == Ammunition::Infinite {
                continue;
            }
            finite_count += 1;
            let mut inventory = WeaponInventory::new();
            inventory.acquire_weapon(PowerUpPayload {
                selector: descriptor.selector() as u8,
                amount: 1,
            });
            assert!(
                matches!(inventory.commit_selected_round(), AmmoCommit::Fired {
                slot: 1, selector, remaining: Ammunition::Finite(0),
                automatic_successor_slot: Some(0),
            } if selector == descriptor.selector())
            );
            assert_eq!(inventory.slot(1), Some(&descriptor));
            assert_eq!(inventory.occupied_slot_count(), 2);
            assert_eq!(inventory.cycle(WeaponCycleDirection::Forward), 0);
            assert_eq!(inventory.cycle(WeaponCycleDirection::Backward), 0);
        }
        assert_eq!(finite_count, 21);
    }

    #[test]
    fn empty_ammo_successor_skips_missing_and_empty_intermediate_selectors() {
        let mut inventory = WeaponInventory::new();
        inventory.acquire_weapon(selector_two(1));

        let mut empty_twelve = [0; WEAPON_DESCRIPTOR_BYTES];
        empty_twelve[..4].copy_from_slice(&0x0c_u32.to_le_bytes());
        inventory.slots[2] = WeaponDescriptor::from_raw(empty_twelve);

        let mut usable_thirteen = [0; WEAPON_DESCRIPTOR_BYTES];
        usable_thirteen[..4].copy_from_slice(&0x0d_u32.to_le_bytes());
        usable_thirteen[AMMO_FIELD_OFFSET..AMMO_FIELD_OFFSET + 4]
            .copy_from_slice(&3_u32.to_le_bytes());
        inventory.slots[3] = WeaponDescriptor::from_raw(usable_thirteen);

        assert_eq!(
            inventory.commit_selected_round(),
            AmmoCommit::Fired {
                slot: 1,
                selector: 2,
                remaining: Ammunition::Finite(0),
                automatic_successor_slot: Some(3),
            }
        );
        assert_eq!(inventory.selected_slot(), 3);
        assert_eq!(inventory.selected_ammunition(), Ammunition::Finite(3));
    }

    #[test]
    fn empty_ammo_successor_table_matches_the_retail_executable() {
        for (selector, successor) in EMPTY_AMMO_SUCCESSORS {
            assert_eq!(empty_ammo_successor(selector), successor);
        }
        assert_eq!(empty_ammo_successor(0x02), EMPTY_AMMO_DEFAULT_SUCCESSOR);
        assert_eq!(
            empty_ammo_successor(0xffff_ffff),
            EMPTY_AMMO_DEFAULT_SUCCESSOR
        );
    }

    #[test]
    fn malformed_inventory_without_usable_successor_fails_closed() {
        let mut inventory = WeaponInventory::new();
        inventory.slots[0].raw[FLAGS_OFFSET] &= !INFINITE_AMMO_FLAG;
        inventory.slots[0].set_stored_resource_count(0);
        inventory.acquire_weapon(selector_two(1));

        assert_eq!(
            inventory.commit_selected_round(),
            AmmoCommit::Fired {
                slot: 1,
                selector: 2,
                remaining: Ammunition::Finite(0),
                automatic_successor_slot: None,
            }
        );
        assert_eq!(inventory.selected_slot(), 1);
    }

    #[test]
    fn captured_selector_two_burst_commits_exactly_thirty_seven_rounds() {
        let mut inventory = WeaponInventory::new();
        inventory.acquire_weapon(selector_two(200));

        for _ in 0..37 {
            assert!(matches!(
                inventory.commit_selected_round(),
                AmmoCommit::Fired { .. }
            ));
        }

        assert_eq!(inventory.selected_ammunition(), Ammunition::Finite(163));
    }

    #[test]
    fn pickup_feedback_fields_come_from_the_installed_descriptor() {
        let default = WeaponDescriptor::from_raw(DEFAULT_WEAPON_DESCRIPTOR_RAW);
        assert_eq!(default.pickup_resource_event_id(), 6);
        assert_eq!(default.pickup_text_argument_id(), 0x010e);

        let upgraded = WeaponDescriptor::from_raw(LEVEL_ONE_WEAPON_DESCRIPTOR_RAW);
        assert_eq!(upgraded.pickup_resource_event_id(), 7);
        assert_eq!(upgraded.pickup_text_argument_id(), 0x0104);
    }

    #[test]
    fn targetter_is_separate_and_idempotent() {
        let mut capabilities = PlayerCapabilities::default();
        let targetter = PowerUpPayload {
            selector: TARGETTER_SELECTOR,
            amount: 0,
        };

        assert_eq!(
            capabilities.acquire_targetter(targetter),
            TargetterAcquisition::Acquired
        );
        assert!(capabilities.has_targetter());
        assert_eq!(
            capabilities.acquire_targetter(targetter),
            TargetterAcquisition::AlreadyAcquired
        );
        assert!(capabilities.has_targetter());
    }

    #[test]
    fn turbo_is_separate_idempotent_and_default_cleared() {
        let mut capabilities = PlayerCapabilities::default();
        let turbo = PowerUpPayload {
            selector: TURBO_SELECTOR,
            amount: 0,
        };

        assert_eq!(
            capabilities.acquire_turbo(turbo),
            TurboAcquisition::Acquired
        );
        assert!(capabilities.has_turbo());
        assert_eq!(
            capabilities.acquire_turbo(turbo),
            TurboAcquisition::AlreadyAcquired
        );

        // `main` deliberately keeps this campaign owner across ordinary
        // in-memory world replacement; copying the owner must retain both
        // capability bits. Returning to the frontend starts a new owner from
        // `Default`, which clears them.
        let next_world = capabilities;
        assert!(next_world.has_turbo());
        assert!(!PlayerCapabilities::default().has_turbo());
    }

    #[test]
    fn selector_3f_and_unknown_selectors_reject_without_mutation() {
        for selector in [0x3f, 0xfe] {
            let payload = PowerUpPayload {
                selector,
                amount: 123,
            };
            let mut inventory = WeaponInventory::new();
            let inventory_before = inventory.clone();
            assert_eq!(
                inventory.acquire_weapon(payload),
                WeaponAcquisition::Rejected(WeaponAcquisitionRejection::UnsupportedSelector)
            );
            assert_eq!(inventory, inventory_before);

            let mut capabilities = PlayerCapabilities::default();
            let capabilities_before = capabilities;
            assert_eq!(
                capabilities.acquire_targetter(payload),
                TargetterAcquisition::RejectedUnsupportedSelector
            );
            assert_eq!(capabilities, capabilities_before);

            assert_eq!(
                capabilities.acquire_turbo(payload),
                TurboAcquisition::RejectedUnsupportedSelector
            );
            assert_eq!(capabilities, capabilities_before);
        }
    }

    #[test]
    fn weapon_inventory_native_payload_round_trips_slots_and_selection() {
        let mut inventory = WeaponInventory::new();
        // Selector 0x12 stores 500 resources but its master flags grant infinite ammo.
        inventory.acquire_weapon(selector_two(200));
        inventory.acquire_weapon(PowerUpPayload {
            selector: 0x12,
            amount: 500,
        });
        inventory.cycle(WeaponCycleDirection::Forward);

        let mut payload = [0u8; 0x248];
        inventory.encode_into_native_payload(&mut payload);

        let restored = WeaponInventory::restore_from_native_payload(&payload)
            .expect("restored inventory from native payload");

        assert_eq!(restored.occupied_slot_count(), 3);
        assert_eq!(restored.selected_slot(), inventory.selected_slot());
        assert_eq!(restored.slot(0).unwrap().selector(), 1);
        assert_eq!(restored.slot(1).unwrap().selector(), 2);
        assert_eq!(
            restored.slot(1).unwrap().ammunition(),
            Ammunition::Finite(200)
        );
        assert_eq!(restored.slot(2).unwrap().selector(), 0x12);
        assert_eq!(restored.slot(2).unwrap().ammunition(), Ammunition::Infinite);
        assert_eq!(restored.slot(2).unwrap().stored_resource_count(), 500);
        assert_eq!(restored, inventory);
    }
}
