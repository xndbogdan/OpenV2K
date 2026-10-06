use v2k_formats::models::AnimVars;
use v2k_game::campaign_transition::{
    authored_campaign_routes, collect_authored_warp_markers, CampaignArrivalCatalog,
    CampaignWarpRuntime, DirectWorldEntry,
};
use v2k_game::power_up_contact::PlayerCampaignProgress;
use v2k_game::session::GameSession;
use v2k_game::static_contact::{
    scan_deepest_static_contact, StaticContactQuery, StaticModelContact,
};

fn session() -> GameSession {
    let root = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&root).expect("retail PRELOAD.DAT is required");
    session.load_auxiliary_ovl(3, 1).unwrap();
    session
}

fn runtime(session: &GameSession, level_id: u32) -> CampaignWarpRuntime {
    let mut runtime = CampaignWarpRuntime::default();
    runtime
        .rebuild(
            level_id,
            session.cache.level_desc().unwrap(),
            session.cache.terrain().unwrap(),
            session.cache.terrain_objects().unwrap(),
        )
        .unwrap();
    runtime
}

fn stamp_contact(cell: [u8; 2], kind_index: u32, terrain_type: u8) -> StaticModelContact {
    StaticModelContact {
        cell,
        attribute: 1,
        terrain_type,
        model_id: 39,
        kind_index,
        normal_q12: [0, 4096, 0],
        penetration_raw: 1,
        response_raw: 1536,
    }
}

#[v2k_test_support::retail_test]
fn all_tier_one_campaign_records_select_authored_arrivals_and_own_pair_columns() {
    let mut session = session();
    let counts = [
        2, 2, 4, 3, 2, 2, 2, 5, 2, 2, 5, 2, 0, 3, 0, 0, 0, 4, 3, 3, 3, 2, 5, 3, 4, 2, 3, 2, 0, 3,
        3, 0, 1, 4, 1, 2,
    ];
    let mut total = 0;
    for level_id in 13..=48 {
        session.load_level_by_id(level_id, 1).unwrap();
        let level = session.cache.level_desc().unwrap();
        let routes = authored_campaign_routes(
            level_id,
            level,
            session.cache.terrain().unwrap(),
            session.cache.terrain_objects().unwrap(),
        )
        .unwrap();
        assert_eq!(
            routes.len(),
            counts[(level_id - 13) as usize],
            "OVL {level_id}"
        );
        total += routes.len();
        for entry in routes {
            let mut runtime = runtime(&session, level_id);
            let mut progress = PlayerCampaignProgress::new();
            progress.set_current_control_slot(Some((level_id - 12) as usize));
            // This control tests the exact selector using the real authored
            // record; the accepted gates below additionally use geometry.
            let bits = entry
                .route
                .marker_args
                .map(|value| if value == 2 { 0 } else { value as u8 });
            runtime.observe_player_static_contact(
                stamp_contact(
                    entry.route.marker_cell,
                    entry.route.marker_kind,
                    (bits[0] << 3) | (bits[1] << 4),
                ),
                100,
            );
            let selected = runtime
                .poll_route(
                    &mut progress,
                    100,
                    v2k_game::campaign_transition::CampaignRouteVisit::FirstMatching,
                )
                .expect("authored contact route");
            assert_eq!(
                selected, entry.route,
                "OVL {level_id} record {}",
                entry.record_index
            );
            assert_eq!(
                selected.arrival.position_raw,
                level.campaign_records[entry.record_index]
                    .marker_transition()
                    .unwrap()
                    .arrival_raw
            );
            assert_eq!(
                selected.destination_level_id,
                selected.destination_logical_level + 12
            );
            assert!(progress.control_slot_pair_neighbor(
                (level_id - 12) as usize,
                entry.record_index as u32 + 1
            ));
            assert_eq!(
                runtime.poll_route(
                    &mut progress,
                    100,
                    v2k_game::campaign_transition::CampaignRouteVisit::FirstMatching
                ),
                None
            );
        }
    }
    assert_eq!(total, 84);
}

#[v2k_test_support::retail_test]
fn cistern_high_flag_bytes_do_not_disable_its_authored_castle_exit() {
    let mut session = session();
    session.load_level_by_id(30, 1).unwrap();
    let level = session.cache.level_desc().unwrap();
    assert_eq!(level.campaign_records[0].flags(), 0x00aa0d10);
    let routes = authored_campaign_routes(
        30,
        level,
        session.cache.terrain().unwrap(),
        session.cache.terrain_objects().unwrap(),
    )
    .unwrap();
    assert_eq!(routes[0].route.destination_level_id, 15);
    assert_eq!(routes[0].route.arrival.position_raw, [0, -1024, 0]);
}

fn actual_marker_contact(session: &GameSession, subtype: u8) -> StaticModelContact {
    let terrain = session.cache.terrain().unwrap();
    let objects = session.cache.terrain_objects().unwrap();
    let marker = collect_authored_warp_markers(terrain, objects)
        .unwrap()
        .into_iter()
        .find(|marker| marker.subtype == subtype)
        .unwrap();
    let player_model = session
        .cache
        .global_model(41)
        .expect("actual player4 model");
    let anim = AnimVars::default();
    for dy in (-768i16..=768).step_by(64) {
        for dx in (-512i16..=512).step_by(64) {
            for dz in (-512i16..=512).step_by(64) {
                let position_raw = [
                    marker.position_raw[0].wrapping_add(dx),
                    marker.position_raw[1].wrapping_add(dy),
                    marker.position_raw[2].wrapping_add(dz),
                ];
                let contact = scan_deepest_static_contact(StaticContactQuery {
                    terrain,
                    terrain_objects: objects,
                    model_pool: &session.cache,
                    tick: 100,
                    active_model: player_model,
                    active_model_to_world_basis: [
                        [1.0, 0.0, 0.0],
                        [0.0, 1.0, 0.0],
                        [0.0, 0.0, 1.0],
                    ],
                    active_anim_vars: &anim,
                    position_raw,
                })
                .unwrap();
                if let Some(contact) = contact.filter(|contact| contact.cell == marker.cell) {
                    return contact;
                }
            }
        }
    }
    panic!("actual player4/levexit geometry must contact subtype {subtype}");
}

#[v2k_test_support::retail_test]
fn real_peasant_and_cistern_model_contacts_commit_both_directions_once() {
    let mut session = session();
    for (level_id, subtype, destination, arrival, column) in [
        (13, 2, 30, [7424, 5120, 9728], 1),
        (30, 4, 13, [-27392, 0, -5120], 4),
    ] {
        session.load_level_by_id(level_id, 1).unwrap();
        let contact = actual_marker_contact(&session, subtype);
        let mut runtime = runtime(&session, level_id);
        let mut progress = PlayerCampaignProgress::new();
        progress.set_current_control_slot(Some((level_id - 12) as usize));
        runtime.observe_player_static_contact(contact, 100);
        let route = runtime
            .poll_route(
                &mut progress,
                101,
                v2k_game::campaign_transition::CampaignRouteVisit::FirstMatching,
            )
            .unwrap();
        assert_eq!(route.destination_level_id, destination);
        assert_eq!(route.arrival.position_raw, arrival);
        assert_eq!(route.arrival.heading_raw, 0x4000);
        assert!(progress.control_slot_pair_neighbor((level_id - 12) as usize, column));
        assert_eq!(
            runtime.poll_route(
                &mut progress,
                101,
                v2k_game::campaign_transition::CampaignRouteVisit::FirstMatching
            ),
            None
        );
    }
}

#[v2k_test_support::retail_test]
fn incoming_arrival_catalog_covers_campaign_worlds_and_distinguishes_arenas() {
    let session = session();
    let catalog = CampaignArrivalCatalog::load(&session, 1).unwrap();
    assert_eq!(catalog.entries().len(), 84);
    let mut defaults = Vec::new();
    for level_id in 13..=48 {
        match catalog.direct_world_entry(level_id).unwrap() {
            DirectWorldEntry::AuthoredRoute(entry) => {
                assert_eq!(entry.destination_level_id, level_id);
                assert_eq!(entry.arrival.heading_raw, 0x4000);
            }
            DirectWorldEntry::RetailControllerDefault(arrival) => {
                defaults.push(level_id);
                assert_eq!(arrival.position_raw, [19712, -500, 14848]);
            }
        }
    }
    assert_eq!(defaults, [25, 27, 28, 29, 41, 44]);
    assert_eq!(
        catalog
            .direct_world_entry(30)
            .unwrap()
            .arrival()
            .position_raw,
        [7424, 5120, 9728]
    );
    assert_eq!(
        catalog
            .direct_world_entry(14)
            .unwrap()
            .arrival()
            .position_raw,
        [17408, 2560, -32512]
    );
    assert!(catalog.direct_world_entry(12).is_none());
    assert!(catalog.direct_world_entry(49).is_none());
    assert!(catalog.direct_world_entry(50).is_none());
}

#[v2k_test_support::retail_test]
fn cistern_roundtrip_waits_for_map_continue_then_restores_native_player_first() {
    use v2k_game::entity::{
        AuthoredWorldConstruction, EntityConstructionResources, EntityManager, NativeSaveWorldState,
    };
    use v2k_game::entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue};
    use v2k_game::player_hull::{HullDamageProfile, PlayerHull};
    use v2k_game::save::{NativeSaveRestore, NativeSaveSnapshot, SavedPlayerState};
    use v2k_game::world_complete_results::WorldCompleteResultsRuntime;
    use v2k_game::world_fx::WorldFx;

    let mut session = session();
    session.load_level_by_id(13, 1).unwrap();
    let metadata = |session: &GameSession| {
        session
            .cache
            .global_entity_model_table()
            .iter()
            .enumerate()
            .map(|(kind, _)| {
                EntityTypeRuntimeMetadata::from_section12(
                    session.cache.global_entity_type(kind).unwrap(),
                )
            })
            .collect::<Vec<_>>()
    };
    let initial_metadata = metadata(&session);
    let mut fx = WorldFx::new();
    let extent = |id| session.cache.global_model(id).map(|model| model.radius);
    let mut entities = EntityManager::from_authored_world(
        AuthoredWorldConstruction {
            level: session.cache.level_desc().unwrap(),
            logical_world_index: 1,
            type_metadata: &initial_metadata,
            resources: EntityConstructionResources {
                terrain: session.cache.terrain(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&extent),
            },
            player_arrival: None,
            retail_tick: 0,
        },
        &mut fx,
    )
    .unwrap();
    let mut craft = v2k_game::player::PlayerCraft::new();
    craft.fuel_raw = 99119;
    let mut hull = PlayerHull::new(HullDamageProfile::from_type_record(
        session.cache.global_entity_type(46).unwrap(),
    ));
    hull.pre_health_damage_buffer_raw = 80833;
    let inventory = v2k_game::weapon_inventory::WeaponInventory::new();
    let capabilities = v2k_game::weapon_inventory::PlayerCapabilities::default();
    let cargo = v2k_game::entity::CampaignCargoControllerState::from_packed(1, []);
    let mut progress = PlayerCampaignProgress::new();
    progress.set_current_control_slot(Some(1));
    let mut map = WorldCompleteResultsRuntime::default();
    let mut baseline = [0; 0x248];

    for (source, subtype, destination, arrival) in [
        (13, 2, 30, [7424, 5120, 9728]),
        (30, 4, 13, [-27392, 0, -5120]),
    ] {
        let mut runtime = runtime(&session, source);
        runtime.observe_player_static_contact(actual_marker_contact(&session, subtype), 100);
        let route = runtime
            .poll_route(
                &mut progress,
                100,
                v2k_game::campaign_transition::CampaignRouteVisit::FirstMatching,
            )
            .unwrap();
        let old_player = entities.player().unwrap();
        let old_pose = old_player.position_raw();
        assert_eq!(map.continue_progress_map(), None);
        let transition = v2k_game::campaign_transition::CampaignTransition::AuthoredMarker(route);
        map.open_progress_map(transition);
        progress.set_current_control_slot(Some((destination - 12) as usize));
        assert!(map.is_progress_map_active());
        assert_eq!(map.progress_map_route(), Some(transition));
        assert_eq!(
            entities.player().unwrap().position_raw(),
            old_pose,
            "map owns no destination player yet"
        );
        let route = map.continue_progress_map().unwrap();
        assert_eq!(map.continue_progress_map(), None);
        let checkpoint = NativeSaveSnapshot {
            logical_level_id: route.destination_logical_level(),
            player: SavedPlayerState {
                position_raw: old_pose,
                velocity_raw: old_player.velocity_raw(),
                heading_raw: old_player.heading_raw(),
                pitch_raw: 0,
                roll_raw: 0,
                health_raw: hull.health_raw,
            },
            craft: &craft,
            hull: &hull,
            inventory: &inventory,
            capabilities: &capabilities,
            cargo: &cargo,
            campaign: &progress,
        }
        .encode_campaign_arrival(&baseline, route.arrival(), "campaign roundtrip")
        .unwrap();
        let restore = NativeSaveRestore::decode(&checkpoint).unwrap();
        baseline = checkpoint.state_payload;
        session.load_level_by_id(destination, 1).unwrap();
        let metadata = metadata(&session);
        let mut terrain = session.cache.level_terrain().unwrap().clone();
        let extent = |id| session.cache.global_model(id).map(|model| model.radius);
        let mut static_damage = v2k_game::static_damage::StaticDamageScheduler::new();
        entities = EntityManager::from_authored_world(
            AuthoredWorldConstruction {
                level: session.cache.level_desc().unwrap(),
                logical_world_index: (destination - 12) as i32,
                type_metadata: &metadata,
                resources: EntityConstructionResources {
                    terrain: session.cache.terrain(),
                    terrain_objects: session.cache.terrain_objects(),
                    model_extent_raw: Some(&extent),
                },
                player_arrival: None,
                retail_tick: 0,
            }
            .with_native_save(NativeSaveWorldState {
                restore: &restore,
                terrain: &mut terrain,
                resources: &session.cache,
                static_damage: &mut static_damage,
            }),
            &mut fx,
        )
        .unwrap();
        let player = entities.player().unwrap();
        assert_eq!(player.position_raw(), arrival);
        assert_eq!(player.velocity_raw(), [0; 3]);
        assert_eq!(player.rotation_heading_pitch_roll_raw(), [0x4000, 0, 0]);
        assert_eq!(
            player.collision.health_raw,
            RetailRuntimeValue::Known(40000)
        );
        assert_eq!(
            player.collision.pre_health_damage_buffer_raw,
            RetailRuntimeValue::Known(80833)
        );
        assert_eq!(
            entities
                .iter_all()
                .filter(|entity| entity.entity_type == 46)
                .count(),
            1
        );
        assert_eq!(entities.retail_live_order_ids().next(), Some(player.id));
        restore.restore_craft(&mut craft);
        assert_eq!(craft.fuel_raw, 99119);
        progress = restore.campaign;
    }
}
