//! Data-driven menu engine — runtime counterpart of [`crate::menu_data`].
//!
//! Mirrors the original screen engine (menu_nav.c cluster): a 10-deep
//! screen stack (`0x4DCA10 + depth*8` slots holding `{screen, sel, col}`),
//! cursor movement that skips invisible/non-selectable items
//! (FUN_0043C1E0/FUN_0043C4A0), spinner adjustment (FUN_0043B7C0, clamped
//! 0..=max, sound 1 on change / 0 at the limit), and the select dispatcher
//! (FUN_0043C780). Side effects that leave the menu (start game, quit,
//! load slot, …) are returned as [`MenuCommand`]s for the shell to apply.
//!
//! Menu sounds (`FUN_0042A860`): 0 = cursor move / limit, 1 = spinner
//! change, 2 = left/right value change, 3 = select / open submenu / back.

use std::collections::HashMap;

use crate::menu_data::{
    ItemDef, MenuTree, ScreenDef, SelectAction, SettingId, MAIN_RING, VIS_FRONTEND,
};

/// Maximum screen-stack depth (original BSS stack holds 10 slots).
pub const MAX_DEPTH: usize = 10;

/// Global sound id of Klaus's Intro Sequence transform whoosh (0x57 = 87; L3
/// alias 80 → blob 16 at 0.7× rate ± 20% random pitch per trigger).
pub const SOUND_WHOOSH: u32 = 0x57;

/// Global sound id of the background flame-cycle rumble (0x39 = 57),
/// played once per 940 ms billboard loop (and once at menu entry).
pub const SOUND_BG_RUMBLE: u32 = 0x39;

// ── Settings ────────────────────────────────────────────────────────────────

/// Runtime settings struct, mirroring the dword block at 0x4CB3D8.
#[derive(Debug, Clone)]
pub struct Settings {
    values: HashMap<SettingId, u32>,
}

impl Default for Settings {
    fn default() -> Self {
        let mut values = HashMap::new();
        // Exact initialized dwords from V2000.EXE 0x4CB3D8..0x4CB418.
        values.insert(SettingId::SoundVolume, 15);
        values.insert(SettingId::AmbientVolume, 15);
        values.insert(SettingId::Joystick, 1);
        values.insert(SettingId::Resolution, 1);
        values.insert(SettingId::Rendering, 1);
        values.insert(SettingId::Bilinear, 1);
        values.insert(SettingId::FullScreen, 1);
        // Port-only option; Native is the modern default.
        values.insert(SettingId::Scaling, 0);
        values.insert(SettingId::SelfRighting, 1);
        values.insert(SettingId::AbsoluteMode, 0);
        values.insert(SettingId::Sensitivity, 10);
        values.insert(SettingId::ActiveCamera, 6);
        values.insert(SettingId::Targetter, 1);
        values.insert(SettingId::Hud, 1);
        values.insert(SettingId::Language, 0);
        values.insert(SettingId::Vibration, 15);
        Self { values }
    }
}

impl Settings {
    pub fn get(&self, id: SettingId) -> u32 {
        self.values.get(&id).copied().unwrap_or(0)
    }

    pub fn set(&mut self, id: SettingId, v: u32) {
        self.values.insert(id, v);
    }
}

// ── Inputs and commands ─────────────────────────────────────────────────────

/// Abstract menu input (key bindings live in the shell).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuInput {
    Up,
    Down,
    Left,
    Right,
    /// Fire / Space / Enter.
    Select,
    /// Escape.
    Back,
}

/// Side effects for the shell to apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuCommand {
    /// Play menu sound (global sound id 0-3).
    PlaySound(u32),
    /// Persistent Klaus `Intro Sequence` command (`FUN_0042CEA0`):
    /// 1 = reset/return, 4 = select-spin/whoosh. This is separate from the
    /// selected player4 ring prop and its screen-fly animation.
    IntroSequenceCommand(u8),
    /// A screen was pushed (already applied; for transition effects).
    ScreenPushed(u32),
    /// A screen was popped (already applied).
    ScreenPopped,
    /// A setting value changed (sync GameConfig / apply video mode).
    SettingChanged(SettingId, u32),
    /// Start the game (main-ring vehicle prop).
    StartGame,
    /// Quit to desktop (Exit prop / confirmed quit in frontend).
    QuitToDesktop,
    /// Confirmed quit from the pause menu → back to frontend.
    QuitToFrontend,
    /// Request gameplay resume (Continue); the shell applies the authored
    /// pause fly-out before emitting the external resume event.
    ResumeGame,
    /// Enter the save flow (slopt / pause Save Game).
    OpenSaveFlow,
    /// Save/load slot picked in the memory-card flow (slot index unknown
    /// to the engine; shell tracks its own slot list).
    SaveSlotPicked,
    LoadSlotPicked,
    /// Grant a cheat weapon/item (0x3E7xx code).
    GrantCheat(u32),
    /// Cheat-menu world action (skip/complete world), by callback address.
    CheatWorld(u32),
}

// ── Engine ──────────────────────────────────────────────────────────────────

/// One stack slot: `{screen_record, sel_index}` (col_index folded into
/// sel since the port renders single-column screens).
#[derive(Debug, Clone, Copy)]
struct StackSlot {
    screen: u32,
    sel: usize,
}

pub struct MenuEngine {
    tree: MenuTree,
    stack: Vec<StackSlot>,
    pub settings: Settings,
    /// Current visibility mask (VIS_* bits; FUN_0043CBA0).
    pub vis_mask: u32,
    /// Display-mode count, bounds the resolution spinner.
    pub resolution_count: u32,
}

impl MenuEngine {
    /// Create an engine showing the given root screen.
    pub fn new(root: u32, vis_mask: u32) -> Self {
        let tree = MenuTree::load();
        let mut engine = Self {
            tree,
            stack: Vec::new(),
            settings: Settings::default(),
            vis_mask,
            resolution_count: 4,
        };
        engine.push(root);
        engine
    }

    /// Create a frontend menu rooted at the main prop ring.
    pub fn main_menu() -> Self {
        Self::new(MAIN_RING, VIS_FRONTEND)
    }

    // ── Stack access ──

    pub fn depth(&self) -> usize {
        self.stack.len()
    }

    pub fn current(&self) -> Option<&ScreenDef> {
        self.stack.last().and_then(|s| self.tree.screen(s.screen))
    }

    pub fn current_va(&self) -> Option<u32> {
        self.stack.last().map(|s| s.screen)
    }

    /// Selected item index (into the screen's full item array).
    pub fn selected(&self) -> usize {
        self.stack.last().map(|s| s.sel).unwrap_or(0)
    }

    pub fn tree(&self) -> &MenuTree {
        &self.tree
    }

    /// Availability independent of whether the decoded row has an action.
    /// Decorative headings remain available.
    pub fn item_available(&self, item: &ItemDef) -> bool {
        item.available
    }

    /// Whether the cursor may currently rest on and activate this item.
    pub fn item_enabled(&self, item: &ItemDef) -> bool {
        item.selectable(self.vis_mask) && self.item_available(item)
    }

    /// Push a screen, clamping at the original 10-slot depth.
    pub fn push(&mut self, va: u32) -> bool {
        if self.stack.len() >= MAX_DEPTH || self.tree.screen(va).is_none() {
            return false;
        }
        let sel = self
            .tree
            .screen(va)
            .map(|s| first_selectable(s, self.vis_mask))
            .unwrap_or(0);
        self.stack.push(StackSlot { screen: va, sel });
        true
    }

    /// Pop the current screen (keeps the root).
    pub fn pop(&mut self) -> bool {
        if self.stack.len() > 1 {
            self.stack.pop();
            true
        } else {
            false
        }
    }

    /// Replace the whole stack with a new root (mode change).
    pub fn reset(&mut self, root: u32, vis_mask: u32) {
        self.vis_mask = vis_mask;
        self.stack.clear();
        self.push(root);
    }

    // ── Input handling ──

    pub fn handle(&mut self, input: MenuInput) -> Vec<MenuCommand> {
        match input {
            MenuInput::Up => self.move_cursor(-1),
            MenuInput::Down => self.move_cursor(1),
            MenuInput::Left => self.adjust_or_move(-1),
            MenuInput::Right => self.adjust_or_move(1),
            MenuInput::Select => self.select(),
            MenuInput::Back => self.pop_with_back_effects(),
        }
    }

    /// Move the cursor to the screen's Continue row and select it, which is
    /// how a Remastered pad resumes from a pause root. A screen without an
    /// enabled Continue row is left alone.
    pub fn select_continue(&mut self) -> Vec<MenuCommand> {
        let mask = self.vis_mask;
        let Some(index) = self.current().and_then(|screen| {
            screen.items.iter().position(|item| {
                item.select == SelectAction::Continue && item_is_enabled(item, mask)
            })
        }) else {
            return Vec::new();
        };
        if let Some(slot) = self.stack.last_mut() {
            slot.sel = index;
        }
        self.select()
    }

    /// Escape/back: sound 3, pop one screen, and issue Klaus Intro Sequence
    /// command 1 when the resulting depth is shallower than 3
    /// (`FUN_0042BAD0`).
    fn pop_with_back_effects(&mut self) -> Vec<MenuCommand> {
        if !self.pop() {
            return Vec::new();
        }
        let mut cmds = vec![MenuCommand::PlaySound(3), MenuCommand::ScreenPopped];
        if self.depth() < 3 {
            cmds.push(MenuCommand::IntroSequenceCommand(1));
        }
        cmds
    }

    /// Move the cursor to the next/previous selectable item (sound 0).
    fn move_cursor(&mut self, dir: i32) -> Vec<MenuCommand> {
        let mask = self.vis_mask;
        let Some(slot) = self.stack.last_mut() else {
            return Vec::new();
        };
        let Some(screen) = self.tree.screen(slot.screen) else {
            return Vec::new();
        };
        let n = screen.items.len();
        if n == 0 {
            return Vec::new();
        }
        let mut idx = slot.sel;
        for _ in 0..n {
            idx = (idx as i64 + dir as i64).rem_euclid(n as i64) as usize;
            if item_is_enabled(&screen.items[idx], mask) {
                if idx != slot.sel {
                    slot.sel = idx;
                    return vec![MenuCommand::PlaySound(0)];
                }
                return Vec::new();
            }
        }
        Vec::new()
    }

    /// Left/Right: adjust the selected spinner/toggle, else move the cursor
    /// (the main ring navigates horizontally).
    fn adjust_or_move(&mut self, dir: i32) -> Vec<MenuCommand> {
        let action = self.selected_action();
        match action {
            Some(SelectAction::Spinner { setting, max }) => self.spin(setting, max, dir),
            Some(SelectAction::Toggle { setting }) => self.spin(setting, 1, dir),
            Some(SelectAction::ResolutionSpinner) => {
                let max = self.resolution_count.saturating_sub(1);
                self.spin(SettingId::Resolution, max, dir)
            }
            _ => self.move_cursor(dir),
        }
    }

    /// FUN_0043B7C0: clamp to 0..=max; sound 1 on change, 0 at the limit.
    fn spin(&mut self, setting: SettingId, max: u32, dir: i32) -> Vec<MenuCommand> {
        let cur = self.settings.get(setting);
        let new = if dir > 0 {
            (cur + 1).min(max)
        } else {
            cur.saturating_sub(1)
        };
        if new == cur {
            return vec![MenuCommand::PlaySound(0)];
        }
        self.settings.set(setting, new);
        vec![
            MenuCommand::PlaySound(1),
            MenuCommand::SettingChanged(setting, new),
        ]
    }

    fn selected_action(&self) -> Option<SelectAction> {
        let screen = self.current()?;
        let item = screen.items.get(self.selected())?;
        self.item_enabled(item).then_some(item.select)
    }

    /// Fire/Enter: dispatch the selected item's action (FUN_0043C780).
    fn select(&mut self) -> Vec<MenuCommand> {
        let Some(action) = self.selected_action() else {
            return Vec::new();
        };
        match action {
            SelectAction::None | SelectAction::Unknown { .. } => Vec::new(),

            SelectAction::OpenScreen { target } => {
                if self.push(target) {
                    vec![MenuCommand::PlaySound(3), MenuCommand::ScreenPushed(target)]
                } else {
                    vec![MenuCommand::PlaySound(0)]
                }
            }

            // Ring select (FUN_0042BD60): sound 3 + Klaus Intro Sequence
            // command 4. C090 consumes that command on the next actor callback
            // and plays the transform whoosh (global sound 0x57,
            // random-pitched per trigger in the sound layer). The selected
            // ring prop's fly curve is independent.
            SelectAction::RingOpenScreen { target } => {
                if self.push(target) {
                    vec![
                        MenuCommand::PlaySound(3),
                        MenuCommand::IntroSequenceCommand(4),
                        MenuCommand::ScreenPushed(target),
                    ]
                } else {
                    vec![MenuCommand::PlaySound(0)]
                }
            }

            SelectAction::Back => self.pop_with_back_effects(),

            SelectAction::StartGame => {
                vec![MenuCommand::PlaySound(3), MenuCommand::StartGame]
            }
            SelectAction::QuitToDesktop => vec![MenuCommand::QuitToDesktop],
            SelectAction::Continue => {
                vec![MenuCommand::PlaySound(3), MenuCommand::ResumeGame]
            }
            SelectAction::QuitConfirmed => vec![MenuCommand::QuitToFrontend],

            // Fire on a spinner behaves like Right (the row's arrows; screen
            // flag 0x100 marks arrows-as-select rows).
            SelectAction::Spinner { .. }
            | SelectAction::Toggle { .. }
            | SelectAction::ResolutionSpinner => self.adjust_or_move(1),

            SelectAction::Cheat { code } => {
                vec![MenuCommand::PlaySound(3), MenuCommand::GrantCheat(code)]
            }
            SelectAction::CheatWorld { addr } => {
                vec![MenuCommand::PlaySound(3), MenuCommand::CheatWorld(addr)]
            }

            SelectAction::OpenMemoryCard => {
                vec![MenuCommand::PlaySound(3), MenuCommand::OpenSaveFlow]
            }
            SelectAction::SaveSlot | SelectAction::SaveProgress => {
                vec![MenuCommand::PlaySound(3), MenuCommand::SaveSlotPicked]
            }
            SelectAction::LoadSlot => {
                vec![MenuCommand::PlaySound(3), MenuCommand::LoadSlotPicked]
            }

            SelectAction::AdvanceCursor => {
                let mut cmds = vec![MenuCommand::PlaySound(0)];
                cmds.extend(self.move_cursor(1));
                cmds
            }

            // Network stubs: acknowledge without effect.
            SelectAction::NetworkJoin | SelectAction::NetworkHost | SelectAction::FormatCard => {
                vec![MenuCommand::PlaySound(0)]
            }
        }
    }
}

/// Shared interaction policy for navigation and activation. A row without a
/// callback may be a decorative heading and must not inherit disabled styling.
fn item_is_enabled(item: &ItemDef, mask: u32) -> bool {
    item.selectable(mask) && item.available
}

/// First selectable item index under the mask (0 if none).
fn first_selectable(screen: &ScreenDef, mask: u32) -> usize {
    screen
        .items
        .iter()
        .position(|item| item_is_enabled(item, mask))
        .unwrap_or(0)
}

// ── Unit tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menu_data::{CONTROLS, DISPLAY, PAUSE_FULL, SOUNDS, VIS_INGAME};

    #[test]
    fn main_menu_starts_on_ring() {
        let engine = MenuEngine::main_menu();
        assert_eq!(engine.current_va(), Some(MAIN_RING));
        assert_eq!(engine.depth(), 1);
        // Cursor rests on a selectable prop.
        let screen = engine.current().unwrap();
        assert!(screen.items[engine.selected()].selectable(engine.vis_mask));
    }

    #[test]
    fn settings_defaults_match_executable_data_block() {
        let settings = Settings::default();
        for (id, value) in [
            (SettingId::SoundVolume, 15),
            (SettingId::AmbientVolume, 15),
            (SettingId::Joystick, 1),
            (SettingId::Resolution, 1),
            (SettingId::Rendering, 1),
            (SettingId::Bilinear, 1),
            (SettingId::FullScreen, 1),
            (SettingId::Scaling, 0),
            (SettingId::SelfRighting, 1),
            (SettingId::AbsoluteMode, 0),
            (SettingId::Sensitivity, 10),
            (SettingId::ActiveCamera, 6),
            (SettingId::Targetter, 1),
            (SettingId::Hud, 1),
            (SettingId::Language, 0),
            (SettingId::Vibration, 15),
        ] {
            assert_eq!(settings.get(id), value, "{id:?}");
        }
    }

    #[test]
    fn ring_navigates_and_opens_submenus() {
        let mut engine = MenuEngine::main_menu();
        // Ring items are all selectable in the frontend: 7 steps wrap around.
        let start = engine.selected();
        for _ in 0..7 {
            engine.handle(MenuInput::Right);
        }
        assert_eq!(engine.selected(), start, "7 steps should wrap");

        // Walk until the Display prop (RingOpenScreen → DISPLAY) and fire.
        for _ in 0..7 {
            let sel = engine.current().unwrap().items[engine.selected()].select;
            if sel == (SelectAction::RingOpenScreen { target: DISPLAY }) {
                break;
            }
            engine.handle(MenuInput::Right);
        }
        let cmds = engine.handle(MenuInput::Select);
        assert!(cmds.contains(&MenuCommand::IntroSequenceCommand(4)));
        assert!(cmds.contains(&MenuCommand::ScreenPushed(DISPLAY)));
        assert_eq!(engine.current_va(), Some(DISPLAY));
        assert_eq!(engine.depth(), 2);

        // Back pops to the ring.
        let cmds = engine.handle(MenuInput::Back);
        assert!(cmds.contains(&MenuCommand::ScreenPopped));
        assert!(cmds.contains(&MenuCommand::IntroSequenceCommand(1)));
        assert_eq!(engine.current_va(), Some(MAIN_RING));
    }

    #[test]
    fn spinner_clamps_and_reports() {
        let mut engine = MenuEngine::main_menu();
        engine.push(SOUNDS);
        // Find the Sound volume spinner.
        let screen = engine.current().unwrap();
        let idx = screen
            .items
            .iter()
            .position(|i| {
                matches!(
                    i.select,
                    SelectAction::Spinner {
                        setting: SettingId::SoundVolume,
                        ..
                    }
                )
            })
            .unwrap();
        while engine.selected() != idx {
            engine.handle(MenuInput::Down);
        }
        engine.settings.set(SettingId::SoundVolume, 15);
        // At max: Right is a no-op with the limit sound.
        let cmds = engine.handle(MenuInput::Right);
        assert_eq!(cmds, vec![MenuCommand::PlaySound(0)]);
        // Left decrements with the change sound.
        let cmds = engine.handle(MenuInput::Left);
        assert!(cmds.contains(&MenuCommand::SettingChanged(SettingId::SoundVolume, 14)));
        assert!(cmds.contains(&MenuCommand::PlaySound(1)));
    }

    #[test]
    fn rendering_row_toggles_software_and_opengl() {
        for mask in [VIS_FRONTEND, VIS_INGAME] {
            let mut engine = MenuEngine::new(DISPLAY, mask);
            let rendering_index = engine
                .current()
                .unwrap()
                .items
                .iter()
                .position(|item| {
                    item.select
                        == (SelectAction::Toggle {
                            setting: SettingId::Rendering,
                        })
                })
                .unwrap();
            let row = &engine.current().unwrap().items[rendering_index];
            assert!(row.visible(mask));
            assert!(engine.item_available(row));

            engine.stack.last_mut().unwrap().sel = rendering_index;
            assert_eq!(engine.settings.get(SettingId::Rendering), 1);
            assert_eq!(
                engine.handle(MenuInput::Left),
                vec![
                    MenuCommand::PlaySound(1),
                    MenuCommand::SettingChanged(SettingId::Rendering, 0),
                ]
            );
            assert_eq!(engine.settings.get(SettingId::Rendering), 0);
            assert_eq!(
                engine.handle(MenuInput::Right),
                vec![
                    MenuCommand::PlaySound(1),
                    MenuCommand::SettingChanged(SettingId::Rendering, 1),
                ]
            );
            assert_eq!(engine.settings.get(SettingId::Rendering), 1);
        }
    }

    #[test]
    fn controls_screen_spinners_reachable() {
        let mut engine = MenuEngine::main_menu();
        assert!(engine.push(CONTROLS));
        // All four visible setting rows are reachable by cursor.
        let mut seen = std::collections::HashSet::new();
        for _ in 0..10 {
            if let Some(SelectAction::Spinner { setting, .. }) = engine.selected_action() {
                seen.insert(setting);
            }
            engine.handle(MenuInput::Down);
        }
        for s in [
            SettingId::AbsoluteMode,
            SettingId::Sensitivity,
            SettingId::Joystick,
            SettingId::SelfRighting,
        ] {
            assert!(seen.contains(&s), "cursor never reached {s:?}");
        }
    }

    #[test]
    fn hidden_items_skipped() {
        let mut engine = MenuEngine::main_menu();
        engine.reset(PAUSE_FULL, VIS_INGAME);
        // Cheats (visibility 8) must never be reachable without the cheat bit.
        for _ in 0..16 {
            let item = &engine.current().unwrap().items[engine.selected()];
            assert!(
                item.visible(VIS_INGAME),
                "cursor on invisible item {:?}",
                item.label
            );
            engine.handle(MenuInput::Down);
        }
    }

    #[test]
    fn select_continue_takes_the_pause_roots_continue_row() {
        let mut engine = MenuEngine::main_menu();
        engine.reset(PAUSE_FULL, VIS_INGAME);
        engine.handle(MenuInput::Down);
        assert_eq!(
            engine.select_continue(),
            vec![MenuCommand::PlaySound(3), MenuCommand::ResumeGame]
        );
        assert_eq!(
            engine.current().unwrap().items[engine.selected()].select,
            SelectAction::Continue
        );
        // The frontend ring has no Continue row.
        let mut frontend = MenuEngine::main_menu();
        let before = frontend.selected();
        assert!(frontend.select_continue().is_empty());
        assert_eq!(frontend.selected(), before);
    }

    #[test]
    fn quit_confirm_flow() {
        let mut engine = MenuEngine::main_menu();
        engine.reset(PAUSE_FULL, VIS_INGAME);
        // Find Quit (OpenScreen → QUIT_CONFIRM) and fire.
        for _ in 0..10 {
            if matches!(
                engine.selected_action(),
                Some(SelectAction::OpenScreen {
                    target: crate::menu_data::QUIT_CONFIRM
                })
            ) {
                break;
            }
            engine.handle(MenuInput::Down);
        }
        engine.handle(MenuInput::Select);
        assert_eq!(engine.current_va(), Some(crate::menu_data::QUIT_CONFIRM));
        // "Yes" → QuitToFrontend; "No" → ResumeGame (Continue).
        let screen = engine.current().unwrap();
        let yes = screen
            .items
            .iter()
            .position(|i| i.select == SelectAction::QuitConfirmed)
            .unwrap();
        while engine.selected() != yes {
            engine.handle(MenuInput::Down);
        }
        let cmds = engine.handle(MenuInput::Select);
        assert!(cmds.contains(&MenuCommand::QuitToFrontend));
    }

    #[test]
    fn depth_capped_at_ten() {
        let mut engine = MenuEngine::main_menu();
        for _ in 0..20 {
            engine.push(DISPLAY);
        }
        assert!(engine.depth() <= MAX_DEPTH);
    }
}
