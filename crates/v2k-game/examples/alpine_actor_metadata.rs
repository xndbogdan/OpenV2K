//! Reproduce the Alpine actor profile and authored-birth census with canonical parsers.
//!
//! From the repository root: cargo run --release -p v2k-game --example alpine_actor_metadata
//! An optional first argument overrides the retail data directory.

use std::{collections::BTreeMap, path::PathBuf};

use v2k_game::{entity_collision_state::EntityTypeRuntimeMetadata, session::GameSession};

fn main() {
    let data = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."));
    let mut session = GameSession::init(&data).expect("retail data is required");
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(17, 1).unwrap();

    for id in [30, 40, 56] {
        let record = session.cache.global_entity_type(id).unwrap();
        println!(
            "ALPINE_TYPE {id} tag={} subs={:?} models={:?} rule={} alternate={}",
            record.type_tag,
            record.active_subs,
            record.model_ids,
            record.behavior_rule_ref,
            record.alternate_behavior_class_ref,
        );
        println!(
            "METADATA {id} {:#?}",
            EntityTypeRuntimeMetadata::from_section12(record)
        );
        let model_id = record.model_ids[0];
        let model = session.cache.global_model(usize::from(model_id)).unwrap();
        println!(
            "MODEL type={id} id={model_id} name={:?} extent={} collision={} slots={}",
            model.name, model.radius, model.collision_radius_raw, model.slot_count,
        );
    }

    let mut variants: BTreeMap<usize, Vec<(EntityTypeRuntimeMetadata, Vec<u32>)>> = BTreeMap::new();
    for world in 13..=50 {
        session.load_level_by_id(world, 1).unwrap();
        let level = session.cache.level_desc().unwrap();
        for id in [30, 40, 56] {
            let births: Vec<_> = level
                .entities
                .iter()
                .filter(|spawn| spawn.entity_type as usize == id)
                .map(|spawn| (spawn.index, spawn.position_raw(), spawn.param))
                .collect();
            if !births.is_empty() {
                println!(
                    "BIRTHS world={world} name={:?} type={id} {births:?}",
                    level.name
                );
            }
            let Some(record) = session.cache.global_entity_type(id) else {
                continue;
            };
            let metadata = EntityTypeRuntimeMetadata::from_section12(record);
            let profiles = variants.entry(id).or_default();
            if let Some((_, worlds)) = profiles.iter_mut().find(|(old, _)| *old == metadata) {
                worlds.push(world);
            } else {
                profiles.push((metadata, vec![world]));
            }
        }
    }
    for (id, profiles) in variants {
        println!("VARIANTS type={id} count={}", profiles.len());
        for (_, worlds) in profiles {
            println!("VARIANT_WORLDS type={id} {worlds:?}");
        }
    }
}
