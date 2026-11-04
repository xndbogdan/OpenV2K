//! The harmful sea of world-style-5 worlds (Volcano, Alien1-3, Arena5).
//!
//! `FUN_00411A80` first runs `FUN_00417130`, before its pair walker: when
//! `FUN_0042F230` finds the world's style 5, every live body with state bits
//! `0x8800` and without `0x4000` takes packet `{3, 0, 40000, 0, -3}` through
//! `FUN_00415040` at ratio `(world clock >> 2) / 0xFFFF` (so the sea burns
//! harder the longer the world has run), if it holds bit `0x200000` or its
//! active model's lower half reaches below the sea word.

use v2k_formats::{levels::LevelDescriptor, terrain::TerrainGrid};

use crate::{
    attached_particle_damage::{
        apply_scaled_checked_damage, AttachedParticleDamageBlock, AttachedParticleDamageFrame,
        AttachedParticleDamageWorld,
    },
    damage::{DamageDeliveryRecord, DamagePacket},
    entity::Entity,
    entity_collision_state::{
        active_model_slot_from_state_flags, RetailRuntimeValue, RetailStateWord,
        ACTIVE_MODEL_SLOT_HIGH_STATE_BIT, ACTIVE_MODEL_SLOT_LOW_STATE_BIT,
    },
    resource_cache::ResourceCache,
};

pub const HARMFUL_SEA_DAMAGE_PACKET: DamagePacket = DamagePacket {
    channels: [3, 0],
    amounts_raw: [40_000, 0],
};
/// The packet's fifth word, read by later damage consumers as its source.
const HARMFUL_SEA_SOURCE_RAW: u32 = 0xFFFF_FFFD;
const HARMFUL_SEA_WORLD_STYLE: u32 = 5;

/// `FUN_0042F230`: the sea word, in a style-5 world only.
pub fn harmful_sea_level_raw(level: &LevelDescriptor, terrain: &TerrainGrid) -> Option<i16> {
    (level.world_style == HARMFUL_SEA_WORLD_STYLE).then(|| (terrain.header[0] >> 8) as i16)
}

/// Why one body's visit could not be decided or delivered.
#[derive(Debug)]
pub enum HarmfulSeaBlock {
    StateUnresolved,
    ActiveModelUnresolved,
    Damage(AttachedParticleDamageBlock),
}

/// `FUN_00417130`'s test of one body.
pub fn body_in_harmful_sea(
    entity: &Entity,
    cache: &ResourceCache,
    sea_raw: i16,
) -> Result<bool, HarmfulSeaBlock> {
    harmful_sea_reaches(
        entity.collision.state_flags_at_0x08,
        entity.position_raw()[1],
        sea_raw,
        |slot| {
            entity
                .model_in_slot(slot)
                .and_then(|id| cache.global_model(id))
                .map(|model| model.collision_radius_raw)
        },
    )
}

/// The test on a body's state word, Y and the active model's header `+0x0A`
/// word (state bit `0x2000` selects slot 2 over slot 0 here, `0x4000` being
/// clear).
fn harmful_sea_reaches(
    state: RetailStateWord,
    y_raw: i16,
    sea_raw: i16,
    collision_radius_raw: impl FnOnce(usize) -> Option<u16>,
) -> Result<bool, HarmfulSeaBlock> {
    let RetailRuntimeValue::Known(gate) = state.masked(0x4000 | 0x8800) else {
        return Err(HarmfulSeaBlock::StateUnresolved);
    };
    if gate != 0x8800 {
        return Ok(false);
    }
    let RetailRuntimeValue::Known(always) = state.masked(0x0020_0000) else {
        return Err(HarmfulSeaBlock::StateUnresolved);
    };
    if always != 0 {
        return Ok(true);
    }
    let RetailRuntimeValue::Known(slot_bits) =
        state.masked(ACTIVE_MODEL_SLOT_LOW_STATE_BIT | ACTIVE_MODEL_SLOT_HIGH_STATE_BIT)
    else {
        return Err(HarmfulSeaBlock::StateUnresolved);
    };
    let radius = collision_radius_raw(active_model_slot_from_state_flags(slot_bits))
        .ok_or(HarmfulSeaBlock::ActiveModelUnresolved)?;
    Ok(i32::from(sea_raw) > i32::from(y_raw) - i32::from(radius >> 1))
}

/// Report an undecided body once per entity type and process.
fn first_block_for_type(entity_type: u32) -> bool {
    static REPORTED: std::sync::Mutex<std::collections::BTreeSet<u32>> =
        std::sync::Mutex::new(std::collections::BTreeSet::new());
    REPORTED
        .lock()
        .map_or(false, |mut reported| reported.insert(entity_type))
}

/// Run `FUN_00417130` over the live list, each body tested at its visit.
/// Returns the bodies damaged with their filtered amounts, and the first
/// undecided or refused body of each entity type (later ones are skipped
/// silently, each still without its damage).
pub fn apply_harmful_sea(
    frame: AttachedParticleDamageFrame<'_>,
) -> (Vec<(u32, i32)>, Vec<(u32, HarmfulSeaBlock)>) {
    let AttachedParticleDamageFrame {
        resources,
        entities,
        world_fx,
        static_damage,
        scheduler,
        notifications,
        retail_tick,
        mut world,
    } = frame;
    let mut damaged = Vec::new();
    let mut blocked = Vec::new();
    let Some(sea_raw) = resources
        .level_desc()
        .zip(resources.terrain())
        .and_then(|(level, terrain)| harmful_sea_level_raw(level, terrain))
    else {
        return (damaged, blocked);
    };
    let ratio = ((retail_tick >> 2) & 0xFFFF) as u16;
    let ids: Vec<u32> = entities.retail_live_order_ids().collect();
    for id in ids {
        let Some(entity) = entities.iter_all().find(|entity| entity.id == id) else {
            continue;
        };
        let entity_type = entity.entity_type;
        match body_in_harmful_sea(entity, resources, sea_raw) {
            Ok(false) => continue,
            Err(block) => {
                if first_block_for_type(entity_type) {
                    blocked.push((id, block));
                }
                continue;
            }
            Ok(true) => {}
        }
        let delivery = DamageDeliveryRecord {
            packet: HARMFUL_SEA_DAMAGE_PACKET,
            source_entity_type_raw: HARMFUL_SEA_SOURCE_RAW,
            owner_handle: 0,
        };
        let result = apply_scaled_checked_damage(
            AttachedParticleDamageFrame {
                resources: &mut *resources,
                entities: &mut *entities,
                world_fx: &mut *world_fx,
                static_damage: &mut *static_damage,
                scheduler: &mut *scheduler,
                notifications: &mut *notifications,
                retail_tick,
                world: match &mut world {
                    AttachedParticleDamageWorld::Cinematic => {
                        AttachedParticleDamageWorld::Cinematic
                    }
                    AttachedParticleDamageWorld::Playing {
                        player_hull,
                        extra_lives,
                    } => AttachedParticleDamageWorld::Playing {
                        player_hull: &mut **player_hull,
                        extra_lives: *extra_lives,
                    },
                },
            },
            id,
            delivery,
            ratio,
            0xFFFF,
        );
        match result {
            Ok(amount) => damaged.push((id, amount)),
            Err(block) => {
                if first_block_for_type(entity_type) {
                    blocked.push((id, HarmfulSeaBlock::Damage(block)));
                }
            }
        }
    }
    (damaged, blocked)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[v2k_test_support::retail_test]
    fn only_style_five_worlds_have_a_harmful_sea() {
        use v2k_formats::{ovl::OvlFile, sections};
        let style = |level: u32| {
            let path = v2k_test_support::retail_dir()
                .join("Overlay")
                .join(format!("1X{level}XX.OVL"));
            let ovl = OvlFile::parse(&std::fs::read(path).unwrap()).unwrap();
            sections::parse_level(&ovl).unwrap().world_style
        };
        let harmful: Vec<u32> = (13..=50)
            .filter(|&level| style(level) == HARMFUL_SEA_WORLD_STYLE)
            .collect();
        assert_eq!(harmful, [41, 42, 43, 46, 47]);
    }

    /// Every body of a native style-5 load is decidable; in world 41 the
    /// working factory (Type66) and the Hive (Type67) start in the sea, as
    /// retail's two burning bodies there do.
    #[v2k_test_support::retail_test]
    fn native_loads_decide_every_body_of_a_harmful_sea_world() {
        use crate::{
            entity::{AuthoredWorldConstruction, EntityConstructionResources, EntityManager},
            entity_collision_state::EntityTypeRuntimeMetadata,
            session::GameSession,
            world_fx::WorldFx,
        };
        for world in [41, 42, 43, 46, 47] {
            let mut session = GameSession::init(&v2k_test_support::retail_dir()).unwrap();
            session.load_auxiliary_ovl(3, 1).unwrap();
            session.load_level_by_id(world, 1).unwrap();
            let metadata: Vec<_> = (0..session.cache.global_entity_model_table().len())
                .map(|id| {
                    EntityTypeRuntimeMetadata::from_section12(
                        session.cache.global_entity_type(id).unwrap(),
                    )
                })
                .collect();
            let manager = EntityManager::from_authored_world(
                AuthoredWorldConstruction {
                    level: session.cache.level_desc().unwrap(),
                    logical_world_index: world as i32 - 12,
                    type_metadata: &metadata,
                    player_arrival: None,
                    retail_tick: 0,
                    resources: EntityConstructionResources {
                        terrain: session.cache.terrain(),
                        terrain_objects: session.cache.terrain_objects(),
                        model_extent_raw: Some(&|id| {
                            session.cache.global_model(id).map(|m| m.radius)
                        }),
                    },
                },
                &mut WorldFx::new(),
            )
            .unwrap();
            let sea = harmful_sea_level_raw(
                session.cache.level_desc().unwrap(),
                session.cache.terrain().unwrap(),
            )
            .unwrap();
            let mut burning = Vec::new();
            for entity in manager.iter_all() {
                match body_in_harmful_sea(entity, &session.cache, sea) {
                    Ok(true) => burning.push(entity.entity_type),
                    Ok(false) => {}
                    Err(block) => panic!("world {world} Type{}: {block:?}", entity.entity_type),
                }
            }
            burning.sort();
            if world == 41 {
                assert_eq!(burning, [66, 67]);
            }
        }
    }

    #[test]
    fn a_body_burns_once_its_lower_half_reaches_below_the_sea() {
        let radius = |slot: usize| (slot == 0).then_some(0x100);
        let body = RetailStateWord::exact(0x8800);
        // Y 0x100 less half of 0x100 is 0x80: a sea at 0x80 spares it.
        assert!(!harmful_sea_reaches(body, 0x100, 0x80, radius).unwrap());
        assert!(harmful_sea_reaches(body, 0x100, 0x81, radius).unwrap());
        // Bit 0x2000 reads slot 2.
        assert!(matches!(
            harmful_sea_reaches(RetailStateWord::exact(0xA800), 0x100, 0x81, radius),
            Err(HarmfulSeaBlock::ActiveModelUnresolved)
        ));
        // 0x200000 burns at any height; without 0x8800, or dying, never.
        assert!(harmful_sea_reaches(RetailStateWord::exact(0x20_8800), 0x7FFF, 0, radius).unwrap());
        assert!(!harmful_sea_reaches(RetailStateWord::exact(0x0800), 0, 0x100, radius).unwrap());
        assert!(!harmful_sea_reaches(RetailStateWord::exact(0xC800), 0, 0x100, radius).unwrap());
        assert!(matches!(
            harmful_sea_reaches(RetailStateWord::unknown(), 0, 0, radius),
            Err(HarmfulSeaBlock::StateUnresolved)
        ));
    }
}
