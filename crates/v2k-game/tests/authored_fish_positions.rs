//! Fish construction preserves authored altitude independently of movement.

use v2k_game::entity::{AuthoredWorldConstruction, EntityConstructionResources, EntityManager};
use v2k_game::entity_collision_state::EntityTypeRuntimeMetadata;
use v2k_game::session::GameSession;
use v2k_game::world_fx::WorldFx;

#[v2k_test_support::retail_test]
fn aquatic_fish_keep_authored_y_including_zero_and_above_static_sea() {
    let data = v2k_test_support::retail_dir();
    assert!(data.join("PRELOAD.DAT").is_file(), "retail corpus required");
    let mut session = GameSession::init(&data).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();

    for (level_id, expected_count) in [(23, 29), (30, 17)] {
        session.load_level_by_id(level_id, 1).unwrap();
        let metadata: Vec<_> = session
            .cache
            .global_entity_model_table()
            .iter()
            .enumerate()
            .map(|(id, _)| {
                EntityTypeRuntimeMetadata::from_section12(
                    session.cache.global_entity_type(id).unwrap(),
                )
            })
            .collect();
        let level = session.cache.level_desc().unwrap();
        let terrain = session.cache.terrain().unwrap();
        let extent = |id| session.cache.global_model(id).map(|model| model.radius);
        let manager = EntityManager::from_authored_world(
            AuthoredWorldConstruction {
                level,
                logical_world_index: level.world_style as i32 - 1,
                type_metadata: &metadata,
                resources: EntityConstructionResources {
                    terrain: Some(terrain),
                    terrain_objects: session.cache.terrain_objects(),
                    model_extent_raw: Some(&extent),
                },
                player_arrival: None,
                retail_tick: 0,
            },
            &mut WorldFx::new(),
        )
        .unwrap();
        let fish: Vec<_> = manager
            .iter_all()
            .filter(|entity| matches!(entity.entity_type, 22 | 24 | 124))
            .collect();
        assert_eq!(fish.len(), expected_count);
        for entity in &fish {
            let spawn = &level.entities[entity.authored_spawn_index.unwrap()];
            assert_eq!(
                metadata[entity.entity_type as usize]
                    .initializer
                    .as_ref()
                    .unwrap()
                    .initializer_state_flags_raw,
                0x200c,
            );
            assert_eq!(
                entity.position_raw(),
                spawn.position_raw(),
                "level {level_id} spawn {} has no D4A0 terrain-snap policy",
                spawn.index,
            );
        }

        if level_id == 23 {
            let zero = fish
                .iter()
                .find(|entity| entity.authored_spawn_index == Some(21))
                .unwrap();
            assert_eq!(zero.position_raw(), [28672, 0, 31232]);
            assert_eq!(terrain.bilinear_height_raw(28672, 31232), 1184);
        } else {
            // The accepted fish-movement-Reef trace loaded gameplay overlay30.
            // These two coarse pinkfish retain their authored Y3840 in retail;
            // the static sea is3768. An underwater clamp would corrupt them.
            assert_eq!(terrain.sea_level_raw(), 3768);
            for (index, position) in [(25, [2048, 3840, -11520]), (30, [-2304, 3840, 23552])] {
                let entity = fish
                    .iter()
                    .find(|entity| entity.authored_spawn_index == Some(index))
                    .unwrap();
                assert_eq!(entity.position_raw(), position);
            }
        }
    }
}
