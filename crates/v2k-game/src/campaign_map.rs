//! Klaus over the progress map: descriptors `4D0AA0` (the map) and `4D0AD8`
//! (its save menu), between the exit leg (`4D0A30`) and the next world's load.
//!
//! Klaus's iris is opaque, so the map shows only while he is open. The map's
//! entry (`FUN_00454FA0`) gives him command 2 with a 1-us visit; its tick
//! (`FUN_004555A0`) visits him every frame with the frame's microseconds
//! (clamped to 125,000). Space or S closes him again (`FUN_00453C20`,
//! command 3); once `FUN_0042D210` reports him closed the map switches to the
//! load or to the save menu. The save menu's entry (`FUN_004557D0`) gives
//! command 4 and model argument 0; its tick (`FUN_004558A0`) waits while the
//! menu is up, loads after Continue (`+297`), and otherwise gives command 1
//! and model argument 1, then returns to the map after 1001 ms.
//!
//! A switch selects the next descriptor; its entry runs on the next frame.

use crate::game_state::{KlausEffects, MenuShell};

/// The descriptor a leg selected this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CampaignMapSwitch {
    /// `4D0918`: load the route's destination.
    Load,
    /// `4D0AD8`: the save menu.
    SaveMenu,
    /// `4D0AA0`: back to the map.
    Map,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Leg {
    /// `+294` a key asked to leave, `+28B` S asked for the save menu, and
    /// `+2C8` Klaus's closing has started.
    Map {
        leave: bool,
        save: bool,
        closing: bool,
    },
    /// `+28B` the menu is up, `+297` Continue closed it, `+2C8` the return
    /// clock in milliseconds (zero before Klaus's command 1).
    SaveMenu {
        menu_open: bool,
        continue_chosen: bool,
        return_ms: u32,
    },
}

/// The persistent Klaus and the map leg he is in.
pub struct CampaignMapKlaus {
    pub klaus: MenuShell,
    leg: Leg,
}

/// `FUN_004555A0` clamps its delta before the visit.
const MAP_VISIT_MAX_MICROS: u32 = 125_000;
/// `FUN_004558A0` returns to the map once `+2C8` reaches 1001.
const SAVE_MENU_RETURN_MS: u32 = 1001;

impl CampaignMapKlaus {
    /// Klaus as `FUN_00453E00` hands him over; the map's entry runs next.
    pub fn new(klaus: MenuShell) -> Self {
        Self {
            klaus,
            leg: Leg::Map {
                leave: false,
                save: false,
                closing: false,
            },
        }
    }

    /// Whether the save menu's descriptor is current.
    pub fn in_save_menu(&self) -> bool {
        matches!(self.leg, Leg::SaveMenu { .. })
    }

    /// Whether the save menu's screen is up (`+28B`).
    pub fn save_menu_open(&self) -> bool {
        matches!(
            self.leg,
            Leg::SaveMenu {
                menu_open: true,
                ..
            }
        )
    }

    /// Space (`FUN_00455CC0`) or S (`FUN_00455CF0`, which adds `+28B`).
    /// Both stay bound while Klaus closes, so a later S still picks the
    /// save menu. The caller plays sound 3.
    pub fn request_leave(&mut self, save_menu: bool) {
        if let Leg::Map { leave, save, .. } = &mut self.leg {
            *leave = true;
            *save |= save_menu;
        }
    }

    /// `FUN_004555A0`.
    pub fn tick_map(
        &mut self,
        frame_micros: u32,
        effects: &mut impl KlausEffects,
    ) -> Option<CampaignMapSwitch> {
        let Leg::Map {
            leave,
            save,
            closing,
        } = &mut self.leg
        else {
            return None;
        };
        let mut switch = None;
        if *leave {
            if !*closing {
                *closing = true;
                self.klaus.queue_klaus_command(3);
            } else if self.klaus.klaus_finished() {
                switch = Some(if *save {
                    CampaignMapSwitch::SaveMenu
                } else {
                    CampaignMapSwitch::Load
                });
            }
        }
        self.klaus
            .visit_klaus(frame_micros.min(MAP_VISIT_MAX_MICROS), effects);
        switch
    }

    /// `FUN_004557D0`, the save menu's entry, then its first tick.
    pub fn enter_save_menu(
        &mut self,
        frame_micros: u32,
        effects: &mut impl KlausEffects,
    ) -> Option<CampaignMapSwitch> {
        self.leg = Leg::SaveMenu {
            menu_open: true,
            continue_chosen: false,
            return_ms: 0,
        };
        self.klaus.queue_klaus_command(4);
        self.klaus.set_klaus_model_argument(0);
        self.tick_save_menu(frame_micros, effects)
    }

    /// The menu's root popped (`FUN_00450E60` clears `+28B`); Continue had
    /// set `+297` first.
    pub fn close_save_menu(&mut self, continue_chosen: bool) {
        if let Leg::SaveMenu {
            menu_open,
            continue_chosen: chosen,
            ..
        } = &mut self.leg
        {
            *menu_open = false;
            *chosen = continue_chosen;
        }
    }

    /// `FUN_004558A0` after the clock and the menu transition.
    pub fn tick_save_menu(
        &mut self,
        frame_micros: u32,
        effects: &mut impl KlausEffects,
    ) -> Option<CampaignMapSwitch> {
        let Leg::SaveMenu {
            menu_open,
            continue_chosen,
            return_ms,
        } = &mut self.leg
        else {
            return None;
        };
        let mut switch = None;
        if !*menu_open {
            if *continue_chosen {
                switch = Some(CampaignMapSwitch::Load);
            } else if *return_ms == 0 {
                self.klaus.queue_klaus_command(1);
                self.klaus.set_klaus_model_argument(1);
                *return_ms = 1;
            } else {
                *return_ms += frame_micros / 1000;
                if *return_ms >= SAVE_MENU_RETURN_MS {
                    *return_ms = 0;
                    switch = Some(CampaignMapSwitch::Map);
                }
            }
        }
        self.klaus.visit_klaus(frame_micros, effects);
        switch
    }

    /// `FUN_00454EE0` -> `FUN_00454FA0`: the map's entry, after the exit
    /// leg and again after the save menu.
    pub fn enter_map(&mut self) {
        self.klaus.open_over_campaign_map();
        self.leg = Leg::Map {
            leave: false,
            save: false,
            closing: false,
        };
    }

    pub fn into_klaus(self) -> MenuShell {
        self.klaus
    }
}

/// The map's one pass after Replay this World (`+292`): `FUN_00454EE0`
/// skips Klaus's command 2 and `FUN_004555A0` stages the load at once, then
/// still visits him with the clamped delta. The pass draws nothing.
pub fn replay_world_map_pass(
    klaus: &mut MenuShell,
    frame_micros: u32,
    effects: &mut impl KlausEffects,
) {
    klaus.visit_klaus(frame_micros.min(MAP_VISIT_MAX_MICROS), effects);
}
