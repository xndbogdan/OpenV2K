//! A normal Hive exit is its real static marker contact, independently of the
//! abort-only `1BEB0 -> 456D10` wreck-interior branch.

use super::*;
use crate::{
    campaign_transition::{CampaignWarpRuntime, RETAIL_CONTROLLER_DEFAULT_ARRIVAL},
    power_up_contact::PlayerCampaignProgress,
    session::GameSession,
    static_contact::{scan_deepest_static_contact, StaticContactQuery, StaticModelContact},
};

fn fixture(level_id: u32) -> (GameSession, EntityManager) {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).expect("canonical retail data required");
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(level_id, 1).unwrap();
    let metadata: Vec<_> = session
        .cache
        .global_entity_model_table()
        .iter()
        .enumerate()
        .map(|(id, _)| {
            EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(id).unwrap())
        })
        .collect();
    let extent = |id| session.cache.global_model(id).map(|model| model.radius);
    let mut fx = WorldFx::new();
    let entities = EntityManager::from_authored_world(
        AuthoredWorldConstruction {
            level: session.cache.level_desc().unwrap(),
            logical_world_index: (level_id - 12) as i32,
            type_metadata: &metadata,
            resources: EntityConstructionResources {
                terrain: session.cache.terrain(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&extent),
            },
            player_arrival: Some(AuthoredPlayerArrival {
                position_raw: RETAIL_CONTROLLER_DEFAULT_ARRIVAL.position_raw,
                heading_raw: RETAIL_CONTROLLER_DEFAULT_ARRIVAL.heading_raw,
            }),
            retail_tick: 0,
        },
        &mut fx,
    )
    .unwrap();
    (session, entities)
}

fn contact_at_marker(
    session: &GameSession,
    player: &Entity,
    origin: [i16; 3],
) -> StaticModelContact {
    let cell = [(origin[0] as u16 >> 8) as u8, (origin[2] as u16 >> 8) as u8];
    let terrain = session.cache.terrain().unwrap();
    let x = usize::from(cell[0]);
    let z = usize::from(cell[1]);
    // Sub-N retains the Hive's Y; it authenticates the selected cell, not the
    // static model's height. 427100 places that model at the average of the
    // four signed terrain corners and the cell's +0x7F X/Z centre.
    let height_sum: i32 = [
        (x, z),
        ((x + 1) & 255, z),
        (x, (z + 1) & 255),
        ((x + 1) & 255, (z + 1) & 255),
    ]
    .into_iter()
    .map(|(x, z)| i32::from(terrain.cell(x, z).unwrap().height as i8) << 5)
    .sum();
    let static_origin = [
        ((u16::from(cell[0]) << 8) | 0x7f) as i16,
        (height_sum / 4) as i16,
        ((u16::from(cell[1]) << 8) | 0x7f) as i16,
    ];
    let model = session
        .cache
        .global_model(player.model_index.unwrap())
        .unwrap();
    let anim_vars = player.presentation_anim_vars(100);
    for dy in (-768i16..=768).step_by(64) {
        for dx in (-512i16..=512).step_by(64) {
            for dz in (-512i16..=512).step_by(64) {
                let position_raw = [
                    static_origin[0].wrapping_add(dx),
                    static_origin[1].wrapping_add(dy),
                    static_origin[2].wrapping_add(dz),
                ];
                let contact = scan_deepest_static_contact(StaticContactQuery {
                    terrain: session.cache.terrain().unwrap(),
                    terrain_objects: session.cache.terrain_objects().unwrap(),
                    model_pool: &session.cache,
                    tick: 100,
                    active_model: model,
                    active_model_to_world_basis: [
                        [1.0, 0.0, 0.0],
                        [0.0, 1.0, 0.0],
                        [0.0, 0.0, 1.0],
                    ],
                    active_anim_vars: &anim_vars,
                    position_raw,
                })
                .unwrap();
                if let Some(contact) = contact.filter(|contact| contact.cell == cell) {
                    return contact;
                }
            }
        }
    }
    panic!("actual player/levexit collision at {cell:?}");
}

#[v2k_test_support::retail_test]
fn native_hive_models_route_through_their_actual_static_marker_subtypes() {
    // Cistern's Hive is beside subtype3, not the distant subtype1 marker.
    for (world, dying_model, subtype, destination) in
        [(13, 343, 1, 14), (16, 585, 1, 37), (30, 1188, 3, 23)]
    {
        let (session, mut entities) = fixture(world);
        let id = entities
            .iter_all()
            .find(|entity| entity.entity_type == 67 && hive_wreck_contact_origin(entity).is_some())
            .unwrap()
            .id;
        let origin = hive_wreck_contact_origin(entities.entity_mut(id).unwrap()).unwrap();
        let mut runtime = CampaignWarpRuntime::default();
        runtime
            .rebuild(
                world,
                session.cache.level_desc().unwrap(),
                session.cache.terrain().unwrap(),
                session.cache.terrain_objects().unwrap(),
            )
            .unwrap();
        let mut progress = PlayerCampaignProgress::new();
        progress.set_current_control_slot(Some((world - 12) as usize));
        assert_eq!(
            runtime.poll_route(
                &mut progress,
                100,
                crate::campaign_transition::CampaignRouteVisit::FirstMatching
            ),
            None
        );

        // Enter the existing source dying initializer after 10C10's health
        // write. No route is manufactured from this model/style transition.
        let hive = entities.entity_mut(id).unwrap();
        hive.collision.health_raw = RetailRuntimeValue::Known(0);
        assert_eq!(hive.apply_hive_dying_initializer(), Some(dying_model));
        assert_eq!(
            hive.collision.active_model_slot(),
            RetailRuntimeValue::Known(3)
        );
        assert_eq!(
            runtime.poll_route(
                &mut progress,
                100,
                crate::campaign_transition::CampaignRouteVisit::FirstMatching
            ),
            None
        );
        let emitter = hive.authored_radial_emitter.as_mut().unwrap();
        assert!(!emitter.wreck_suction_ready());
        emitter.advance_wreck_timer(6_000_001);
        assert!(emitter.wreck_suction_ready());
        assert_eq!(
            runtime.poll_route(
                &mut progress,
                100,
                crate::campaign_transition::CampaignRouteVisit::FirstMatching
            ),
            None,
            "a ready wreck alone does not stamp a normal route"
        );

        let contact = contact_at_marker(&session, entities.player().unwrap(), origin);
        assert_eq!(contact.kind_index, 21 + subtype);
        runtime.observe_player_static_contact(contact, 100);
        let route = runtime
            .poll_route(
                &mut progress,
                100,
                crate::campaign_transition::CampaignRouteVisit::FirstMatching,
            )
            .unwrap();
        assert_eq!(route.marker_subtype, subtype as u8);
        assert_eq!(route.marker_cell, contact.cell);
        assert_eq!(route.destination_level_id, destination);
        assert_eq!(
            runtime.poll_route(
                &mut progress,
                100,
                crate::campaign_transition::CampaignRouteVisit::FirstMatching
            ),
            None
        );
    }
}
