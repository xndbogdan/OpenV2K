//! Starting a network game ([MULTIPLAYER.md](../../../../docs/re/MULTIPLAYER.md),
//! "Starting a game"): the arena, the line files and where each player
//! enters.

use std::path::Path;

use super::scoring::{LineTable, LineTables};

/// `0x4D0D88`: the six arenas' logical world numbers (global overlays 25,
/// 27, 28, 29, 41 and 44).
pub const ARENAS: [u16; 6] = [13, 15, 16, 17, 29, 32];
/// Logical world number to global overlay id.
pub const OVERLAY_OFFSET: u16 = 12;

/// `FUN_0044F650` mode 7 on the host: the arena the frontend's keys 1..=6
/// forced (`DAT_004DB228`), else one of the first five at random.
pub fn host_world(forced: Option<u8>, random: u32) -> u16 {
    match forced {
        Some(arena @ 1..=6) => ARENAS[usize::from(arena) - 1],
        _ => ARENAS[(random & 0xFFFF) as usize % 5],
    }
}

/// `FUN_00439290` -> `FUN_004393C0`: the five line files from the game
/// folder, each falling back to its built-in text (strings 301, 300 and 302
/// for kill, death and self-kill lines; empty for names and host lines).
pub fn load_line_tables(game_folder: &Path, built_in: impl Fn(usize) -> String) -> LineTables {
    let read = |name: &str| std::fs::read(game_folder.join(name)).ok();
    let table = |name: &str, limit: usize, fallback: String| {
        let bytes = read(name);
        let text = bytes
            .as_deref()
            .map(|b| String::from_utf8_lossy(b).into_owned());
        LineTable::parse(text.as_deref(), limit, &fallback)
    };
    LineTables {
        kill: table("kill.txt", 10, built_in(301)),
        die: table("Die.txt", 10, built_in(300)),
        self_kill: table("selfkill.txt", 10, built_in(302)),
        names: table("Names.txt", 2, String::new()),
        host: table("Host.txt", 10, String::new()),
    }
}

/// `FUN_0046CB50`'s session name (the computer's) and `FUN_0046CAC0`'s
/// player name (the user's), with retail's fallbacks.
pub fn local_names() -> (String, String) {
    let env = |keys: &[&str]| {
        keys.iter()
            .find_map(|key| std::env::var(key).ok().filter(|value| !value.is_empty()))
    };
    (
        env(&["COMPUTERNAME", "HOSTNAME"]).unwrap_or_else(|| "Session".into()),
        env(&["USERNAME", "USER"]).unwrap_or_else(|| "unknown".into()),
    )
}

/// `FUN_00449270`: the profile's name and initials. The port keeps no
/// registry name, so these are the values retail shows without one:
/// "Novice" and the slot, "?" around it.
pub fn profile_names(slot: u8) -> (String, String) {
    (format!("Novice {slot}"), format!("?{slot}? "))
}

/// `FUN_00451AF0` in a network game: X and Z at random, Y 0x600 above the
/// higher of the cell's ground and the sea.
pub fn entry_position(
    random: &mut dyn FnMut() -> u16,
    ground_raw: impl Fn(u8, u8) -> i16,
    sea_raw: i16,
) -> [i16; 3] {
    let x = random();
    let z = random();
    let ground = ground_raw((x >> 8) as u8, (z >> 8) as u8);
    let y = ground.max(sea_raw).wrapping_add(0x600);
    [x as i16, y, z as i16]
}

/// The line tables with the loaded string table's built-in lines.
pub fn line_tables_for(
    cache: &crate::resource_cache::ResourceCache,
    game_folder: &Path,
) -> LineTables {
    load_line_tables(game_folder, |id| {
        cache.global_string(id).unwrap_or_default().to_string()
    })
}

/// `EnumSessions` until a session with room answers, for fifteen seconds.
pub fn find_session(
    target: super::transport::SearchTarget,
) -> Result<super::transport::SessionInfo, super::transport::NetSessionError> {
    let mut search = super::transport::SessionSearch::new(target)?;
    let start = std::time::Instant::now();
    while start.elapsed() < std::time::Duration::from_secs(15) {
        if let Some(info) = search.refresh(std::time::Instant::now()).into_iter().next() {
            return Ok(info);
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    Err(super::transport::NetSessionError::JoinTimedOut)
}

/// `FUN_00451AF0`'s network entry over the loaded terrain.
pub fn entry_arrival(
    cache: &crate::resource_cache::ResourceCache,
    random: &mut dyn FnMut() -> u16,
) -> Option<crate::entity::AuthoredPlayerArrival> {
    let terrain = cache.terrain()?;
    let position_raw = entry_position(
        random,
        |x, z| {
            terrain
                .cell(usize::from(x), usize::from(z))
                .map_or(0, |cell| i16::from(cell.height as i8) * 0x20)
        },
        terrain.sea_level_raw(),
    );
    Some(crate::entity::AuthoredPlayerArrival {
        position_raw,
        heading_raw: crate::campaign_transition::RETAIL_CONTROLLER_DEFAULT_ARRIVAL.heading_raw,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_host_picks_a_forced_or_random_arena() {
        assert_eq!(host_world(Some(6), 0), 32);
        assert_eq!(host_world(Some(1), 0), 13);
        assert_eq!(host_world(None, 4), 29);
        assert_eq!(host_world(Some(9), 5), 13);
    }

    #[test]
    fn a_player_enters_above_ground_or_sea() {
        let mut words = [0x1234u16, 0x8000].into_iter();
        let position = entry_position(
            &mut || words.next().unwrap(),
            |x, z| {
                assert_eq!((x, z), (0x12, 0x80));
                -64
            },
            0x100,
        );
        assert_eq!(position, [0x1234, 0x700, -0x8000]);
    }
}
