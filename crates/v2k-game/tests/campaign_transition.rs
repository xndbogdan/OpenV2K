//! Real-data acceptance coverage for the recovered secret-world warp route.

use v2k_formats::anim_sound::SoundPool;
use v2k_game::campaign_transition::{
    authored_campaign_routes, collect_authored_warp_markers, CISTERN_LEVEL_ID, PEASANT_LEVEL_ID,
};
use v2k_game::entity::{AuthoredWorldConstruction, EntityConstructionResources, EntityManager};
use v2k_game::entity_collision_state::EntityTypeRuntimeMetadata;
use v2k_game::session::GameSession;
use v2k_game::world_fx::WorldFx;

struct RouteMarker {
    marker: v2k_game::campaign_transition::AuthoredWarpMarker,
    route: v2k_game::campaign_transition::CampaignWarpRoute,
}

fn route_gate(
    session: &mut GameSession,
    level_id: u32,
    captured_cell: [u8; 2],
) -> Option<RouteMarker> {
    session
        .load_level_by_id(level_id, 1)
        .expect("retail fixture must load");
    let marker =
        collect_authored_warp_markers(session.cache.terrain()?, session.cache.terrain_objects()?)
            .expect("retail fixture must load")
            .into_iter()
            .find(|marker| marker.cell == captured_cell)?;
    let routes = authored_campaign_routes(
        level_id,
        session.cache.level_desc()?,
        session.cache.terrain()?,
        session.cache.terrain_objects()?,
    )
    .expect("retail fixture must load");
    let route = routes
        .into_iter()
        .find(|entry| entry.route.marker_subtype == marker.subtype)?
        .route;
    Some(RouteMarker { marker, route })
}

fn type_metadata(session: &GameSession) -> Vec<EntityTypeRuntimeMetadata> {
    let type_models = session.cache.global_entity_model_table();
    type_models
        .iter()
        .copied()
        .enumerate()
        .map(|(entity_type, model_slots)| {
            session
                .cache
                .global_entity_type(entity_type)
                .map(EntityTypeRuntimeMetadata::from_section12)
                .unwrap_or(EntityTypeRuntimeMetadata {
                    model_slots,
                    ..EntityTypeRuntimeMetadata::default()
                })
        })
        .collect()
}

fn destination_entities(
    session: &GameSession,
    position_raw: [i16; 3],
    heading_raw: u16,
) -> EntityManager {
    let type_metadata = type_metadata(session);
    let mut entities = EntityManager::from_level_with_type_metadata(
        session.cache.level_desc().expect("destination descriptor"),
        &type_metadata,
        session.cache.terrain(),
    );
    assert!(
        entities.player().is_none(),
        "Cistern should require the campaign player allocation"
    );
    entities.place_or_spawn_player_at_campaign_arrival(
        type_metadata.get(46),
        position_raw,
        heading_raw,
        session.cache.terrain(),
    );
    entities
}

#[v2k_test_support::retail_test]
fn high_resolution_world_data_contains_both_captured_secret_route_markers() {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).expect("init session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load high-resolution common resources");

    let outward = route_gate(&mut session, PEASANT_LEVEL_ID, [149, 239])
        .expect("captured Peasant secret gate");
    assert_eq!(outward.marker.cell, [149, 239]);
    assert_eq!(outward.marker.kind_index, 0x17);
    assert_eq!(outward.marker.subtype, 2);
    assert_eq!(outward.marker.model_id, 39);
    assert_eq!(outward.marker.position_raw, [-27264, -2080, -4224]);
    assert_eq!(outward.route.destination_level_id, CISTERN_LEVEL_ID);
    assert_eq!(outward.route.destination_logical_level, 18);
    assert_eq!(outward.route.arrival.position_raw, [7424, 5120, 9728]);
    assert_eq!(outward.route.arrival.heading_raw, 0x4000);
    assert_eq!(
        session
            .cache
            .global_model(usize::from(outward.marker.model_id))
            .and_then(|model| model.name.as_deref()),
        Some("levexit")
    );

    // Peasant's second transition record and subtype-1 marker are the exact
    // ordinary Level-2 route seen in the accepted campaign capture. They are
    // decoded here but must remain inert until the hostile/hive gate opens.
    let ordinary = session
        .cache
        .level_desc()
        .expect("Peasant level descriptor")
        .campaign_records
        .iter()
        .filter_map(|record| record.marker_transition())
        .find(|route| route.marker_subtype == 1)
        .expect("Peasant ordinary Level-2 route");
    assert_eq!(ordinary.destination_global_level(), Some(14));
    assert_eq!(ordinary.arrival_raw, [17408, 2560, -32512]);
    let ordinary_marker = collect_authored_warp_markers(
        session.cache.terrain().expect("Peasant terrain"),
        session
            .cache
            .terrain_objects()
            .expect("Peasant object table"),
    )
    .expect("Peasant campaign markers")
    .into_iter()
    .find(|marker| marker.subtype == 1)
    .expect("Peasant ordinary marker");
    assert_eq!(ordinary_marker.cell, [187, 129]);
    assert_eq!(ordinary_marker.kind_index, 0x16);
    let dead_hive_route = authored_campaign_routes(
        PEASANT_LEVEL_ID,
        session
            .cache
            .level_desc()
            .expect("Peasant level descriptor"),
        session.cache.terrain().expect("Peasant terrain"),
        session
            .cache
            .terrain_objects()
            .expect("Peasant object table"),
    )
    .expect("Peasant authored routes")
    .into_iter()
    .find(|entry| entry.route.marker_subtype == 1)
    .expect("Peasant ordinary dead-hive route")
    .route;
    assert_eq!(dead_hive_route.marker_cell, [187, 129]);
    assert_eq!(dead_hive_route.destination_level_id, 14);
    assert_eq!(dead_hive_route.arrival.position_raw, [17408, 2560, -32512]);
    assert_eq!(dead_hive_route.arrival.heading_raw, 0x4000);

    let returning =
        route_gate(&mut session, CISTERN_LEVEL_ID, [21, 39]).expect("captured Cistern return gate");
    let cistern_level = session.cache.level_desc().expect("Cistern descriptor");
    assert_eq!(
        &cistern_level.ground_response_selectors()[..5],
        &[12, 12, 11, 11, 12]
    );
    assert_eq!(cistern_level.water_response_selectors(), [6; 8]);
    assert_eq!(returning.marker.cell, [21, 39]);
    assert_eq!(returning.marker.kind_index, 0x19);
    assert_eq!(returning.marker.subtype, 4);
    assert_eq!(returning.marker.model_id, 39);
    assert_eq!(returning.marker.position_raw, [5504, 2912, 10112]);
    assert_eq!(returning.route.destination_level_id, PEASANT_LEVEL_ID);
    assert_eq!(returning.route.destination_logical_level, 1);
    assert_eq!(returning.route.arrival.position_raw, [-27392, 0, -5120]);
    assert_eq!(returning.route.arrival.heading_raw, 0x4000);

    // The route catalog chooses its first X-major static marker as a subtype
    // representative. 42EFB0 instead removes earlier nearby Type111 helpers;
    // the accepted capture observes the retained last helper at [21,39].
    assert_eq!(returning.route.marker_cell, [20, 38]);
    let metadata = type_metadata(&session);
    let extent = |id| session.cache.global_model(id).map(|model| model.radius);
    let mut native = EntityManager::from_authored_world(
        AuthoredWorldConstruction {
            level: cistern_level,
            logical_world_index: 18,
            type_metadata: &metadata,
            resources: EntityConstructionResources {
                terrain: session.cache.terrain(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&extent),
            },
            player_arrival: None,
            retail_tick: 0,
        },
        &mut WorldFx::new(),
    )
    .unwrap();
    let captured_helper = native
        .iter_all()
        .find(|entity| {
            entity.entity_type == 111 && entity.position_raw() == returning.marker.position_raw
        })
        .expect("captured Type111 is one actual constructor attempt")
        .id;
    assert!(!native
        .pending_actor_deferred_destroy_ids()
        .contains(&captured_helper));
    assert!(!native.cleanup_pending_actor_deferred_destroys().is_empty());
    assert!(native.iter_all().any(|entity| entity.id == captured_helper));

    // Cistern has no special first-world player constructor. The campaign
    // transaction must allocate a fresh persistent craft there rather than
    // merely trying to reposition a nonexistent entity.
    let destination = destination_entities(
        &session,
        outward.route.arrival.position_raw,
        outward.route.arrival.heading_raw,
    );
    let player = destination
        .player()
        .expect("campaign-spawned Cistern player");
    assert_eq!(player.position_raw(), [7424, 5120, 9728]);
    assert_eq!(player.velocity_raw(), [0; 3]);

    let sound_tables = session.cache.global_sound_tables();
    let sound_pool = SoundPool::from_tables(&sound_tables);
    let gate_loop = sound_pool.resolve(100).expect("secret gate loop alias");
    assert_eq!(gate_loop.resolution_chain, vec![100, 38]);
    assert_eq!(gate_loop.pcm_global_id, 38);
}
