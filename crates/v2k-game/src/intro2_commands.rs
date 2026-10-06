//! Section-2 actor commands, evaluated by 52CB0/52790 during presentation.
//!
//! 44F8B0 clears the selected handle. 52270 then retains selection across
//! record gaps, and ORs/clears 68000 on every operation-2/3 record visit.
//! The next world update consumes these writes, including behind Klaus.

use crate::entity::EntityManager;
use crate::opening::{intro2_backdrop, Intro2Backdrop, IntroCameraSubject};
use crate::resource_cache::ResourceCache;

// Accepted Intro2 constructor census: 56C20's consecutive allocation tags
// 38914..38975 correspond to Section-13 spawns 0..61. They are not live IDs.
const INTRO2_FIRST_ALLOCATION_TAG: u32 = 38914;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2ActorOperation {
    SelectCamera,
    EnableComponents,
    DisableComponents,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Intro2ActorCommand {
    pub start_millis: i32,
    pub end_millis: Option<i32>,
    pub spawn_index: usize,
    pub operation: Intro2ActorOperation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2CommandError {
    LevelUnavailable,
    MissingEndMarker,
    MalformedRecord(usize),
    UnknownAllocationTag { record: usize, tag: u32 },
}

#[derive(Debug, Clone, Default)]
pub struct Intro2Commands {
    records: Vec<Intro2ActorCommand>,
    camera_spawn: Option<usize>,
}

impl Intro2Commands {
    pub fn from_cache(cache: &ResourceCache) -> Result<Self, Intro2CommandError> {
        let level = cache.level().ok_or(Intro2CommandError::LevelUnavailable)?;
        let count = level
            .level
            .as_ref()
            .ok_or(Intro2CommandError::LevelUnavailable)?
            .entities
            .len();
        Self::parse(&level.strings, count)
    }

    fn parse(strings: &[String], spawn_count: usize) -> Result<Self, Intro2CommandError> {
        let end = strings
            .iter()
            .position(|s| s.starts_with('#'))
            .ok_or(Intro2CommandError::MissingEndMarker)?;
        let mut records = Vec::new();
        for (index, raw) in strings[..end].iter().enumerate() {
            let raw = raw.trim_start_matches([' ', '\t']);
            let Some(tag) = raw.strip_prefix('<') else {
                continue;
            };
            let tag = tag
                .split_once('>')
                .ok_or(Intro2CommandError::MalformedRecord(index))?
                .0;
            let mut fields = [0i32; 8];
            let mut cursor = 0usize;
            for token in tag.split(',') {
                let token = token.trim();
                let value = if token == "*" || token.is_empty() {
                    0
                } else {
                    token
                        .parse()
                        .map_err(|_| Intro2CommandError::MalformedRecord(index))?
                };
                if cursor < fields.len() {
                    fields[cursor] = value;
                }
                // A nonzero layout preset skips explicit x/y/width tokens.
                cursor = if cursor == 2 && value != 0 {
                    6
                } else {
                    cursor + 1
                };
            }
            let allocation_tag = fields[7] as u32;
            if allocation_tag <= 0x40 {
                continue;
            }
            let spawn_index = allocation_tag
                .checked_sub(INTRO2_FIRST_ALLOCATION_TAG)
                .map(|index| index as usize)
                .filter(|index| *index < spawn_count)
                .ok_or(Intro2CommandError::UnknownAllocationTag {
                    record: index,
                    tag: allocation_tag,
                })?;
            records.push(Intro2ActorCommand {
                start_millis: fields[0],
                end_millis: (fields[1] != 0).then_some(fields[1]),
                spawn_index,
                operation: match fields[6] {
                    2 => Intro2ActorOperation::EnableComponents,
                    3 => Intro2ActorOperation::DisableComponents,
                    _ => Intro2ActorOperation::SelectCamera,
                },
            });
        }
        Ok(Self {
            records,
            camera_spawn: None,
        })
    }

    pub fn records(&self) -> &[Intro2ActorCommand] {
        &self.records
    }

    pub fn camera_subject(&self, entities: &EntityManager) -> Option<IntroCameraSubject> {
        let spawn = self.camera_spawn?;
        entities
            .iter()
            .find(|entity| entity.authored_spawn_index == Some(spawn))
            .map(|entity| IntroCameraSubject {
                position_raw: entity.position_raw(),
                velocity_raw: entity.velocity_raw(),
            })
    }

    pub fn present(&mut self, entities: &mut EntityManager, retail_tick: u32) {
        if intro2_backdrop(retail_tick) != Intro2Backdrop::World {
            return;
        }
        let millis = retail_tick.wrapping_mul(1000) as i32 / 50;
        for record in &self.records {
            if millis < record.start_millis || record.end_millis.is_some_and(|end| end < millis) {
                continue;
            }
            match record.operation {
                Intro2ActorOperation::SelectCamera => self.camera_spawn = Some(record.spawn_index),
                Intro2ActorOperation::EnableComponents => {
                    entities.set_authored_behavior_components_enabled(record.spawn_index, true);
                }
                Intro2ActorOperation::DisableComponents => {
                    entities.set_authored_behavior_components_enabled(record.spawn_index, false);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn intro2_commands_parse_presets_explicit_operations_and_end_marker() {
        let strings = [
            "<*,5000,*,0,0,0,*,38946>",
            "<500,2000,*,0,0,0,2,38947>",
            "<700,*,*,0,0,0,3,38947>",
            "<2000,6000,2,30,*>Caption",
            "#",
            "<*,*,*,0,0,0,2,99999>",
        ]
        .map(str::to_owned);
        let commands = Intro2Commands::parse(&strings, 62).unwrap();
        assert_eq!(commands.records.len(), 3);
        assert_eq!(commands.records[0].spawn_index, 32);
        assert_eq!(
            commands.records[1].operation,
            Intro2ActorOperation::EnableComponents
        );
        assert_eq!(
            commands.records[2].operation,
            Intro2ActorOperation::DisableComponents
        );
        assert_eq!(commands.records[2].end_millis, None);
        assert_eq!(commands.camera_spawn, None);
    }
}
