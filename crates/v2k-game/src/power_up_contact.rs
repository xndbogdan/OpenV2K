//! Bounded player contact with authored type-61 Power Ups.
//!
//! This module closes only the proven bounded player/Power-Up path. It
//! does not claim to implement retail's general active-pair dispatcher:
//! unrelated behavior callbacks, component chains, physical response, and
//! damage still remain under that dispatcher's evidence gate. The bounded
//! path nevertheless uses the same authored Section-8 model/model collision,
//! signed 8.8 toroidal deltas, live-list order, inventory transaction, and
//! deferred destruction required by `FUN_00411AD0` / `FUN_00425AF0`.

use v2k_formats::models::{CollisionModelPool, ModelCollisionError};
use v2k_render::orientation_from_ypr;

use crate::entity::EntityManager;
use crate::entity_collision_state::RetailRuntimeValue;
use crate::player::{PlayerCraft, FUEL_FULL_RAW, FUEL_PICKUP_ACCEPT_BELOW_RAW};
use crate::player_hull::PlayerHull;
use crate::weapon_inventory::{
    PlayerCapabilities, PowerUpPayload, TargetterAcquisition, TurboAcquisition, WeaponAcquisition,
    WeaponAcquisitionRejection, WeaponInventory, TARGETTER_SELECTOR, TURBO_SELECTOR,
    WEAPON_SELECTOR_LIMIT,
};

const PLAYER_ENTITY_TYPE: u32 = 46;
const POWER_UP_ENTITY_TYPE: u32 = 61;
const POWER_UP_RECIPIENT_CAPABILITY: u32 = 1;
pub const TROPHY_SELECTOR: u8 = 0x3f;
pub const RETAIL_CONTROL_SLOT_COUNT: usize = 37;
pub const RETAIL_CONTROL_WORLD_SAVED_BIT: u32 = 0x01;
pub const RETAIL_CONTROL_HIDDEN_TROPHY_BIT: u32 = 0x02;
pub const RETAIL_CONTROL_TIME_TROPHY_BIT: u32 = 0x08;
/// `FUN_0042edc0` tests `(dword >> 9) & (1 << (column - 1))` for overlay-51
/// pair-link columns `1..=7`. `FUN_0042DD10` is the recovered per-exit writer.
pub const RETAIL_CONTROL_PAIR_LINK_SHIFT: u32 = 9;
pub const RETAIL_CONTROL_PAIR_LINK_COLUMNS: u32 = 7;
/// `FUN_0042ef20` writes `(1 << (subtype - 1)) << 4` for marker subtypes `1..=5`.
pub const RETAIL_CONTROL_EXIT_MARKER_SHIFT: u32 = 4;
pub const RETAIL_CONTROL_EXIT_MARKER_LIMIT: i32 = 5;
pub const TROPHY_RESTORED_HULL_RAW: i32 = 40_000;
pub const TROPHY_RESTORED_BUFFER_RAW: i32 = 100_000;
pub const TROPHY_EXTRA_LIFE_INTERVAL: u8 = 5;
pub const TROPHY_PICKUP_SOUND_ID: u16 = 0x32;
pub const TROPHY_EXTRA_LIFE_RATE_Q16: u32 = 0x2_0000;
pub const WEAPON_PICKUP_REJECTED_SOUND_ID: u16 = 2;
pub const TARGETTER_DUPLICATE_RATE_Q16: u32 = 0xAAAA;
pub const TURBO_DUPLICATE_RATE_Q16: u32 = 0xAAAA;
pub const FUEL_SELECTOR: u8 = 0x33;
pub const SHIELD_SELECTOR: u8 = 0x35;
pub const EXTRA_LIFE_SELECTOR: u8 = 0x36;
pub const HULL_REPAIR_SELECTOR: u8 = 0x37;
pub const CARGO_CAPACITY_SELECTOR: u8 = 0x3a;
pub const SHIELD_BUFFER_MAX_RAW: i32 = 100_000;
pub const HULL_REPAIR_ACCEPT_BELOW_RAW: i32 = 0x9471;
pub const HULL_REPAIR_MAX_RAW: i32 = 40_000;

/// Controller fields touched by `FUN_00445A90`'s trophy branch.
///
/// Retail has 37 control-state dwords at entity `+0xD8`. This owner retains the
/// three independently proven bits: campaign completion `0x01`, the hidden
/// selector-`0x3F` trophy `0x02`, and the time trophy `0x08`. The two counters
/// correspond to vehicle/controller `+0x194` and `+0x19A`; campaign
/// completion alone does not increment the pickup counter. A newly claimed
/// time trophy separately invokes player operation `0x13F`, which does.
/// The session owns a campaign control-slot id separately from the loaded
/// overlay id and installs that slot at controller `+0xC4` through
/// `FUN_0042E960`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerCampaignProgress {
    current_control_slot: Option<usize>,
    control_slot_state_bits: [u32; RETAIL_CONTROL_SLOT_COUNT],
    extra_lives: u8,
    trophy_count: u8,
}

impl PlayerCampaignProgress {
    /// Exact native 42ECC0/42E3B0 restoration. Keep every bit of all37
    /// dwords, including pair links and fields beyond the currently decoded
    /// trophy bits; loading does not replay acquisition events.
    pub fn from_native_snapshot(
        current_control_slot: usize,
        control_slot_state_bits: [u32; RETAIL_CONTROL_SLOT_COUNT],
        extra_lives: u8,
        trophy_count: u8,
    ) -> Self {
        Self {
            current_control_slot: (current_control_slot < RETAIL_CONTROL_SLOT_COUNT)
                .then_some(current_control_slot),
            control_slot_state_bits,
            extra_lives,
            trophy_count,
        }
    }

    pub const fn new() -> Self {
        Self {
            current_control_slot: None,
            control_slot_state_bits: [0; RETAIL_CONTROL_SLOT_COUNT],
            extra_lives: 0,
            trophy_count: 0,
        }
    }

    /// Select the current loaded world's retail control-state slot.
    /// `None` deliberately leaves zero-amount trophy handling fail-closed.
    pub fn set_current_control_slot(&mut self, slot: Option<usize>) {
        self.current_control_slot = slot.filter(|slot| *slot < RETAIL_CONTROL_SLOT_COUNT);
    }

    pub const fn current_control_slot(&self) -> Option<usize> {
        self.current_control_slot
    }

    pub const fn extra_lives(&self) -> u8 {
        self.extra_lives
    }

    pub const fn trophy_count(&self) -> u8 {
        self.trophy_count
    }

    /// Whether retail's hidden selector-`0x3F` trophy bit `0x02` is set for
    /// one control slot.
    pub fn control_slot_claimed(&self, slot: usize) -> Option<bool> {
        self.control_slot_has_bit(slot, RETAIL_CONTROL_HIDDEN_TROPHY_BIT)
    }

    /// Whether retail's campaign-completion bit `0x01` is set for one slot.
    pub fn control_slot_world_saved(&self, slot: usize) -> Option<bool> {
        self.control_slot_has_bit(slot, RETAIL_CONTROL_WORLD_SAVED_BIT)
    }

    /// Whether retail's independent time-trophy bit `0x08` is set for one
    /// control slot.
    pub fn control_slot_time_trophy_claimed(&self, slot: usize) -> Option<bool> {
        self.control_slot_has_bit(slot, RETAIL_CONTROL_TIME_TROPHY_BIT)
    }

    /// Raw `+0xD8` dword for one control slot. Overlay 51's `FUN_00454ff0`
    /// reads bits `0x01/0x02/0x04/0x08` from this word, then `FUN_0042edc0`
    /// reads bits `9..=15` as pair-link columns.
    pub fn control_slot_bits(&self, slot: usize) -> Option<u32> {
        self.control_slot_state_bits.get(slot).copied()
    }

    /// `FUN_0042edc0(controller, slot, column)` for overlay-51 neighbor `1..=7`.
    pub fn control_slot_pair_neighbor(&self, slot: usize, column: u32) -> bool {
        if !(1..=RETAIL_CONTROL_PAIR_LINK_COLUMNS).contains(&column) {
            return false;
        }
        let Some(bits) = self.control_slot_bits(slot) else {
            return false;
        };
        (bits >> RETAIL_CONTROL_PAIR_LINK_SHIFT) & (1 << (column - 1)) != 0
    }

    /// `FUN_0042DD10`'s `(1 << column_index) << 9` OR into `+0xD8`.
    pub fn set_pair_neighbor_bit(
        &mut self,
        slot: usize,
        column: u32,
    ) -> Result<bool, CampaignControlSlotError> {
        if !(1..=RETAIL_CONTROL_PAIR_LINK_COLUMNS).contains(&column) {
            return Ok(false);
        }
        let Some(state) = self.control_slot_state_bits.get_mut(slot) else {
            return Err(CampaignControlSlotError::ControlSlotUnresolved);
        };
        let mask = 1u32 << (RETAIL_CONTROL_PAIR_LINK_SHIFT + column - 1);
        let was_clear = *state & mask == 0;
        *state |= mask;
        Ok(was_clear)
    }

    /// `FUN_0042ef20` on the current control slot after a flag-`0x10` record matches.
    pub fn set_exit_marker_bit(&mut self, subtype: i32) -> Result<bool, CampaignControlSlotError> {
        if subtype < 1 || subtype > RETAIL_CONTROL_EXIT_MARKER_LIMIT {
            return Ok(false);
        }
        let Some(slot) = self.current_control_slot else {
            return Err(CampaignControlSlotError::ControlSlotUnresolved);
        };
        let state = &mut self.control_slot_state_bits[slot];
        let mask = (1u32 << (subtype as u32 - 1)) << RETAIL_CONTROL_EXIT_MARKER_SHIFT;
        let was_clear = *state & mask == 0;
        *state |= mask;
        Ok(was_clear)
    }

    pub(crate) fn current_control_state_bits(&self) -> Result<u32, CampaignControlSlotError> {
        self.current_control_slot
            .map(|slot| self.control_slot_state_bits[slot])
            .ok_or(CampaignControlSlotError::ControlSlotUnresolved)
    }

    pub(crate) fn mark_current_world_saved(&mut self) -> Result<bool, CampaignControlSlotError> {
        self.set_current_control_bit(RETAIL_CONTROL_WORLD_SAVED_BIT)
    }

    pub(crate) fn claim_current_time_trophy(&mut self) -> Result<bool, CampaignControlSlotError> {
        self.set_current_control_bit(RETAIL_CONTROL_TIME_TROPHY_BIT)
    }

    fn control_slot_has_bit(&self, slot: usize, bit: u32) -> Option<bool> {
        self.control_slot_state_bits
            .get(slot)
            .map(|state| state & bit != 0)
    }

    fn set_current_control_bit(&mut self, bit: u32) -> Result<bool, CampaignControlSlotError> {
        let Some(slot) = self.current_control_slot else {
            return Err(CampaignControlSlotError::ControlSlotUnresolved);
        };
        let state = &mut self.control_slot_state_bits[slot];
        let was_clear = *state & bit == 0;
        *state |= bit;
        Ok(was_clear)
    }

    fn acquire_trophy(
        &mut self,
        payload: PowerUpPayload,
        hull: &mut PlayerHull,
    ) -> Result<TrophyAcquisition, TrophyAcquisitionRejection> {
        let restore_hull = payload.amount == 0;
        let mut first_claim = false;

        if restore_hull {
            let Some(slot) = self.current_control_slot else {
                return Err(TrophyAcquisitionRejection::ControlSlotUnresolved);
            };

            // The executable writes both literal values before consulting the
            // per-level CLAIMED bit, so an already-claimed zero-amount trophy
            // still restores both fields before being consumed.
            hull.health_raw = TROPHY_RESTORED_HULL_RAW;
            hull.pre_health_damage_buffer_raw = TROPHY_RESTORED_BUFFER_RAW;

            if self.control_slot_state_bits[slot] & RETAIL_CONTROL_HIDDEN_TROPHY_BIT != 0 {
                return Ok(TrophyAcquisition {
                    restored_hull: true,
                    first_claim: false,
                    counted_trophy: false,
                    awarded_extra_life: false,
                    trophy_count: self.trophy_count,
                    extra_lives: self.extra_lives,
                });
            }
            self.control_slot_state_bits[slot] |= RETAIL_CONTROL_HIDDEN_TROPHY_BIT;
            first_claim = true;
        }

        Ok(self.count_trophy(restore_hull, first_claim))
    }

    /// `456820(1) -> 413600(player, 0x13F) -> 40DB20 -> 445A90`.
    /// This is selector 0x3F with amount 1: it counts a trophy without the
    /// amount-zero hidden-pickup hull/shield writes or hidden campaign bit.
    pub(crate) fn acquire_time_trophy_award(&mut self) -> TrophyAcquisition {
        self.count_trophy(false, false)
    }

    fn count_trophy(&mut self, restored_hull: bool, first_claim: bool) -> TrophyAcquisition {
        self.trophy_count = self.trophy_count.wrapping_add(1);
        let awarded_extra_life = self.trophy_count % TROPHY_EXTRA_LIFE_INTERVAL == 0;
        if awarded_extra_life {
            self.extra_lives = self.extra_lives.wrapping_add(1);
        }

        TrophyAcquisition {
            restored_hull,
            first_claim,
            counted_trophy: true,
            awarded_extra_life,
            trophy_count: self.trophy_count,
            extra_lives: self.extra_lives,
        }
    }

    fn acquire_extra_lives(&mut self, amount: i32) -> (u8, u8) {
        let before = self.extra_lives;
        // `FUN_00445A90` truncates the signed payload to `char` and performs
        // an 8-bit addition at controller +0x194.
        self.extra_lives = self.extra_lives.wrapping_add(amount as u8);
        (before, self.extra_lives)
    }
}

impl Default for PlayerCampaignProgress {
    fn default() -> Self {
        Self::new()
    }
}

/// Exact observable result of selector `0x3F`'s controller transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrophyAcquisition {
    pub restored_hull: bool,
    pub first_claim: bool,
    pub counted_trophy: bool,
    pub awarded_extra_life: bool,
    pub trophy_count: u8,
    pub extra_lives: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrophyAcquisitionRejection {
    ControlSlotUnresolved,
}

/// A controller transaction required a loaded campaign control slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CampaignControlSlotError {
    ControlSlotUnresolved,
}

/// Related mutable controller/entity state consumed by one bounded contact
/// pass. Keeping it as one explicit transaction context avoids an expanding
/// train of loosely ordered mutable arguments.
pub struct PlayerPowerUpRecipient<'a> {
    pub craft: &'a mut PlayerCraft,
    pub weapon_inventory: &'a mut WeaponInventory,
    pub capabilities: &'a mut PlayerCapabilities,
    pub hull: &'a mut PlayerHull,
    pub campaign_progress: &'a mut PlayerCampaignProgress,
}

/// Exact state mutation performed by one of `FUN_00445A90`'s proven
/// non-spawning controller branches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NonSpawningPowerUpAcquisition {
    Fuel {
        fuel_before_raw: i32,
        fuel_after_raw: i32,
    },
    Shield {
        buffer_before_raw: i32,
        buffer_after_raw: i32,
    },
    ExtraLife {
        extra_lives_before: u8,
        extra_lives_after: u8,
    },
    HullRepair {
        health_before_raw: i32,
        health_after_raw: i32,
    },
    CargoCapacity {
        capacity_before: usize,
        capacity_after: usize,
        unlock_raw_before: u8,
        unlock_raw_after: u8,
        /// Retail still consumes the pickup when the player's authored Sub-J
        /// attachment list is absent, but skips its text and resize call.
        cargo_list_available: bool,
    },
}

/// One accepted contact, in retail live-list order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcceptedPowerUpContact {
    Weapon {
        entity_id: u32,
        position_raw: [i16; 3],
        payload: PowerUpPayload,
        acquisition: WeaponAcquisition,
    },
    Targetter {
        entity_id: u32,
        position_raw: [i16; 3],
        payload: PowerUpPayload,
    },
    Turbo {
        entity_id: u32,
        position_raw: [i16; 3],
        payload: PowerUpPayload,
    },
    NonSpawning {
        entity_id: u32,
        position_raw: [i16; 3],
        payload: PowerUpPayload,
        acquisition: NonSpawningPowerUpAcquisition,
    },
    Trophy {
        entity_id: u32,
        position_raw: [i16; 3],
        payload: PowerUpPayload,
        acquisition: TrophyAcquisition,
    },
}

impl AcceptedPowerUpContact {
    pub const fn entity_id(self) -> u32 {
        match self {
            Self::Weapon { entity_id, .. }
            | Self::Targetter { entity_id, .. }
            | Self::Turbo { entity_id, .. }
            | Self::NonSpawning { entity_id, .. }
            | Self::Trophy { entity_id, .. } => entity_id,
        }
    }
}

/// Why an exact player/Power-Up overlap left the allocation alive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerUpContactRejection {
    UnsupportedSelector { selector: u8 },
    WeaponInventory(WeaponAcquisitionRejection),
    TargetterAlreadyAcquired,
    TurboAlreadyAcquired,
    FuelAtCapacity { fuel_raw: i32 },
    HullRepairAtCapacity { health_raw: i32 },
    Trophy(TrophyAcquisitionRejection),
}

/// One rejected exact contact, retained for presentation/debug policy without
/// conflating it with a geometric miss.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RejectedPowerUpContact {
    pub entity_id: u32,
    /// Retail resolves the positional pickup cue from this type-61 entity's
    /// runtime `+0x96`, not from the receiving player's location.
    pub position_raw: [i16; 3],
    pub payload: PowerUpPayload,
    pub reason: PowerUpContactRejection,
}

/// One exact overlap result in retail live-list order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerUpContactOutcome {
    Accepted(AcceptedPowerUpContact),
    Rejected(RejectedPowerUpContact),
}

/// Complete result of one atomic bounded scan.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlayerPowerUpContactPass {
    pub contacts: Vec<PowerUpContactOutcome>,
}

impl PlayerPowerUpContactPass {
    pub fn accepted(&self) -> impl Iterator<Item = &AcceptedPowerUpContact> {
        self.contacts.iter().filter_map(|contact| match contact {
            PowerUpContactOutcome::Accepted(accepted) => Some(accepted),
            PowerUpContactOutcome::Rejected(_) => None,
        })
    }

    pub fn rejected(&self) -> impl Iterator<Item = &RejectedPowerUpContact> {
        self.contacts.iter().filter_map(|contact| match contact {
            PowerUpContactOutcome::Accepted(_) => None,
            PowerUpContactOutcome::Rejected(rejected) => Some(rejected),
        })
    }
}

/// A prerequisite for an otherwise eligible candidate could not be resolved.
/// No inventory, capability, or deferred-destroy state is committed on error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerPowerUpContactError {
    PlayerMissing,
    PlayerWrongType {
        entity_type: u32,
    },
    PlayerActiveModelMissing,
    PlayerModelMissing {
        model_id: usize,
    },
    PlayerAttachmentRuntimeUnavailable,
    PowerUpPayloadMissing {
        entity_id: u32,
    },
    PowerUpActiveModelMissing {
        entity_id: u32,
    },
    PowerUpModelMissing {
        entity_id: u32,
        model_id: usize,
    },
    CollisionProgram {
        entity_id: u32,
        source: ModelCollisionError,
    },
}

/// The two cargo-capacity states that retail deliberately keeps separate.
/// Controller byte `+0x199` is compared and written without the Sub-J clamp;
/// `FUN_00418620` bounds only the live attachment-list capacity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StagedCargoUnlock {
    raw: u8,
    capacity: usize,
    capacity_max: usize,
}

impl EntityManager {
    /// Run the bounded player/type-61 pass at the caller's recovered pair-
    /// contact ordering point. Accepted allocations remain present until the
    /// next [`EntityManager::update`] or explicit deferred cleanup.
    pub fn resolve_player_power_up_contacts<P: CollisionModelPool + ?Sized>(
        &mut self,
        model_pool: &P,
        retail_tick: u32,
        recipient: PlayerPowerUpRecipient<'_>,
    ) -> Result<PlayerPowerUpContactPass, PlayerPowerUpContactError> {
        resolve_player_power_up_contacts(self, model_pool, retail_tick, recipient)
    }
}

/// Resolve exact player/type-61 contacts and stage accepted pickups.
///
/// All collision and handler work is evaluated against cloned player state.
/// Only after every candidate has resolved successfully are the inventory,
/// capability bits, and deferred-destroy ids committed. This avoids leaking an
/// earlier pickup when a later live-list candidate exposes an unsupported
/// collision program or missing runtime resource.
pub fn resolve_player_power_up_contacts<P: CollisionModelPool + ?Sized>(
    entities: &mut EntityManager,
    model_pool: &P,
    retail_tick: u32,
    recipient: PlayerPowerUpRecipient<'_>,
) -> Result<PlayerPowerUpContactPass, PlayerPowerUpContactError> {
    let PlayerPowerUpRecipient {
        craft: player_craft,
        weapon_inventory,
        capabilities,
        hull,
        campaign_progress,
    } = recipient;
    let player = entities
        .player()
        .ok_or(PlayerPowerUpContactError::PlayerMissing)?;
    if player.entity_type != PLAYER_ENTITY_TYPE {
        return Err(PlayerPowerUpContactError::PlayerWrongType {
            entity_type: player.entity_type,
        });
    }
    if player.capability_flags & POWER_UP_RECIPIENT_CAPABILITY == 0 {
        return Ok(PlayerPowerUpContactPass::default());
    }

    let player_model_id = player
        .model_index
        .or(player.model_slots[0])
        .ok_or(PlayerPowerUpContactError::PlayerActiveModelMissing)?;
    let player_model = model_pool.collision_model(player_model_id).ok_or(
        PlayerPowerUpContactError::PlayerModelMissing {
            model_id: player_model_id,
        },
    )?;
    let player_position_raw = player.position_raw();
    let player_to_world = player_craft.model_orientation(player.heading);
    let player_anim_vars = player_craft.anim_vars();

    let candidates = entities
        .iter_all()
        .filter(|candidate| {
            candidate.entity_type == POWER_UP_ENTITY_TYPE
                && candidate.active
                && candidate.attached_to.is_none()
                && !entities.is_power_up_destroy_pending(candidate.id)
        })
        .map(|candidate| {
            (
                candidate.id,
                candidate.position_raw(),
                candidate.heading,
                candidate.model_index.or(candidate.model_slots[0]),
                candidate.power_up_payload_packed,
                candidate.presentation_anim_vars(retail_tick),
            )
        })
        .collect::<Vec<_>>();

    let mut staged_inventory = weapon_inventory.clone();
    let mut staged_capabilities = *capabilities;
    let mut staged_craft = player_craft.clone();
    let mut staged_hull = *hull;
    let mut staged_campaign_progress = *campaign_progress;
    let (cargo_capacity, cargo_capacity_max, cargo_runtime_resolved) =
        match entities.player_cargo_capacity_state() {
            RetailRuntimeValue::Known((capacity, capacity_max)) => (capacity, capacity_max, true),
            RetailRuntimeValue::Unresolved => (0, 0, false),
        };
    let mut staged_cargo = StagedCargoUnlock {
        raw: entities.player_cargo_unlock_raw(),
        capacity: cargo_capacity,
        capacity_max: cargo_capacity_max,
    };
    let mut pass = PlayerPowerUpContactPass::default();

    for (entity_id, position_raw, heading, model_id, packed_payload, target_anim_vars) in candidates
    {
        let model_id =
            model_id.ok_or(PlayerPowerUpContactError::PowerUpActiveModelMissing { entity_id })?;
        let target_model = model_pool.collision_model(model_id).ok_or(
            PlayerPowerUpContactError::PowerUpModelMissing {
                entity_id,
                model_id,
            },
        )?;

        let target_to_world = orientation_from_ypr(std::f32::consts::FRAC_PI_2 - heading, 0.0, 0.0);
        let world_to_target = transpose_f64(target_to_world);
        let world_delta_raw = std::array::from_fn(|axis| {
            f64::from(player_position_raw[axis].wrapping_sub(position_raw[axis]))
        });
        let player_origin_in_target = transform_vector(world_to_target, world_delta_raw);
        let player_to_target = multiply_basis(world_to_target, player_to_world);

        let hit = player_model
            .collide_model_raw_oriented(
                target_model,
                player_origin_in_target,
                player_to_target,
                &player_anim_vars,
                &target_anim_vars,
                model_pool,
            )
            .map_err(|source| PlayerPowerUpContactError::CollisionProgram { entity_id, source })?;
        if hit.is_none() {
            continue;
        }

        let packed_payload =
            packed_payload.ok_or(PlayerPowerUpContactError::PowerUpPayloadMissing { entity_id })?;
        let payload = PowerUpPayload::from_runtime_word(packed_payload);
        if payload.selector == CARGO_CAPACITY_SELECTOR && !cargo_runtime_resolved {
            return Err(PlayerPowerUpContactError::PlayerAttachmentRuntimeUnavailable);
        }
        match payload.selector {
            selector if selector < WEAPON_SELECTOR_LIMIT => {
                let before = staged_inventory.clone();
                match staged_inventory.acquire_weapon(payload) {
                    acquisition @ WeaponAcquisition::Accepted { .. } => {
                        pass.contacts.push(PowerUpContactOutcome::Accepted(
                            AcceptedPowerUpContact::Weapon {
                                entity_id,
                                position_raw,
                                payload,
                                acquisition,
                            },
                        ));
                    }
                    WeaponAcquisition::Rejected(reason) => {
                        staged_inventory = before;
                        pass.contacts.push(PowerUpContactOutcome::Rejected(
                            RejectedPowerUpContact {
                                entity_id,
                                position_raw,
                                payload,
                                reason: PowerUpContactRejection::WeaponInventory(reason),
                            },
                        ));
                    }
                }
            }
            TARGETTER_SELECTOR => match staged_capabilities.acquire_targetter(payload) {
                TargetterAcquisition::Acquired => {
                    pass.contacts.push(PowerUpContactOutcome::Accepted(
                        AcceptedPowerUpContact::Targetter {
                            entity_id,
                            position_raw,
                            payload,
                        },
                    ));
                }
                TargetterAcquisition::AlreadyAcquired => {
                    pass.contacts
                        .push(PowerUpContactOutcome::Rejected(RejectedPowerUpContact {
                            entity_id,
                            position_raw,
                            payload,
                            reason: PowerUpContactRejection::TargetterAlreadyAcquired,
                        }));
                }
                TargetterAcquisition::RejectedUnsupportedSelector => {
                    unreachable!("selector was matched before dispatching the Targetter handler")
                }
            },
            TURBO_SELECTOR => match staged_capabilities.acquire_turbo(payload) {
                TurboAcquisition::Acquired => {
                    pass.contacts.push(PowerUpContactOutcome::Accepted(
                        AcceptedPowerUpContact::Turbo {
                            entity_id,
                            position_raw,
                            payload,
                        },
                    ));
                }
                TurboAcquisition::AlreadyAcquired => {
                    pass.contacts
                        .push(PowerUpContactOutcome::Rejected(RejectedPowerUpContact {
                            entity_id,
                            position_raw,
                            payload,
                            reason: PowerUpContactRejection::TurboAlreadyAcquired,
                        }));
                }
                TurboAcquisition::RejectedUnsupportedSelector => {
                    unreachable!("selector was matched before dispatching the Turbo handler")
                }
            },
            FUEL_SELECTOR
            | SHIELD_SELECTOR
            | EXTRA_LIFE_SELECTOR
            | HULL_REPAIR_SELECTOR
            | CARGO_CAPACITY_SELECTOR => match acquire_non_spawning_power_up(
                payload,
                &mut staged_craft,
                &mut staged_hull,
                &mut staged_campaign_progress,
                &mut staged_cargo,
            ) {
                Ok(acquisition) => {
                    pass.contacts.push(PowerUpContactOutcome::Accepted(
                        AcceptedPowerUpContact::NonSpawning {
                            entity_id,
                            position_raw,
                            payload,
                            acquisition,
                        },
                    ));
                }
                Err(reason) => {
                    pass.contacts
                        .push(PowerUpContactOutcome::Rejected(RejectedPowerUpContact {
                            entity_id,
                            position_raw,
                            payload,
                            reason,
                        }));
                }
            },
            TROPHY_SELECTOR => {
                let before_hull = staged_hull;
                let before_progress = staged_campaign_progress;
                match staged_campaign_progress.acquire_trophy(payload, &mut staged_hull) {
                    Ok(acquisition) => pass.contacts.push(PowerUpContactOutcome::Accepted(
                        AcceptedPowerUpContact::Trophy {
                            entity_id,
                            position_raw,
                            payload,
                            acquisition,
                        },
                    )),
                    Err(reason) => {
                        staged_hull = before_hull;
                        staged_campaign_progress = before_progress;
                        pass.contacts.push(PowerUpContactOutcome::Rejected(
                            RejectedPowerUpContact {
                                entity_id,
                                position_raw,
                                payload,
                                reason: PowerUpContactRejection::Trophy(reason),
                            },
                        ));
                    }
                }
            }
            selector => {
                pass.contacts
                    .push(PowerUpContactOutcome::Rejected(RejectedPowerUpContact {
                        entity_id,
                        position_raw,
                        payload,
                        reason: PowerUpContactRejection::UnsupportedSelector { selector },
                    }))
            }
        }
    }

    *weapon_inventory = staged_inventory;
    *capabilities = staged_capabilities;
    *player_craft = staged_craft;
    *hull = staged_hull;
    *campaign_progress = staged_campaign_progress;
    entities.raise_player_cargo_capacity_to(staged_cargo.capacity);
    entities.set_player_cargo_unlock_raw(staged_cargo.raw);
    let accepted_ids = pass
        .accepted()
        .copied()
        .map(AcceptedPowerUpContact::entity_id)
        .collect::<Vec<_>>();
    entities.queue_power_up_destroys(&accepted_ids);
    Ok(pass)
}

fn acquire_non_spawning_power_up(
    payload: PowerUpPayload,
    craft: &mut PlayerCraft,
    hull: &mut PlayerHull,
    campaign_progress: &mut PlayerCampaignProgress,
    cargo: &mut StagedCargoUnlock,
) -> Result<NonSpawningPowerUpAcquisition, PowerUpContactRejection> {
    let acquisition = match payload.selector {
        FUEL_SELECTOR => {
            let fuel_before_raw = craft.fuel_raw;
            if fuel_before_raw >= FUEL_PICKUP_ACCEPT_BELOW_RAW {
                return Err(PowerUpContactRejection::FuelAtCapacity {
                    fuel_raw: fuel_before_raw,
                });
            }
            // Retail uses wrapping signed addition and only an upper cap. Do
            // not import the static-pickup helper's non-negative normalization.
            craft.fuel_raw = fuel_before_raw.wrapping_add(payload.amount);
            if craft.fuel_raw > FUEL_FULL_RAW {
                craft.fuel_raw = FUEL_FULL_RAW;
            }
            NonSpawningPowerUpAcquisition::Fuel {
                fuel_before_raw,
                fuel_after_raw: craft.fuel_raw,
            }
        }
        SHIELD_SELECTOR => {
            let buffer_before_raw = hull.pre_health_damage_buffer_raw;
            hull.pre_health_damage_buffer_raw = buffer_before_raw
                .wrapping_add(payload.amount)
                .min(SHIELD_BUFFER_MAX_RAW);
            NonSpawningPowerUpAcquisition::Shield {
                buffer_before_raw,
                buffer_after_raw: hull.pre_health_damage_buffer_raw,
            }
        }
        EXTRA_LIFE_SELECTOR => {
            let (extra_lives_before, extra_lives_after) =
                campaign_progress.acquire_extra_lives(payload.amount);
            NonSpawningPowerUpAcquisition::ExtraLife {
                extra_lives_before,
                extra_lives_after,
            }
        }
        HULL_REPAIR_SELECTOR => {
            let health_before_raw = hull.health_raw;
            if health_before_raw >= HULL_REPAIR_ACCEPT_BELOW_RAW {
                return Err(PowerUpContactRejection::HullRepairAtCapacity {
                    health_raw: health_before_raw,
                });
            }
            hull.health_raw = health_before_raw
                .wrapping_add(payload.amount)
                .min(HULL_REPAIR_MAX_RAW);
            NonSpawningPowerUpAcquisition::HullRepair {
                health_before_raw,
                health_after_raw: hull.health_raw,
            }
        }
        CARGO_CAPACITY_SELECTOR => {
            let cargo_list_available = cargo.capacity_max != 0;
            let capacity_before = cargo.capacity;
            let unlock_raw_before = cargo.raw;
            if cargo_list_available && i32::from(cargo.raw) < payload.amount {
                // `FUN_00418620` receives the signed target while the runtime
                // list remains bounded by type 46's Sub-J maximum. The raw
                // controller byte independently truncates the authored dword.
                cargo.capacity = cargo
                    .capacity
                    .max((payload.amount as usize).min(cargo.capacity_max));
                cargo.raw = payload.amount as u8;
            }
            return Ok(NonSpawningPowerUpAcquisition::CargoCapacity {
                capacity_before,
                capacity_after: cargo.capacity,
                unlock_raw_before,
                unlock_raw_after: cargo.raw,
                cargo_list_available,
            });
        }
        selector => return Err(PowerUpContactRejection::UnsupportedSelector { selector }),
    };
    Ok(acquisition)
}

fn transpose_f64(matrix: [[f32; 3]; 3]) -> [[f64; 3]; 3] {
    std::array::from_fn(|row| std::array::from_fn(|column| f64::from(matrix[column][row])))
}

fn transform_vector(matrix: [[f64; 3]; 3], vector: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|row| {
        matrix[row][0] * vector[0] + matrix[row][1] * vector[1] + matrix[row][2] * vector[2]
    })
}

fn multiply_basis(left: [[f64; 3]; 3], right: [[f32; 3]; 3]) -> [[f64; 3]; 3] {
    std::array::from_fn(|row| {
        std::array::from_fn(|column| {
            (0..3)
                .map(|axis| left[row][axis] * f64::from(right[axis][column]))
                .sum()
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::entity_collision_state::EntityTypeRuntimeMetadata;
    use crate::session::GameSession;

    fn level_one_fixture() -> (GameSession, EntityManager) {
        let data_dir = v2k_test_support::retail_dir();
        let mut session = GameSession::init(&data_dir).expect("retail PRELOAD");
        session
            .load_auxiliary_ovl(3, 1)
            .expect("normal-tier system overlay");
        let type_models = session.cache.global_entity_model_table();
        let type_metadata = type_models
            .iter()
            .copied()
            .enumerate()
            .map(|(entity_type, model_slots)| {
                session
                    .cache
                    .global_entity_type(entity_type)
                    .map(EntityTypeRuntimeMetadata::from_section12)
                    .unwrap_or(EntityTypeRuntimeMetadata {
                        model_slots,
                        ..EntityTypeRuntimeMetadata::default()
                    })
            })
            .collect::<Vec<_>>();
        session
            .load_level_by_id(13, 1)
            .expect("normal-tier world 13");
        let entities = EntityManager::from_level_with_type_metadata(
            session.cache.level_desc().expect("world 13 descriptor"),
            &type_metadata,
            Some(session.cache.terrain().expect("world 13 terrain")),
        );
        (session, entities)
    }

    fn resolve_fixture_contacts(
        session: &GameSession,
        entities: &mut EntityManager,
        capabilities: &mut PlayerCapabilities,
    ) -> Result<PlayerPowerUpContactPass, PlayerPowerUpContactError> {
        let mut craft = PlayerCraft::new();
        let mut inventory = WeaponInventory::new();
        let mut hull = PlayerHull::default();
        let mut campaign_progress = PlayerCampaignProgress::default();
        resolve_player_power_up_contacts(
            entities,
            &session.cache,
            0,
            PlayerPowerUpRecipient {
                craft: &mut craft,
                weapon_inventory: &mut inventory,
                capabilities,
                hull: &mut hull,
                campaign_progress: &mut campaign_progress,
            },
        )
    }

    fn acquire_status(
        selector: u8,
        amount: i32,
        craft: &mut PlayerCraft,
        hull: &mut PlayerHull,
        progress: &mut PlayerCampaignProgress,
        cargo_capacity: usize,
        cargo_capacity_max: usize,
    ) -> Result<(NonSpawningPowerUpAcquisition, usize), PowerUpContactRejection> {
        let mut cargo = StagedCargoUnlock {
            raw: cargo_capacity as u8,
            capacity: cargo_capacity,
            capacity_max: cargo_capacity_max,
        };
        acquire_non_spawning_power_up(
            PowerUpPayload { selector, amount },
            craft,
            hull,
            progress,
            &mut cargo,
        )
        .map(|acquisition| (acquisition, cargo.capacity))
    }

    #[test]
    fn fuel_branch_uses_strict_gate_wrapping_add_and_upper_cap() {
        let mut craft = PlayerCraft::new();
        let mut hull = PlayerHull::default();
        let mut progress = PlayerCampaignProgress::new();
        craft.fuel_raw = 190_000;

        let accepted = acquire_status(
            FUEL_SELECTOR,
            25_000,
            &mut craft,
            &mut hull,
            &mut progress,
            1,
            5,
        );
        assert_eq!(
            accepted,
            Ok((
                NonSpawningPowerUpAcquisition::Fuel {
                    fuel_before_raw: 190_000,
                    fuel_after_raw: FUEL_FULL_RAW,
                },
                1,
            ))
        );

        craft.fuel_raw = FUEL_PICKUP_ACCEPT_BELOW_RAW;
        assert_eq!(
            acquire_status(FUEL_SELECTOR, 1, &mut craft, &mut hull, &mut progress, 1, 5,),
            Err(PowerUpContactRejection::FuelAtCapacity {
                fuel_raw: FUEL_PICKUP_ACCEPT_BELOW_RAW,
            })
        );
        assert_eq!(craft.fuel_raw, FUEL_PICKUP_ACCEPT_BELOW_RAW);
    }

    #[test]
    fn shield_and_repair_keep_their_distinct_caps_and_rejection_policy() {
        let mut craft = PlayerCraft::new();
        let mut hull = PlayerHull::default();
        let mut progress = PlayerCampaignProgress::new();
        hull.pre_health_damage_buffer_raw = 99_950;
        assert_eq!(
            acquire_status(
                SHIELD_SELECTOR,
                100,
                &mut craft,
                &mut hull,
                &mut progress,
                1,
                5,
            ),
            Ok((
                NonSpawningPowerUpAcquisition::Shield {
                    buffer_before_raw: 99_950,
                    buffer_after_raw: SHIELD_BUFFER_MAX_RAW,
                },
                1,
            ))
        );

        hull.health_raw = HULL_REPAIR_ACCEPT_BELOW_RAW - 1;
        assert_eq!(
            acquire_status(
                HULL_REPAIR_SELECTOR,
                20_000,
                &mut craft,
                &mut hull,
                &mut progress,
                1,
                5,
            ),
            Ok((
                NonSpawningPowerUpAcquisition::HullRepair {
                    health_before_raw: HULL_REPAIR_ACCEPT_BELOW_RAW - 1,
                    health_after_raw: HULL_REPAIR_MAX_RAW,
                },
                1,
            ))
        );
        hull.health_raw = HULL_REPAIR_ACCEPT_BELOW_RAW;
        assert_eq!(
            acquire_status(
                HULL_REPAIR_SELECTOR,
                20_000,
                &mut craft,
                &mut hull,
                &mut progress,
                1,
                5,
            ),
            Err(PowerUpContactRejection::HullRepairAtCapacity {
                health_raw: HULL_REPAIR_ACCEPT_BELOW_RAW,
            })
        );
    }

    #[test]
    fn extra_life_branch_truncates_amount_and_wraps_the_controller_byte() {
        let mut craft = PlayerCraft::new();
        let mut hull = PlayerHull::default();
        let mut progress = PlayerCampaignProgress::new();

        assert_eq!(
            acquire_status(
                EXTRA_LIFE_SELECTOR,
                -1,
                &mut craft,
                &mut hull,
                &mut progress,
                1,
                5,
            ),
            Ok((
                NonSpawningPowerUpAcquisition::ExtraLife {
                    extra_lives_before: 0,
                    extra_lives_after: u8::MAX,
                },
                1,
            ))
        );
        assert_eq!(progress.extra_lives(), u8::MAX);
    }

    #[test]
    fn cargo_capacity_is_monotonic_bounded_and_consumable_without_a_list() {
        let mut craft = PlayerCraft::new();
        let mut hull = PlayerHull::default();
        let mut progress = PlayerCampaignProgress::new();

        assert_eq!(
            acquire_status(
                CARGO_CAPACITY_SELECTOR,
                3,
                &mut craft,
                &mut hull,
                &mut progress,
                1,
                5,
            ),
            Ok((
                NonSpawningPowerUpAcquisition::CargoCapacity {
                    capacity_before: 1,
                    capacity_after: 3,
                    unlock_raw_before: 1,
                    unlock_raw_after: 3,
                    cargo_list_available: true,
                },
                3,
            ))
        );
        assert_eq!(
            acquire_status(
                CARGO_CAPACITY_SELECTOR,
                99,
                &mut craft,
                &mut hull,
                &mut progress,
                3,
                5,
            ),
            Ok((
                NonSpawningPowerUpAcquisition::CargoCapacity {
                    capacity_before: 3,
                    capacity_after: 5,
                    unlock_raw_before: 3,
                    unlock_raw_after: 99,
                    cargo_list_available: true,
                },
                5,
            ))
        );
        assert_eq!(
            acquire_status(
                CARGO_CAPACITY_SELECTOR,
                3,
                &mut craft,
                &mut hull,
                &mut progress,
                0,
                0,
            ),
            Ok((
                NonSpawningPowerUpAcquisition::CargoCapacity {
                    capacity_before: 0,
                    capacity_after: 0,
                    unlock_raw_before: 0,
                    unlock_raw_after: 0,
                    cargo_list_available: false,
                },
                0,
            ))
        );
    }

    #[test]
    fn cargo_comparison_uses_raw_unlock_byte_not_clamped_live_capacity() {
        let mut craft = PlayerCraft::new();
        let mut hull = PlayerHull::default();
        let mut progress = PlayerCampaignProgress::new();
        let mut cargo = StagedCargoUnlock {
            raw: 3,
            capacity: 3,
            capacity_max: 5,
        };

        let oversized = acquire_non_spawning_power_up(
            PowerUpPayload {
                selector: CARGO_CAPACITY_SELECTOR,
                amount: 99,
            },
            &mut craft,
            &mut hull,
            &mut progress,
            &mut cargo,
        )
        .unwrap();
        assert!(matches!(
            oversized,
            NonSpawningPowerUpAcquisition::CargoCapacity {
                capacity_after: 5,
                unlock_raw_after: 99,
                ..
            }
        ));

        let lower_authored_target = acquire_non_spawning_power_up(
            PowerUpPayload {
                selector: CARGO_CAPACITY_SELECTOR,
                amount: 10,
            },
            &mut craft,
            &mut hull,
            &mut progress,
            &mut cargo,
        )
        .unwrap();
        assert_eq!(cargo.capacity, 5);
        assert_eq!(cargo.raw, 99);
        assert!(matches!(
            lower_authored_target,
            NonSpawningPowerUpAcquisition::CargoCapacity {
                capacity_before: 5,
                capacity_after: 5,
                unlock_raw_before: 99,
                unlock_raw_after: 99,
                ..
            }
        ));
    }

    #[v2k_test_support::retail_test]
    fn turbo_contact_accepts_once_then_duplicate_stays_live() {
        let (session, mut entities) = level_one_fixture();
        let (pickup_id, pickup_position, pickup_position_raw) = entities
            .iter_all()
            .find(|entity| entity.power_up_payload_packed == Some(u32::from(TARGETTER_SELECTOR)))
            .map(|entity| (entity.id, entity.position, entity.position_raw()))
            .expect("authored Level-1 Targetter");
        entities
            .entity_mut_for_test(pickup_id)
            .expect("Targetter allocation")
            .power_up_payload_packed = Some(u32::from(TURBO_SELECTOR));
        entities.player_mut().expect("persistent player").position = pickup_position;
        let mut capabilities = PlayerCapabilities::default();

        let accepted =
            resolve_fixture_contacts(&session, &mut entities, &mut capabilities).unwrap();
        assert_eq!(
            accepted.accepted().copied().collect::<Vec<_>>(),
            vec![AcceptedPowerUpContact::Turbo {
                entity_id: pickup_id,
                position_raw: pickup_position_raw,
                payload: PowerUpPayload {
                    selector: TURBO_SELECTOR,
                    amount: 0,
                },
            }]
        );
        assert!(capabilities.has_turbo());
        assert_eq!(entities.pending_power_up_destroy_ids(), &[pickup_id]);

        let (session, mut entities) = level_one_fixture();
        let (pickup_id, pickup_position) = entities
            .iter_all()
            .find(|entity| entity.power_up_payload_packed == Some(u32::from(TARGETTER_SELECTOR)))
            .map(|entity| (entity.id, entity.position))
            .expect("authored Level-1 Targetter");
        entities
            .entity_mut_for_test(pickup_id)
            .expect("Targetter allocation")
            .power_up_payload_packed = Some(u32::from(TURBO_SELECTOR));
        entities.player_mut().expect("persistent player").position = pickup_position;
        let mut capabilities = PlayerCapabilities::default();
        assert_eq!(
            capabilities.acquire_turbo(PowerUpPayload {
                selector: TURBO_SELECTOR,
                amount: 0,
            }),
            TurboAcquisition::Acquired
        );

        let duplicate =
            resolve_fixture_contacts(&session, &mut entities, &mut capabilities).unwrap();
        assert!(duplicate.accepted().next().is_none());
        let rejected = duplicate.rejected().copied().collect::<Vec<_>>();
        assert_eq!(rejected.len(), 1);
        assert_eq!(rejected[0].entity_id, pickup_id);
        assert_eq!(
            rejected[0].reason,
            PowerUpContactRejection::TurboAlreadyAcquired
        );
        assert!(entities.pending_power_up_destroy_ids().is_empty());
        assert!(entities.iter_all().any(|entity| entity.id == pickup_id));
    }

    #[v2k_test_support::retail_test]
    fn later_candidate_error_rolls_back_turbo_and_deferred_destroy() {
        let (session, mut entities) = level_one_fixture();
        let candidate_ids = entities
            .iter_all()
            .filter(|entity| entity.entity_type == POWER_UP_ENTITY_TYPE)
            .map(|entity| entity.id)
            .take(2)
            .collect::<Vec<_>>();
        assert_eq!(candidate_ids.len(), 2);
        let contact_position = entities
            .iter_all()
            .find(|entity| entity.id == candidate_ids[0])
            .expect("first Power Up")
            .position;
        entities.player_mut().expect("persistent player").position = contact_position;
        let first = entities
            .entity_mut_for_test(candidate_ids[0])
            .expect("first Power Up");
        first.position = contact_position;
        first.power_up_payload_packed = Some(u32::from(TURBO_SELECTOR));
        let second = entities
            .entity_mut_for_test(candidate_ids[1])
            .expect("second Power Up");
        second.position = contact_position;
        second.power_up_payload_packed = None;
        let mut capabilities = PlayerCapabilities::default();

        assert_eq!(
            resolve_fixture_contacts(&session, &mut entities, &mut capabilities),
            Err(PlayerPowerUpContactError::PowerUpPayloadMissing {
                entity_id: candidate_ids[1],
            })
        );
        assert!(!capabilities.has_turbo());
        assert!(entities.pending_power_up_destroy_ids().is_empty());
        assert!(candidate_ids
            .iter()
            .all(|id| entities.iter_all().any(|entity| entity.id == *id)));
    }

    #[test]
    fn oriented_basis_conversion_uses_target_inverse() {
        let target_to_world = orientation_from_ypr(std::f32::consts::FRAC_PI_2, 0.0, 0.0);
        let world_to_target = transpose_f64(target_to_world);
        let in_target = transform_vector(world_to_target, [100.0, -20.0, 30.0]);

        assert!((in_target[0] + 30.0).abs() < 1.0e-5);
        assert!((in_target[1] + 20.0).abs() < 1.0e-5);
        assert!((in_target[2] - 100.0).abs() < 1.0e-5);
    }

    #[v2k_test_support::retail_test]
    fn authored_trophy_is_contact_consumed_without_becoming_cargo() {
        use crate::entity::{BeamCommand, BeamOutcome};
        use crate::native_type122::construction_tests::native_fixture_with_player;

        let (session, mut entities, _) = native_fixture_with_player(13);
        let trophy = entities
            .iter_all()
            .find(|entity| {
                entity.entity_type == POWER_UP_ENTITY_TYPE
                    && entity.power_up_payload_packed == Some(u32::from(TROPHY_SELECTOR))
            })
            .expect("Level 1 authors an amount-zero trophy");
        let trophy_id = trophy.id;
        assert_eq!(trophy.authored_spawn_index, Some(33));
        assert_eq!(trophy.model_slots, [Some(138); 4]);
        // 446C80 requires capability 0x1000. Trophy artwork is a model override
        // on Type61 (capability 0x40), not an independently beamable cargo type.
        assert_eq!(trophy.capability_flags, 0x40);
        let player_id = entities.player().unwrap().id;
        let ids = entities
            .iter_all()
            .map(|entity| entity.id)
            .collect::<Vec<_>>();
        for id in ids {
            entities.entity_mut_for_test(id).unwrap().position =
                if id == player_id || id == trophy_id {
                    [0.0, 16.0, 0.0]
                } else {
                    [128.0, 16.0, 128.0]
                };
        }
        let cargo_before = entities.player_cargo_hud_models();
        assert_eq!(cargo_before, [None]);
        entities.queue_beam(BeamCommand::Collect);
        assert_eq!(entities.tick_beam(81_000), Some(BeamOutcome::NoTarget));
        assert_eq!(entities.player_cargo_hud_models(), cargo_before);

        let mut craft = PlayerCraft::new();
        let mut inventory = WeaponInventory::new();
        let mut capabilities = PlayerCapabilities::default();
        let mut hull = PlayerHull::default();
        hull.health_raw = 1;
        hull.pre_health_damage_buffer_raw = 2;
        let mut progress = PlayerCampaignProgress::new();
        progress.set_current_control_slot(Some(1));
        let pass = resolve_player_power_up_contacts(
            &mut entities,
            &session.cache,
            0,
            PlayerPowerUpRecipient {
                craft: &mut craft,
                weapon_inventory: &mut inventory,
                capabilities: &mut capabilities,
                hull: &mut hull,
                campaign_progress: &mut progress,
            },
        )
        .unwrap();
        let accepted = pass.accepted().copied().collect::<Vec<_>>();
        assert!(matches!(
            accepted.as_slice(),
            [AcceptedPowerUpContact::Trophy { entity_id, acquisition, .. }]
                if *entity_id == trophy_id && acquisition.restored_hull
                    && acquisition.first_claim && acquisition.trophy_count == 1
        ));
        assert_eq!(hull.health_raw, TROPHY_RESTORED_HULL_RAW);
        assert_eq!(
            hull.pre_health_damage_buffer_raw,
            TROPHY_RESTORED_BUFFER_RAW
        );
        assert_eq!(entities.player_cargo_hud_models(), cargo_before);
        assert_eq!(entities.pending_power_up_destroy_ids(), &[trophy_id]);
        assert_eq!(entities.cleanup_pending_power_up_destroys(), [trophy_id]);
        assert!(entities.iter_all().all(|entity| entity.id != trophy_id));
        assert_eq!(entities.player_cargo_hud_models(), cargo_before);
    }

    #[test]
    fn claimed_zero_trophy_still_restores_hull_without_counting_twice() {
        let payload = PowerUpPayload {
            selector: TROPHY_SELECTOR,
            amount: 0,
        };
        let mut progress = PlayerCampaignProgress::new();
        progress.set_current_control_slot(Some(0));
        let mut hull = PlayerHull::default();

        let first = progress.acquire_trophy(payload, &mut hull).unwrap();
        assert!(first.first_claim);
        assert!(first.counted_trophy);
        hull.health_raw = 1;
        hull.pre_health_damage_buffer_raw = 2;

        let repeated = progress.acquire_trophy(payload, &mut hull).unwrap();
        assert_eq!(
            repeated,
            TrophyAcquisition {
                restored_hull: true,
                first_claim: false,
                counted_trophy: false,
                awarded_extra_life: false,
                trophy_count: 1,
                extra_lives: 0,
            }
        );
        assert_eq!(hull.health_raw, TROPHY_RESTORED_HULL_RAW);
        assert_eq!(
            hull.pre_health_damage_buffer_raw,
            TROPHY_RESTORED_BUFFER_RAW
        );
    }

    #[test]
    fn every_fifth_counted_trophy_awards_one_extra_life() {
        let payload = PowerUpPayload {
            selector: TROPHY_SELECTOR,
            amount: 1,
        };
        let mut progress = PlayerCampaignProgress::new();
        let mut hull = PlayerHull::default();

        for expected in 1..=4 {
            let acquisition = progress.acquire_trophy(payload, &mut hull).unwrap();
            assert_eq!(acquisition.trophy_count, expected);
            assert!(!acquisition.awarded_extra_life);
        }
        let fifth = progress.acquire_trophy(payload, &mut hull).unwrap();
        assert!(fifth.awarded_extra_life);
        assert_eq!(fifth.trophy_count, 5);
        assert_eq!(fifth.extra_lives, 1);
        assert!(!fifth.restored_hull);
    }

    #[test]
    fn wrapped_trophy_count_zero_still_awards_an_extra_life() {
        let mut progress = PlayerCampaignProgress::new();
        progress.trophy_count = u8::MAX;
        progress.extra_lives = 7;
        let mut hull = PlayerHull::default();

        let acquisition = progress
            .acquire_trophy(
                PowerUpPayload {
                    selector: TROPHY_SELECTOR,
                    amount: 1,
                },
                &mut hull,
            )
            .unwrap();

        assert_eq!(acquisition.trophy_count, 0);
        assert_eq!(acquisition.extra_lives, 8);
        assert!(acquisition.awarded_extra_life);
    }
}
