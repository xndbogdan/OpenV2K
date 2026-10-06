use serde::Serialize;

use crate::entity::MODEL_POOL_PTR;
use crate::entity::TICK_50HZ;
use crate::process::Process;

const GAME_STATE: usize = 0x004D_B200;
const MENU_ACTION: usize = 0x004D_B210;
const FRONTEND_FLAG: usize = 0x004D_B218;
const MENU_SUB_MODE: usize = 0x004D_B224;
const GAME_CONTEXT_PTR: usize = 0x004F_72C8;
const LEVEL_COMPLETION_SIGNAL: usize = 0x004D_1108;
const STACK_BASE: usize = 0x004D_CA10;
const RING_SPIN: usize = 0x004D_CEB0;
const HORIZONTAL_INTERP: usize = 0x004D_CEBC;
const VERTICAL_INTERP: usize = 0x004D_CEC0;
const OPEN_MS: usize = 0x004D_CEC4;
const SCROLL_MS: usize = 0x004D_CEC8;
const TRANSITION_PENDING: usize = 0x004D_CED0;
const TRANSITION_PUSH: usize = 0x004D_CED2;
const TRANSITION_SCREEN: usize = 0x004D_CED4;
const TRANSITION_DELTA: usize = 0x004D_CED8;
const STACK_DEPTH: usize = 0x004D_CEDC;
const FLY_CLOCK: usize = 0x004C_B4E0;
const SETTINGS_BASE: usize = 0x004C_B3D8;
pub const MAIN_RING_SCREEN: u32 = 0x004C_0CF8;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MenuStackEntry {
    pub depth: u32,
    pub screen: u32,
    pub screen_name: &'static str,
    pub selection: i16,
    pub window_scroll: i16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MenuSettings {
    pub sound: i32,
    pub ambient: i32,
    pub joystick: i32,
    pub unused_0c: i32,
    pub resolution: i32,
    pub rendering: i32,
    pub bilinear: i32,
    pub fullscreen: i32,
    pub unused_20: i32,
    pub self_righting: i32,
    pub absolute_mode: i32,
    pub sensitivity: i32,
    pub active_camera: i32,
    pub targetter: i32,
    pub hud: i32,
    pub language: i32,
    pub vibration: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SelectedMenuItem {
    pub address: u32,
    pub flags: u32,
    pub draw_callback: u32,
    pub resource_id: u32,
    pub aux: u32,
    pub select_callback: u32,
    pub arg1: u32,
    pub arg2: u32,
    pub is_model_item: bool,
    pub model_resource: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct MenuSnapshot {
    pub tick_before: u32,
    pub tick_after: u32,
    pub tick_stable: bool,
    pub game_state: i32,
    pub menu_action: u32,
    pub frontend_flag: u32,
    pub menu_sub_mode: u32,
    pub level_completion_signal: u32,
    pub game_context_pointer: u32,
    pub game_context_pointer_after: u32,
    pub context_stable: bool,
    pub context_state_28e: u8,
    pub context_state_28f: u8,
    pub context_state_290: u8,
    pub context_state_292: u8,
    pub context_state_295: u8,
    pub context_state_296: u8,
    pub context_level_result_at_2bc: i32,
    pub depth: u32,
    pub depth_after: u32,
    pub stack_stable: bool,
    pub stack: Vec<MenuStackEntry>,
    pub top_screen: u32,
    pub top_screen_after: u32,
    pub top_screen_name: &'static str,
    pub top_screen_flags: u32,
    pub selected_item: Option<SelectedMenuItem>,
    pub model_items: Vec<SelectedMenuItem>,
    pub background_model_id: Option<u16>,
    pub background_model_resource: u32,
    pub transition_pending: u16,
    pub transition_push: u16,
    pub transition_screen: u32,
    pub transition_screen_name: &'static str,
    pub transition_delta: i32,
    pub fly_clock: u32,
    pub ring_spin: u16,
    pub horizontal_interp: i32,
    pub vertical_interp: i32,
    pub open_ms: u32,
    pub scroll_ms: u32,
    pub settings: MenuSettings,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GameContextSnapshot {
    pointer: u32,
    state: [u8; 9],
    level_result_at_2bc: i32,
}

/// Minimal frontend state for timelines that only need a reliable phase
/// boundary. Unlike [`MenuSnapshot`], this does not walk menu items, resolve
/// models, or read settings on every sample.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MenuPhaseSnapshot {
    pub tick_before: u32,
    pub tick_after: u32,
    pub tick_stable: bool,
    pub game_state: i32,
    pub frontend_flag: u32,
    pub depth: u32,
    pub depth_after: u32,
    pub top_screen: u32,
    pub top_screen_after: u32,
    pub top_screen_name: &'static str,
    pub stack_stable: bool,
    pub stable_main_ring: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MenuEventState {
    pub game_state: i32,
    pub menu_action: u32,
    pub frontend_flag: u32,
    pub menu_sub_mode: u32,
    pub level_completion_signal: u32,
    pub game_context_pointer: u32,
    pub context_state_28e: u8,
    pub context_state_28f: u8,
    pub context_state_290: u8,
    pub context_state_292: u8,
    pub context_state_295: u8,
    pub context_state_296: u8,
    pub context_level_result_at_2bc: i32,
    pub depth: u32,
    pub stack: Vec<MenuStackEntry>,
    pub top_screen: u32,
    pub top_screen_name: &'static str,
    pub top_screen_flags: u32,
    pub selected_item: Option<SelectedMenuItem>,
    pub model_items: Vec<SelectedMenuItem>,
    pub background_model_id: Option<u16>,
    pub background_model_resource: u32,
    pub transition_pending: u16,
    pub transition_push: u16,
    pub transition_screen: u32,
    pub transition_delta: i32,
}

impl MenuSnapshot {
    pub fn event_state(&self) -> MenuEventState {
        MenuEventState {
            game_state: self.game_state,
            menu_action: self.menu_action,
            frontend_flag: self.frontend_flag,
            menu_sub_mode: self.menu_sub_mode,
            level_completion_signal: self.level_completion_signal,
            game_context_pointer: self.game_context_pointer,
            context_state_28e: self.context_state_28e,
            context_state_28f: self.context_state_28f,
            context_state_290: self.context_state_290,
            context_state_292: self.context_state_292,
            context_state_295: self.context_state_295,
            context_state_296: self.context_state_296,
            context_level_result_at_2bc: self.context_level_result_at_2bc,
            depth: self.depth,
            stack: self.stack.clone(),
            top_screen: self.top_screen,
            top_screen_name: self.top_screen_name,
            top_screen_flags: self.top_screen_flags,
            selected_item: self.selected_item.clone(),
            model_items: self.model_items.clone(),
            background_model_id: self.background_model_id,
            background_model_resource: self.background_model_resource,
            transition_pending: self.transition_pending,
            transition_push: self.transition_push,
            transition_screen: self.transition_screen,
            transition_delta: self.transition_delta,
        }
    }
}

pub fn read_snapshot(process: &Process) -> Result<MenuSnapshot, String> {
    let tick_before = process.read_u32(TICK_50HZ)?;
    let game_state = process.read_i32(GAME_STATE)?;
    let menu_action = process.read_u32(MENU_ACTION)?;
    let frontend_flag = process.read_u32(FRONTEND_FLAG)?;
    let menu_sub_mode = process.read_u32(MENU_SUB_MODE)?;
    let level_completion_signal = process.read_u32(LEVEL_COMPLETION_SIGNAL)?;
    let context_before = read_game_context(process)?;
    let depth = process.read_u32(STACK_DEPTH)?;
    if depth > 10 {
        return Err(format!(
            "menu stack depth {depth} is outside the proven 0..10 range"
        ));
    }

    let mut stack = Vec::with_capacity(depth as usize);
    // Slot zero is scratch/unused. Active menu screens occupy slots 1..depth.
    for index in 1..=depth {
        let address = STACK_BASE + index as usize * 8;
        let screen = process.read_u32(address)?;
        stack.push(MenuStackEntry {
            depth: index,
            screen,
            screen_name: screen_name(screen),
            selection: process.read_i16(address + 4)?,
            window_scroll: process.read_i16(address + 6)?,
        });
    }
    let top_screen = stack.last().map_or(0, |entry| entry.screen);
    let top_screen_flags = if top_screen == 0 {
        0
    } else {
        process.read_u32(top_screen as usize + 0x08)?
    };
    let selected_item = read_selected_item(process, stack.last())?;
    let model_items = read_model_items(process, top_screen)?;
    // Screen flag 0x10 asks FUN_0043B130 to draw the shared options backdrop
    // (global model 73) before the ordinary item rows.
    let background_model_id = (top_screen_flags & 0x10 != 0).then_some(73);
    let model_pool = process.read_u32(MODEL_POOL_PTR)?;
    let background_model_resource = background_model_id
        .and_then(|model_id| {
            (model_pool != 0)
                .then(|| {
                    process
                        .read_u32(model_pool as usize + usize::from(model_id) * 4)
                        .ok()
                })
                .flatten()
        })
        .unwrap_or(0);

    let transition_screen = process.read_u32(TRANSITION_SCREEN)?;
    let transition_pending = process.read_u16(TRANSITION_PENDING)?;
    let transition_push = process.read_u16(TRANSITION_PUSH)?;
    let transition_delta = process.read_i32(TRANSITION_DELTA)?;
    let fly_clock = process.read_u32(FLY_CLOCK)?;
    let ring_spin = process.read_u16(RING_SPIN)?;
    let horizontal_interp = process.read_i32(HORIZONTAL_INTERP)?;
    let vertical_interp = process.read_i32(VERTICAL_INTERP)?;
    let open_ms = process.read_u32(OPEN_MS)?;
    let scroll_ms = process.read_u32(SCROLL_MS)?;
    let settings = MenuSettings {
        sound: process.read_i32(SETTINGS_BASE)?,
        ambient: process.read_i32(SETTINGS_BASE + 0x04)?,
        joystick: process.read_i32(SETTINGS_BASE + 0x08)?,
        unused_0c: process.read_i32(SETTINGS_BASE + 0x0C)?,
        resolution: process.read_i32(SETTINGS_BASE + 0x10)?,
        rendering: process.read_i32(SETTINGS_BASE + 0x14)?,
        bilinear: process.read_i32(SETTINGS_BASE + 0x18)?,
        fullscreen: process.read_i32(SETTINGS_BASE + 0x1C)?,
        unused_20: process.read_i32(SETTINGS_BASE + 0x20)?,
        self_righting: process.read_i32(SETTINGS_BASE + 0x24)?,
        absolute_mode: process.read_i32(SETTINGS_BASE + 0x28)?,
        sensitivity: process.read_i32(SETTINGS_BASE + 0x2C)?,
        active_camera: process.read_i32(SETTINGS_BASE + 0x30)?,
        targetter: process.read_i32(SETTINGS_BASE + 0x34)?,
        hud: process.read_i32(SETTINGS_BASE + 0x38)?,
        language: process.read_i32(SETTINGS_BASE + 0x3C)?,
        vibration: process.read_i32(SETTINGS_BASE + 0x40)?,
    };
    let depth_after = process.read_u32(STACK_DEPTH)?;
    let top_screen_after = if (1..=10).contains(&depth_after) {
        process.read_u32(STACK_BASE + depth_after as usize * 8)?
    } else {
        0
    };
    let context_after = read_game_context(process)?;
    let tick_after = process.read_u32(TICK_50HZ)?;

    Ok(MenuSnapshot {
        tick_before,
        tick_after,
        tick_stable: tick_before == tick_after,
        game_state,
        menu_action,
        frontend_flag,
        menu_sub_mode,
        level_completion_signal,
        game_context_pointer: context_before.pointer,
        game_context_pointer_after: context_after.pointer,
        context_stable: context_before == context_after,
        context_state_28e: context_before.state[0],
        context_state_28f: context_before.state[1],
        context_state_290: context_before.state[2],
        context_state_292: context_before.state[4],
        context_state_295: context_before.state[7],
        context_state_296: context_before.state[8],
        context_level_result_at_2bc: context_before.level_result_at_2bc,
        depth,
        depth_after,
        stack_stable: depth == depth_after && top_screen == top_screen_after,
        stack,
        top_screen,
        top_screen_after,
        top_screen_name: screen_name(top_screen),
        top_screen_flags,
        selected_item,
        model_items,
        background_model_id,
        background_model_resource,
        transition_pending,
        transition_push,
        transition_screen,
        transition_screen_name: screen_name(transition_screen),
        transition_delta,
        fly_clock,
        ring_spin,
        horizontal_interp,
        vertical_interp,
        open_ms,
        scroll_ms,
        settings,
    })
}

fn read_game_context(process: &Process) -> Result<GameContextSnapshot, String> {
    let pointer = process.read_u32(GAME_CONTEXT_PTR)?;
    let (state, level_result_at_2bc) = if pointer == 0 {
        ([0; 9], 0)
    } else {
        let state = process
            .read_bytes(pointer as usize + 0x28E, 9)?
            .try_into()
            .expect("nine-byte game-context state read");
        let result = process.read_i32(pointer as usize + 0x2BC)?;
        (state, result)
    };
    Ok(GameContextSnapshot {
        pointer,
        state,
        level_result_at_2bc,
    })
}

pub fn read_phase_snapshot(process: &Process) -> Result<MenuPhaseSnapshot, String> {
    let tick_before = process.read_u32(TICK_50HZ)?;
    let game_state = process.read_i32(GAME_STATE)?;
    let frontend_flag = process.read_u32(FRONTEND_FLAG)?;
    let depth = process.read_u32(STACK_DEPTH)?;
    if depth > 10 {
        return Err(format!(
            "menu stack depth {depth} is outside the proven 0..10 range"
        ));
    }
    let top_screen = read_top_screen(process, depth)?;
    let depth_after = process.read_u32(STACK_DEPTH)?;
    let top_screen_after = if depth_after <= 10 {
        read_top_screen(process, depth_after)?
    } else {
        0
    };
    let tick_after = process.read_u32(TICK_50HZ)?;
    let tick_stable = tick_before == tick_after;
    let stack_stable = depth == depth_after && top_screen == top_screen_after;
    let stable_main_ring = is_stable_main_ring(
        tick_stable,
        stack_stable,
        game_state,
        frontend_flag,
        depth,
        top_screen,
    );
    Ok(MenuPhaseSnapshot {
        tick_before,
        tick_after,
        tick_stable,
        game_state,
        frontend_flag,
        depth,
        depth_after,
        top_screen,
        top_screen_after,
        top_screen_name: screen_name(top_screen),
        stack_stable,
        stable_main_ring,
    })
}

fn read_top_screen(process: &Process, depth: u32) -> Result<u32, String> {
    if depth == 0 {
        Ok(0)
    } else {
        process.read_u32(STACK_BASE + depth as usize * 8)
    }
}

fn is_stable_main_ring(
    tick_stable: bool,
    stack_stable: bool,
    game_state: i32,
    frontend_flag: u32,
    depth: u32,
    top_screen: u32,
) -> bool {
    tick_stable
        && stack_stable
        && game_state == 2
        && frontend_flag != 0
        && depth == 1
        && top_screen == MAIN_RING_SCREEN
}

fn read_selected_item(
    process: &Process,
    top: Option<&MenuStackEntry>,
) -> Result<Option<SelectedMenuItem>, String> {
    let Some(top) = top else {
        return Ok(None);
    };
    if top.screen == 0 || !(0..=255).contains(&top.selection) {
        return Ok(None);
    }
    let items = process.read_u32(top.screen as usize)?;
    if items == 0 {
        return Ok(None);
    }
    let address = items as usize + top.selection as usize * 0x1C;
    read_item(process, address).map(Some)
}

fn read_model_items(process: &Process, screen: u32) -> Result<Vec<SelectedMenuItem>, String> {
    if screen == 0 {
        return Ok(Vec::new());
    }
    let items = process.read_u32(screen as usize)?;
    if items == 0 {
        return Ok(Vec::new());
    }
    let item_count: usize = (0..3)
        .map(|group| {
            process
                .read_u16(screen as usize + 0x14 + group * 10 + 8)
                .map(usize::from)
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .sum();
    if item_count > 256 {
        return Err(format!(
            "menu screen {screen:08X} advertises implausible item count {item_count}"
        ));
    }
    let mut model_items = Vec::new();
    for index in 0..item_count {
        let item = read_item(process, items as usize + index * 0x1C)?;
        if item.is_model_item {
            model_items.push(item);
        }
    }
    Ok(model_items)
}

fn read_item(process: &Process, address: usize) -> Result<SelectedMenuItem, String> {
    let draw_callback = process.read_u32(address + 0x04)?;
    let resource_id = process.read_u32(address + 0x08)?;
    let is_model_item = draw_callback == 0x0043_B410;
    let model_pool = process.read_u32(MODEL_POOL_PTR)?;
    let model_resource = if is_model_item && model_pool != 0 {
        process
            .read_u32(model_pool as usize + resource_id as usize * 4)
            .unwrap_or(0)
    } else {
        0
    };
    Ok(SelectedMenuItem {
        address: address as u32,
        flags: process.read_u32(address)?,
        draw_callback,
        resource_id,
        aux: process.read_u32(address + 0x0C)?,
        select_callback: process.read_u32(address + 0x10)?,
        arg1: process.read_u32(address + 0x14)?,
        arg2: process.read_u32(address + 0x18)?,
        is_model_item,
        model_resource,
    })
}

fn screen_name(screen: u32) -> &'static str {
    match screen {
        0 => "none",
        MAIN_RING_SCREEN => "main_ring",
        0x004C_11D0 => "network",
        0x004C_1260 => "options_ingame",
        0x004C_12F0 => "sounds",
        0x004C_13D0 => "controls",
        0x004C_14D0 => "display",
        0x004C_1740 => "memory_card",
        0x004C_4DD0 => "quit_confirm",
        0x004D_0FE8 => "cheats",
        0x004D_1090 => "pause_simple",
        0x004D_1190 => "pause_full",
        _ => "unlabelled",
    }
}

#[cfg(test)]
mod tests {
    use super::{is_stable_main_ring, MAIN_RING_SCREEN};

    #[test]
    fn stable_main_ring_requires_every_retail_anchor() {
        assert!(is_stable_main_ring(true, true, 2, 1, 1, MAIN_RING_SCREEN));
        for candidate in [
            is_stable_main_ring(false, true, 2, 1, 1, MAIN_RING_SCREEN),
            is_stable_main_ring(true, false, 2, 1, 1, MAIN_RING_SCREEN),
            is_stable_main_ring(true, true, 1, 1, 1, MAIN_RING_SCREEN),
            is_stable_main_ring(true, true, 2, 0, 1, MAIN_RING_SCREEN),
            is_stable_main_ring(true, true, 2, 1, 2, MAIN_RING_SCREEN),
            is_stable_main_ring(true, true, 2, 1, 1, 0),
        ] {
            assert!(!candidate);
        }
    }
}
