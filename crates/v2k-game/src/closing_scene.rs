//! The closing scene: overlay 49 played in session phase 3.
//!
//! The dead Queen's `FUN_00456F20` leaves world 47 for logical world `0x25`
//! in phase 3. `FUN_00453E00` keeps that phase for world `0x25`, so
//! `FUN_00451710` loads overlay 49 without a player (`FUN_00450390`) and
//! clears the behavior bits of its capability-8 bodies. The world then runs
//! as a cinematic: `FUN_004503C0` drives the class-1 camera proxy toward the
//! body `FUN_00452270` selected from the Section-2 records, which
//! `FUN_00452CB0` visits every frame. Its last record, command `0x40`
//! (`FUN_00452770`), sets the next world to 1 and leaves; world 13 then loads
//! in ordinary play (phase 5).

use crate::entity::EntityManager;
use crate::entity_collision_state::RetailRuntimeValue;
use crate::gameplay_notifications::{section2_record_fields, Section2RecordFields};
use crate::opening::{IntroCameraController, IntroCameraSubject};

/// Overlay 49, logical world `0x25`.
pub const CLOSING_SCENE_LEVEL_ID: u32 = 49;

/// `FUN_00452CB0` starts a world's records at its `4D056A` table entry. Every
/// world's entry is its first Section-2 string except logical `0x25`'s: 461,
/// one past overlay 49's first string (460). Its first record, a camera
/// command for the first three seconds, is never visited.
pub const FIRST_VISITED_RECORD: usize = 1;

/// `FUN_00452270` case `0x40`.
const LEAVE_COMMAND: u32 = 0x40;

/// The records `FUN_00452CB0` visits for the closing scene, up to `#`.
pub fn visited_records(strings: &[String]) -> impl Iterator<Item = &str> {
    strings
        .iter()
        .skip(FIRST_VISITED_RECORD)
        .map(String::as_str)
        .take_while(|record| !record.starts_with('#'))
}

/// `FUN_00451710` moves the class-1 camera proxy, carried from the previous
/// world, up 0x200 and along Z by `0x700 - far`, where far is Section 13
/// `+88` shifted left eight (`FUN_0042E920`) and truncated to a word.
pub fn camera_arrival_raw(carried_raw: [i16; 3], far_cells: u32) -> [i16; 3] {
    let far_raw = far_cells.wrapping_shl(8) as i16;
    [
        carried_raw[0],
        carried_raw[1].wrapping_add(0x200),
        carried_raw[2].wrapping_add(0x700i16.wrapping_sub(far_raw)),
    ]
}

/// One frame's outcome of the closing scene's commands.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ClosingSceneCommandOutcome {
    /// `FUN_00452770`: session `+34` to 1, `+290` set, and `FUN_00456750`'s
    /// flash requested.
    pub leave: bool,
}

pub struct ClosingScene {
    pub camera: IntroCameraController,
    commands: Vec<Section2RecordFields>,
    /// Session `+2D8`, the stamp last selected for the camera. A world-49
    /// stamp never equals the stale value retail carries in.
    selected_stamp: u32,
    /// Session `+308`. The handle retail carries in never resolves here.
    camera_target: Option<u32>,
    /// Phase 4, the attract demo: `FUN_004503C0` picks the subject itself and
    /// the world has no command records.
    pub attract: Option<crate::attract_demo::AttractCamera>,
}

impl ClosingScene {
    pub fn new(strings: &[String], camera: IntroCameraController) -> Self {
        Self {
            camera,
            commands: visited_records(strings)
                .map(section2_record_fields)
                .filter(|fields| fields.command >= LEAVE_COMMAND)
                .collect(),
            selected_stamp: 0,
            camera_target: None,
            attract: None,
        }
    }

    /// A phase-4 attract world: its records (`FUN_00452CB0` from global string
    /// `0x77`) hold only the "Demo" caption, no commands.
    pub fn attract(camera: IntroCameraController) -> Self {
        Self {
            camera,
            commands: Vec::new(),
            selected_stamp: 0,
            camera_target: None,
            attract: Some(crate::attract_demo::AttractCamera::default()),
        }
    }

    /// `FUN_00452790`'s calls to `FUN_00452270` for every active command
    /// record, in table order, at `(DAT_004FED60 * 1000) / 50`.
    pub fn present(
        &mut self,
        entities: &mut EntityManager,
        retail_tick: u32,
    ) -> ClosingSceneCommandOutcome {
        let millis = retail_tick.wrapping_mul(1000) as i32 / 50;
        let mut outcome = ClosingSceneCommandOutcome::default();
        for command in self
            .commands
            .iter()
            .filter(|fields| fields.active_at(millis))
        {
            if command.command == LEAVE_COMMAND {
                outcome.leave = true;
                continue;
            }
            let operation = command.operation;
            if command.command == self.selected_stamp && operation != 2 && operation != 3 {
                continue;
            }
            // The walk stops at the first body with this stamp; a miss leaves
            // it on the last live body.
            let mut found = None;
            for id in entities.retail_live_order_ids() {
                found = Some(id);
                let stamp = entities
                    .iter_all()
                    .find(|entity| entity.id == id)
                    .map(|entity| entity.construction_stamp_at_0xb4);
                if stamp == Some(RetailRuntimeValue::Known(command.command as u16)) {
                    break;
                }
            }
            let Some(id) = found else {
                continue;
            };
            match operation {
                2 | 3 => entities.set_behavior_components_enabled(id, operation == 2),
                _ => {
                    self.selected_stamp = command.command;
                    self.camera_target = Some(id);
                }
            }
        }
        outcome
    }

    /// `FUN_004503C0` follows the selected body while it lives.
    pub fn camera_subject(&self, entities: &EntityManager) -> Option<IntroCameraSubject> {
        let id = match &self.attract {
            Some(attract) => attract.subject()?,
            None => self.camera_target?,
        };
        entities
            .iter_all()
            .find(|entity| entity.id == id)
            .map(|entity| IntroCameraSubject {
                position_raw: entity.position_raw(),
                velocity_raw: entity.velocity_raw(),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_arrival_adds_451710_offsets_in_words() {
        // far 30 cells: 0x700 - 0x1E00 = -0x1700.
        assert_eq!(
            camera_arrival_raw([100, -2772, 32000], 30),
            [100, -2772 + 0x200, 32000i16.wrapping_sub(0x1700)]
        );
    }

    #[test]
    fn first_record_is_skipped_and_commands_keep_table_order() {
        let strings = [
            "\t<    *, 3000,*, 0, 0, 0,*, 37889>",
            "\t<    *, 1000,*, 0, 0, 0,2, 37890>",
            "< 2000, 4000,2,30,*>WELL DONE!",
            "< 3000, 4000,*, 0, 0, 0,*, 37890>",
            "<290000,292000,1,       *,    64>",
            "#",
            "<*,*,*,0,0,0,*,37891>",
        ]
        .map(str::to_owned);
        let visited: Vec<_> = visited_records(&strings).collect();
        assert_eq!(visited.len(), 4);
        let scene = ClosingScene::new(&strings, IntroCameraController::default());
        let commands: Vec<_> = scene
            .commands
            .iter()
            .map(|fields| (fields.command, fields.operation, fields.start_millis))
            .collect();
        assert_eq!(commands, [(37890, 2, 0), (37890, 0, 3000), (64, 0, 290000)]);
        assert!(scene.commands[2].active_at(292_000));
        assert!(!scene.commands[2].active_at(292_001));
    }

    #[test]
    fn leave_command_runs_at_290_seconds() {
        let strings = ["<x>", "<290000,292000,1,       *,    64>", "#"].map(str::to_owned);
        let mut scene = ClosingScene::new(&strings, IntroCameraController::default());
        let mut manager = EntityManager::from_entities_for_test(Vec::new());
        assert!(!scene.present(&mut manager, 14_499).leave);
        assert!(scene.present(&mut manager, 14_500).leave);
        assert!(scene.present(&mut manager, 14_600).leave);
        assert!(!scene.present(&mut manager, 14_601).leave);
    }
}
