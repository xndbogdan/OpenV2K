//! V2000's cheats: the session's cheat word `+1C4`, its typed codes and the
//! in-play keys each cheat unlocks.
//!
//! Six cheats, one bit each in session `+1C4`, show the Cheats screen's rows
//! of the same index (`4D0F20 + id * 0x1C`): 0 Skip to World, 1 Weapons Menu,
//! 2 Repairs Menu, 3 Cargo Menu, 4 Complete this World, 5 Complete All
//! Worlds. `FUN_00450FC0` switches one on or off; `FUN_00455BD0` types the
//! eight-digit codes that switch them on.

use std::collections::HashSet;

use sdl2::keyboard::Keycode;

pub const SKIP_TO_WORLD: u32 = 0;
pub const WEAPONS_MENU: u32 = 1;
pub const REPAIRS_MENU: u32 = 2;
pub const CARGO_MENU: u32 = 3;
pub const COMPLETE_THIS_WORLD: u32 = 4;
pub const COMPLETE_ALL_WORLDS: u32 = 5;
/// `FUN_00450FC0`, `FUN_0044F830` and `FUN_004562B0` visit ids 0..5.
pub const CHEAT_COUNT: u32 = 6;

/// The code table at `004D1328`: eight ASCII digits and the cheat id, in
/// table order. The last code switches every cheat on, in this order.
pub const CHEAT_CODES: [(&[u8; 8], u32); 12] = [
    (b"66389545", WEAPONS_MENU),
    (b"52367323", SKIP_TO_WORLD),
    (b"11647239", REPAIRS_MENU),
    (b"65437321", CARGO_MENU),
    (b"62367832", COMPLETE_THIS_WORLD),
    (b"96238275", COMPLETE_ALL_WORLDS),
    (b"16245729", WEAPONS_MENU),
    (b"16245729", SKIP_TO_WORLD),
    (b"16245729", REPAIRS_MENU),
    (b"16245729", CARGO_MENU),
    (b"16245729", COMPLETE_THIS_WORLD),
    (b"16245729", COMPLETE_ALL_WORLDS),
];

/// What a Cheats screen row asks of the session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheatMenuAction {
    /// `FUN_00455F50(code)`: with the Cargo Menu cheat on, menu sound 3 and,
    /// on a living player, operation `code` (`FUN_00413600`).
    Grant(u32),
    /// `FUN_00455FF0`: the world's native into the hold (`FUN_0042EBE0`).
    AddNative,
    /// `FUN_00456020`: the world's scientist into the hold (`FUN_0042EB70`).
    AddScientist,
    /// `FUN_00456050` -> `FUN_00455E20`: every Weapons row.
    AllWeapons,
    /// `FUN_004562A0` -> `FUN_004170F0`.
    CompleteThisWorld,
    /// `FUN_00456300`: every campaign control word gains `0xF` and
    /// `0xFFFFFFF0` (`FUN_0042EE30`, `FUN_0042EE50`).
    CompleteAllWorlds,
    /// `FUN_004562B0`: all six cheats off, then one screen pops.
    DisableAll,
    /// `FUN_00450D70`'s select: menu sound 3, session `+28C`, the menu
    /// stack cleared (`FUN_0043C7D0`) and play resumed (`FUN_004512E0`);
    /// the gameplay frame then leaves for this logical world.
    SkipToWorld(u32),
}

/// `FUN_00450FC0` on the cheat word: a network game (`004F741C`) only ever
/// switches a cheat off. Returns whether the switch-on sound (`0x57`
/// through `FUN_0042A860`) plays. The Cheats rows follow the word
/// ([`crate::menu_data::MenuTree::apply_cheat_enables`]).
pub fn set_cheat(word: &mut u32, id: u32, on: bool, network: bool) -> bool {
    let bit = 1 << (id & 0x1f);
    if on && !network {
        *word |= bit;
        true
    } else {
        *word &= !bit;
        false
    }
}

/// `FUN_0044F830`, from a native save's load (`FUN_0044F650` mode 6): each
/// of the six bits goes back through `FUN_00450FC0`, so each one on plays
/// its sound again. Returns the ids switched on, in order.
pub fn reapply_cheats(word: &mut u32, network: bool) -> Vec<u32> {
    let mut sounds = Vec::new();
    for id in 0..CHEAT_COUNT {
        let on = *word & (1 << id) != 0;
        if set_cheat(word, id, on, network) {
            sounds.push(id);
        }
    }
    sounds
}

/// Session `+280`: the digits typed so far, at most eight and NUL-terminated.
/// `FUN_0044F8B0` clears it on every world entry.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CheatCodeEntry {
    typed: [u8; 9],
}

impl CheatCodeEntry {
    fn len(&self) -> usize {
        self.typed.iter().take(8).position(|&b| b == 0).unwrap_or(8)
    }

    /// The digits typed so far.
    pub fn typed(&self) -> &[u8] {
        &self.typed[..self.len()]
    }

    /// `FUN_00455BD0`: append the digit, then compare the buffer with every
    /// code in table order. A partial match keeps the buffer; a whole
    /// eight-digit match switches that code's cheat on and keeps comparing.
    /// Without a partial match the buffer restarts from the digit just
    /// typed. Returns the cheat ids to switch on, in table order.
    pub fn type_digit(&mut self, digit: u8) -> Vec<u32> {
        let n = self.len();
        if n < 8 {
            self.typed[n] = b'0' + digit;
            self.typed[n + 1] = 0;
        }
        let mut matched = Vec::new();
        let mut partial = false;
        for (code, id) in CHEAT_CODES {
            if strncmp_eq(code, &self.typed, n + 1) {
                if n != 7 {
                    partial = true;
                    break;
                }
                matched.push(id);
            }
        }
        if !partial {
            self.typed[0] = self.typed[n];
            self.typed[1] = 0;
        }
        matched
    }
}

/// `_strncmp(code, typed, count) == 0`, stopping at the first NUL as the CRT
/// does. The codes hold no NUL in their eight digits.
fn strncmp_eq(code: &[u8; 8], typed: &[u8; 9], count: usize) -> bool {
    for i in 0..count {
        let a = code.get(i).copied().unwrap_or(0);
        let b = typed[i];
        if a != b {
            return false;
        }
        if a == 0 {
            return true;
        }
    }
    true
}

/// One in-play key record (`0x004C25E0`) whose callback a cheat gates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InPlayCheatKey {
    /// `FUN_00455AB0`..`FUN_00455BB0`: LSHIFT with C E R A T H S O Q types
    /// digits 1 to 9 (`FUN_00455BD0`).
    Digit(u8),
    /// `FUN_00455DB0`, LSHIFT+D, cheat 0: the session's body `+2A8` takes
    /// [`SELF_DAMAGE_PACKET`] through checked damage (`FUN_00415040`).
    SelfDamage,
    /// `FUN_00455E20`, F9, cheat 1: every Cheats -> Weapons row.
    AllWeapons,
    /// `FUN_00456100`, F8, cheat 2: operation `0x35` with 100000.
    Shields,
    /// `FUN_00456070`, F11, cheat 2: operation `0x37` with 40000 less health.
    HullRepair,
    /// `FUN_00455EB0`, F12, cheat 2: operation `0x33` with 200000 less fuel.
    Fuel,
}

impl InPlayCheatKey {
    /// The cheat bit the callback tests; typing digits needs none.
    pub const fn cheat(self) -> Option<u32> {
        match self {
            Self::Digit(_) => None,
            Self::SelfDamage => Some(SKIP_TO_WORLD),
            Self::AllWeapons => Some(WEAPONS_MENU),
            Self::Shields | Self::HullRepair | Self::Fuel => Some(REPAIRS_MENU),
        }
    }

    /// Whether the cheat word lets the callback act.
    pub const fn enabled(self, word: u32) -> bool {
        match self.cheat() {
            Some(id) => word & (1 << id) != 0,
            None => true,
        }
    }
}

/// The in-play set's records for these callbacks, in table order: each needs
/// all its keys down and has no key that must be up.
const IN_PLAY_CHEAT_RECORDS: [(&[Keycode], InPlayCheatKey); 14] = [
    (&[Keycode::LShift, Keycode::D], InPlayCheatKey::SelfDamage),
    (&[Keycode::F9], InPlayCheatKey::AllWeapons),
    (&[Keycode::F8], InPlayCheatKey::Shields),
    (&[Keycode::F11], InPlayCheatKey::HullRepair),
    (&[Keycode::F12], InPlayCheatKey::Fuel),
    (&[Keycode::C, Keycode::LShift], InPlayCheatKey::Digit(1)),
    (&[Keycode::E, Keycode::LShift], InPlayCheatKey::Digit(2)),
    (&[Keycode::R, Keycode::LShift], InPlayCheatKey::Digit(3)),
    (&[Keycode::A, Keycode::LShift], InPlayCheatKey::Digit(4)),
    (&[Keycode::T, Keycode::LShift], InPlayCheatKey::Digit(5)),
    (&[Keycode::H, Keycode::LShift], InPlayCheatKey::Digit(6)),
    (&[Keycode::S, Keycode::LShift], InPlayCheatKey::Digit(7)),
    (&[Keycode::O, Keycode::LShift], InPlayCheatKey::Digit(8)),
    (&[Keycode::Q, Keycode::LShift], InPlayCheatKey::Digit(9)),
];

/// The records that became active between two key states: FGDK runs a press
/// callback when its record's keys become all held, in whatever order.
pub fn newly_active_in_play_keys(
    before: &HashSet<Keycode>,
    after: &HashSet<Keycode>,
) -> Vec<InPlayCheatKey> {
    IN_PLAY_CHEAT_RECORDS
        .iter()
        .filter(|(keys, _)| {
            keys.iter().all(|key| after.contains(key))
                && !keys.iter().all(|key| before.contains(key))
        })
        .map(|&(_, action)| action)
        .collect()
}

/// `FUN_00455DB0`'s packet: channels 2 and 3, 3000 each.
pub const SELF_DAMAGE_PACKET: crate::damage::DamagePacket = crate::damage::DamagePacket {
    channels: [2, 3],
    amounts_raw: [3000, 3000],
};

/// `FUN_00455EB0`: fuel's operation tops it up to 200000 (`0x30D4033 - fuel * 0x100`).
pub const fn fuel_top_up_code(fuel_raw: i32) -> u32 {
    0x30D_4033_u32.wrapping_sub((fuel_raw as u32).wrapping_mul(0x100))
}

/// `FUN_00456070`: repairs top health up to 40000 (`0x9C4037 - health * 0x100`).
pub const fn hull_repair_code(health_raw: i32) -> u32 {
    0x9C_4037_u32.wrapping_sub((health_raw as u32).wrapping_mul(0x100))
}

/// `FUN_00456100`: shields, 100000.
pub const SHIELDS_CODE: u32 = 0x186_A035;

/// `FUN_00455FD0`'s row, Larger Cargo Capacity: operation `0x3A` with 5.
pub const LARGER_CARGO_CODE: u32 = 0x53A;

/// `FUN_00455FF0` and `FUN_00456020`: operation `0x38` adds a body of the
/// world's native or scientist type to the hold.
pub const fn add_to_hold_code(entity_type: u32) -> u32 {
    (entity_type << 8) + 0x38
}

/// `FUN_0042EBE0`: the native a world's theme (Section 13 `+0x48`) carries
/// as cargo; 0 for the themes without one.
pub const fn native_cargo_type(theme: u32) -> u32 {
    match theme {
        1 => 9,
        2 => 0x5f,
        3 => 0x56,
        4 => 0x4e,
        _ => 0,
    }
}

/// `FUN_0042EB70`: the scientist a world's theme carries as cargo.
pub const fn scientist_cargo_type(theme: u32) -> u32 {
    match theme {
        1 => 8,
        2 => 0x5b,
        3 => 0x5a,
        4 => 0x4f,
        5 => 0x74,
        6 => 7,
        _ => 0,
    }
}

/// `FUN_00450D70`'s range: the row's argument 36, the last logical world.
pub const SKIP_WORLD_MAX: u32 = 36;

/// The logical worlds the selector passes over: the six arenas.
pub const fn skip_world_passes_over(world: u32) -> bool {
    matches!(world, 13 | 15 | 16 | 17 | 29 | 32)
}

/// `FUN_00450D70` on Left (`right == false`) or Right: the next logical
/// world past the arenas, or `None` at a limit (menu sound 0; a step plays
/// menu sound 1).
pub fn step_skip_world(world: u32, right: bool, max: u32) -> Option<u32> {
    let can_step = if right {
        (world as i32) < max as i32
    } else {
        (world as i32) > 1
    };
    if !can_step {
        return None;
    }
    let mut next = world;
    loop {
        next = if right {
            next.wrapping_add(1)
        } else {
            next.wrapping_sub(1)
        };
        if !skip_world_passes_over(next) {
            return Some(next);
        }
    }
}

/// `DAT_004D04E0`, the selector's world: process-global, 1 at startup, and
/// kept across sessions and pause menus.
static SKIP_WORLD: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);

pub fn skip_world() -> u32 {
    SKIP_WORLD.load(std::sync::atomic::Ordering::Relaxed)
}

pub fn set_skip_world(world: u32) {
    SKIP_WORLD.store(world, std::sync::atomic::Ordering::Relaxed);
}

/// A player operation the cheats reach whose `FUN_00445A90` branch has no
/// port owner. Each stops its cheat with this logged reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeldCheatOperation {
    /// Operation `0x39`: controller `+195` gains the count (at most 4) and a
    /// Type65 is born at the player.
    Megablast { count: i32 },
}

/// Whether `code`'s operation is one the port holds.
pub fn held_operation(code: u32) -> Option<HeldCheatOperation> {
    let payload = crate::weapon_inventory::PowerUpPayload::from_runtime_word(code);
    match payload.selector {
        0x39 => Some(HeldCheatOperation::Megablast {
            count: payload.amount,
        }),
        _ => None,
    }
}

/// The type operation `0x38` births into the hold, when `code` is one
/// (`EntityManager::dispatch_controller_operation_0x38`).
pub fn add_to_hold_type(code: u32) -> Option<u32> {
    let payload = crate::weapon_inventory::PowerUpPayload::from_runtime_word(code);
    (payload.selector == 0x38).then_some(payload.amount as u32)
}

/// `FUN_00455E20`'s codes: every Cheats -> Weapons row whose callback is
/// `FUN_00455F50`, in row order (`004C4884`, 0x1C apart, to `004C4B5C`).
pub fn all_weapons_codes() -> Vec<u32> {
    crate::menu_data::MenuTree::load()
        .screen(crate::menu_data::CHEAT_WEAPONS)
        .map(|screen| {
            screen
                .items
                .iter()
                .filter_map(|item| match item.select {
                    crate::menu_data::SelectAction::Cheat { code } => Some(code),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

/// `FUN_004170F0`'s victims: capability 8 with state bit `0x01000000` and
/// without `0x10000000`.
const COMPLETE_WORLD_STATE_BIT: u32 = 0x0100_0000;
const COMPLETE_WORLD_EXCLUDED_BIT: u32 = 0x1000_0000;

/// The world a Cheats-row death sweep runs in, between frames: the scheduler
/// holds every owner.
pub struct CheatWorldFrame<'a> {
    pub entities: &'a mut crate::entity::EntityManager,
    pub resources: &'a mut crate::resource_cache::ResourceCache,
    pub static_damage: &'a mut crate::static_damage::StaticDamageScheduler,
    pub player_hull: &'a mut crate::player_hull::PlayerHull,
    pub world_fx: &'a mut crate::world_fx::WorldFx,
    pub scheduler: &'a mut crate::specialized_actor_task_production::SpecializedActorTaskScheduler,
    pub notifications: &'a mut crate::gameplay_notifications::GameplayNotifications,
    pub retail_tick: u32,
    pub extra_lives: u8,
}

/// Why Complete this World stopped part way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompleteWorldBlock {
    /// A body's capability or state word is unresolved.
    StateUnresolved { entity_id: u32 },
    /// A death removed the body before its successor was read.
    LiveListLost { entity_id: u32 },
    /// A victim's `FUN_00410C10` owner stopped.
    Death {
        entity_id: u32,
        block: crate::main_base_abort_production::GenericDeathBlock,
    },
}

/// `FUN_004170F0`, Complete this World's row (`FUN_004562A0`): `FUN_00410C10`
/// on each body in the live list with capability 8 and state bit
/// `0x01000000` but not `0x10000000`, reading each successor after the
/// death. Returns whether a death changed the terrain.
pub fn complete_this_world(frame: CheatWorldFrame<'_>) -> Result<bool, CompleteWorldBlock> {
    use crate::entity_collision_state::RetailRuntimeValue;
    let CheatWorldFrame {
        entities,
        resources,
        static_damage,
        player_hull,
        world_fx,
        scheduler,
        notifications,
        retail_tick,
        extra_lives,
    } = frame;
    let mut terrain_changed = false;
    let mut current = entities.retail_live_order_ids().next();
    while let Some(id) = current {
        let entity = entities
            .iter_all()
            .find(|entity| entity.id == id)
            .ok_or(CompleteWorldBlock::LiveListLost { entity_id: id })?;
        let victim = entity.capability_flags & 8 != 0
            && match entity
                .collision
                .state_flags_at_0x08
                .masked(COMPLETE_WORLD_STATE_BIT | COMPLETE_WORLD_EXCLUDED_BIT)
            {
                RetailRuntimeValue::Known(bits) => bits == COMPLETE_WORLD_STATE_BIT,
                RetailRuntimeValue::Unresolved => {
                    return Err(CompleteWorldBlock::StateUnresolved { entity_id: id })
                }
            };
        if victim {
            terrain_changed |= crate::main_base_abort_production::run_generic_death(
                id,
                crate::main_base_abort_production::GenericDeathFrame {
                    entities: &mut *entities,
                    resources: &mut *resources,
                    static_damage: &mut *static_damage,
                    player_hull: &mut *player_hull,
                    world_fx: &mut *world_fx,
                    scheduler: &mut *scheduler,
                    notifications: &mut *notifications,
                    retail_tick,
                    extra_lives: RetailRuntimeValue::Known(extra_lives),
                },
            )
            .map_err(|block| CompleteWorldBlock::Death {
                entity_id: id,
                block,
            })?;
        }
        current = entities
            .retail_live_successor_id(id)
            .map_err(|()| CompleteWorldBlock::LiveListLost { entity_id: id })?;
    }
    Ok(terrain_changed)
}

/// `FUN_00455DB0` on the player: [`SELF_DAMAGE_PACKET`] through
/// `FUN_00415040` with no ratio.
pub fn self_damage(
    frame: crate::attached_particle_damage::AttachedParticleDamageFrame<'_>,
    target_id: u32,
) -> Result<i32, crate::attached_particle_damage::AttachedParticleDamageBlock> {
    crate::attached_particle_damage::apply_scaled_checked_damage(
        frame,
        target_id,
        crate::damage::DamageDeliveryRecord {
            packet: SELF_DAMAGE_PACKET,
            ..Default::default()
        },
        0,
        0,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn type_code(entry: &mut CheatCodeEntry, code: &[u8]) -> Vec<u32> {
        code.iter()
            .flat_map(|digit| entry.type_digit(digit - b'0'))
            .collect()
    }

    #[test]
    fn the_master_code_switches_every_cheat_on_in_table_order() {
        let mut entry = CheatCodeEntry::default();
        assert_eq!(type_code(&mut entry, b"16245729"), vec![1, 0, 2, 3, 4, 5]);
        // The buffer restarts from the last digit, which begins 96238275.
        assert_eq!(entry.typed(), b"9");
        assert_eq!(type_code(&mut entry, b"6238275"), vec![COMPLETE_ALL_WORLDS]);
    }

    #[test]
    fn each_single_code_switches_its_cheat_on() {
        for (code, id) in &CHEAT_CODES[..6] {
            let mut entry = CheatCodeEntry::default();
            assert_eq!(type_code(&mut entry, *code), vec![*id]);
        }
    }

    #[test]
    fn a_wrong_digit_restarts_from_that_digit() {
        let mut entry = CheatCodeEntry::default();
        assert!(type_code(&mut entry, b"1624").is_empty());
        assert_eq!(entry.typed(), b"1624");
        // 3 continues no code, so the buffer becomes "3", itself no prefix.
        assert!(entry.type_digit(3).is_empty());
        assert_eq!(entry.typed(), b"3");
        // A digit that starts a code is kept as its first digit.
        assert!(type_code(&mut entry, b"5").is_empty());
        assert_eq!(entry.typed(), b"5");
        assert_eq!(type_code(&mut entry, b"2367323"), vec![SKIP_TO_WORLD]);
    }

    #[test]
    fn network_play_only_switches_cheats_off() {
        let mut word = 0;
        assert!(!set_cheat(&mut word, CARGO_MENU, true, true));
        assert_eq!(word, 0);
        assert!(set_cheat(&mut word, CARGO_MENU, true, false));
        assert_eq!(word, 8);
        assert!(!set_cheat(&mut word, CARGO_MENU, false, false));
        assert_eq!(word, 0);
        let mut word = 0x8000_0005;
        assert_eq!(reapply_cheats(&mut word, false), vec![0, 2]);
        assert_eq!(word, 0x8000_0005);
    }

    #[test]
    fn the_world_selector_passes_over_the_arenas_and_stops_at_its_limits() {
        assert_eq!(step_skip_world(1, false, SKIP_WORLD_MAX), None);
        assert_eq!(step_skip_world(1, true, SKIP_WORLD_MAX), Some(2));
        assert_eq!(step_skip_world(12, true, SKIP_WORLD_MAX), Some(14));
        assert_eq!(step_skip_world(14, true, SKIP_WORLD_MAX), Some(18));
        assert_eq!(step_skip_world(18, false, SKIP_WORLD_MAX), Some(14));
        assert_eq!(step_skip_world(28, true, SKIP_WORLD_MAX), Some(30));
        assert_eq!(step_skip_world(31, true, SKIP_WORLD_MAX), Some(33));
        assert_eq!(step_skip_world(36, true, SKIP_WORLD_MAX), None);
        let mut world = 1;
        let mut reached = vec![world];
        while let Some(next) = step_skip_world(world, true, SKIP_WORLD_MAX) {
            world = next;
            reached.push(world);
        }
        // The lab's skip_world.py list: global levels 13-24, 26, 30-40, 42,
        // 43 and 45-48.
        let levels: Vec<u32> = reached.iter().map(|w| w + 12).collect();
        let expected: Vec<u32> = (13..=24)
            .chain([26])
            .chain(30..=40)
            .chain([42, 43])
            .chain(45..=48)
            .collect();
        assert_eq!(levels, expected);
    }

    #[test]
    fn records_act_when_their_keys_become_held_in_any_order() {
        let set = |keys: &[Keycode]| keys.iter().copied().collect::<HashSet<_>>();
        assert_eq!(
            newly_active_in_play_keys(
                &set(&[Keycode::LShift]),
                &set(&[Keycode::LShift, Keycode::C])
            ),
            vec![InPlayCheatKey::Digit(1)]
        );
        assert_eq!(
            newly_active_in_play_keys(&set(&[Keycode::Q]), &set(&[Keycode::Q, Keycode::LShift])),
            vec![InPlayCheatKey::Digit(9)]
        );
        assert!(newly_active_in_play_keys(&set(&[]), &set(&[Keycode::C])).is_empty());
        assert!(newly_active_in_play_keys(
            &set(&[Keycode::RShift]),
            &set(&[Keycode::RShift, Keycode::C])
        )
        .is_empty());
        assert_eq!(
            newly_active_in_play_keys(&set(&[]), &set(&[Keycode::F9])),
            vec![InPlayCheatKey::AllWeapons]
        );
        assert!(!InPlayCheatKey::AllWeapons.enabled(1));
        assert!(InPlayCheatKey::AllWeapons.enabled(2));
        assert!(InPlayCheatKey::Fuel.enabled(4));
        assert!(InPlayCheatKey::SelfDamage.enabled(1));
    }

    #[test]
    fn top_up_codes_match_the_callbacks_constants() {
        assert_eq!(fuel_top_up_code(0), 0x30D_4033);
        let code =
            crate::weapon_inventory::PowerUpPayload::from_runtime_word(fuel_top_up_code(150_000));
        assert_eq!((code.selector, code.amount), (0x33, 50_000));
        let code =
            crate::weapon_inventory::PowerUpPayload::from_runtime_word(hull_repair_code(30_000));
        assert_eq!((code.selector, code.amount), (0x37, 10_000));
        assert_eq!(add_to_hold_code(native_cargo_type(2)), 0x5F38);
        assert_eq!(add_to_hold_code(scientist_cargo_type(6)), 0x738);
    }

    #[test]
    fn all_weapons_gives_the_twenty_five_weapon_rows_in_order() {
        let codes = all_weapons_codes();
        assert_eq!(codes.len(), 25);
        assert_eq!(codes[0], 0x3E712);
        assert_eq!(codes[24], 0x3E71F);
        // Megablast is the held operation; Autobot and Defender Ball go to
        // the hold.
        let held: Vec<_> = codes
            .iter()
            .filter_map(|&code| held_operation(code))
            .collect();
        assert_eq!(held, vec![HeldCheatOperation::Megablast { count: 5 }]);
        let hold: Vec<_> = codes
            .iter()
            .filter_map(|&code| add_to_hold_type(code))
            .collect();
        assert_eq!(hold, vec![64, 63]);
    }
}
