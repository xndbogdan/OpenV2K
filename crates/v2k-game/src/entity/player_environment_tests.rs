//! Authored calm-water and later-world wind controls for the native player.

use super::*;
use crate::common_mover::environment::{
    apply_common_wind_drag_raw, gust_vector_raw, CommonWindDrag, CommonWindDragFrame,
    INITIAL_WIND_PHASE_RAW,
};
use crate::hover::HoverFrameForces;
use crate::session::GameSession;
use crate::vtol::VtolControlFrame;
use std::num::NonZeroU16;
use v2k_formats::terrain::{TerrainCell, GRID_SIZE};

pub(super) fn session(world: u32) -> GameSession {
    let root = v2k_test_support::retail_dir();
    assert!(root.join("PRELOAD.DAT").exists(), "retail corpus required");
    let mut session = GameSession::init(&root).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(world, 1).unwrap();
    session
}

pub(super) fn native(session: &GameSession, world: u32, fx: &mut WorldFx) -> EntityManager {
    static ARRIVALS: std::sync::OnceLock<crate::campaign_transition::CampaignArrivalCatalog> =
        std::sync::OnceLock::new();
    let arrival = ARRIVALS
        .get_or_init(|| {
            crate::campaign_transition::CampaignArrivalCatalog::load(session, 1).unwrap()
        })
        .direct_world_entry(world)
        .unwrap()
        .arrival();
    let metadata: Vec<_> = session
        .cache
        .global_entity_model_table()
        .iter()
        .enumerate()
        .map(|(id, &model_slots)| {
            session
                .cache
                .global_entity_type(id)
                .map(EntityTypeRuntimeMetadata::from_section12)
                .unwrap_or(EntityTypeRuntimeMetadata {
                    model_slots,
                    ..Default::default()
                })
        })
        .collect();
    let extent = |id| session.cache.global_model(id).map(|model| model.radius);
    let mut manager = EntityManager::from_authored_world(
        AuthoredWorldConstruction {
            level: session.cache.level_desc().unwrap(),
            logical_world_index: world as i32 - 12,
            type_metadata: &metadata,
            resources: EntityConstructionResources {
                terrain: session.cache.terrain(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&extent),
            },
            player_arrival: Some(AuthoredPlayerArrival {
                position_raw: arrival.position_raw,
                heading_raw: arrival.heading_raw,
            }),
            retail_tick: 4793,
        },
        fx,
    )
    .unwrap();
    manager.set_hover_physics(
        HoverPhysicsConfig::from_record(session.cache.global_entity_type(46).unwrap()).unwrap(),
    );
    manager
}

fn frame(hover: bool) -> VehicleFrameForces {
    if hover {
        VehicleFrameForces::Hover(HoverFrameForces::default())
    } else {
        VehicleFrameForces::Vtol(VtolControlFrame {
            previous_basis: HoverBasis::from_angle_words(0, 0, 0),
            pitch_before_a690_raw: 0,
            roll_before_a690_raw: 0,
            throttle_q16: 0,
            has_fuel: false,
        })
    }
}

pub(super) fn request(terrain: &TerrainGrid, hover: bool) -> PlayerUpdateRequest<'_> {
    PlayerUpdateRequest {
        elapsed_micros: 32_000,
        terrain,
        sea_level: Some(terrain.sea_level_world_y()),
        retail_tick: 4793,
        water_response_selectors: [6; 8],
        attached_cargo_mass: 0,
        active_model_half_radius_raw: 0,
        active_model_collision_radius_raw: 0,
        frame: frame(hover),
        body_pitch_roll_raw: [1234, -2222],
        vtol_boost: VtolBoost::Inactive,
        vtol_height_policy: VtolHeightPolicy::RetailAttenuation,
    }
}

#[v2k_test_support::retail_test]
fn authored_cistern_uses_static_sea_for_entry_and_both_lift_modes() {
    let session = session(30);
    let level = session.cache.level_desc().unwrap();
    let terrain = session.cache.terrain().unwrap();
    assert_eq!(level.raw_u32(0x84), Some(0));
    assert_eq!(terrain.sea_level_raw(), 3768);
    let sea = terrain.sea_level_raw();
    let [x, z] = (0..GRID_SIZE)
        .flat_map(|x| (0..GRID_SIZE).map(move |z| (x, z)))
        .find_map(|(x, z)| {
            let raw = [(x as i16).wrapping_mul(256), (z as i16).wrapping_mul(256)];
            let ground = i16::from(terrain.cell(x, z).unwrap().height as i8) * 32;
            let wave = v2k_formats::terrain::wave_surface_raw(raw[0], raw[1], 4793, sea, ground);
            (ground < sea - 512 && wave != sea).then_some(raw)
        })
        .expect("Cistern has wet cells whose hypothetical animated surface differs");
    let mut fx = WorldFx::new();
    let mut manager = native(&session, 30, &mut fx);
    let player = manager.player_mut().unwrap();
    player.set_motion_raw([x, sea, z], [0; 3]);
    player
        .collision
        .state_flags_at_0x08
        .overwrite(SURFACE_STATE_MASK, FULLY_ABOVE_SURFACE_STATE_BIT);
    let outcome = manager.update(PlayerUpdateRequest {
        elapsed_micros: 0,
        ..request(terrain, false)
    });
    assert_eq!(outcome.water_entry.unwrap().surface_y_raw, sea);
    assert_eq!(
        manager
            .player()
            .unwrap()
            .collision
            .state_flags_at_0x08
            .masked(SURFACE_STATE_MASK),
        RetailRuntimeValue::Known(0)
    );

    for hover in [false, true] {
        let mut values = Vec::new();
        for tick in [4793, 4817] {
            let mut manager = native(&session, 30, &mut fx);
            manager
                .player_mut()
                .unwrap()
                .set_motion_raw([x, sea + 100, z], [300, 0, -500]);
            let (outcome, _) = manager.update_with_player_surface_contacts(
                PlayerUpdateRequest {
                    retail_tick: tick,
                    ..request(terrain, hover)
                },
                false,
                |_, phase| match phase {
                    PlayerSurfaceContactPhase::Terrain => PlayerSurfaceContactDispatch::Terrain(()),
                    PlayerSurfaceContactPhase::Water { .. } => {
                        PlayerSurfaceContactDispatch::WaterComplete
                    }
                },
            );
            if let Some(diagnostics) = outcome.vtol_diagnostics {
                let expected = VTOL_RIDE_PROBE_OFFSETS_RAW.map(|[dx, dz]| {
                    i32::from(
                        terrain_height_raw(terrain, x.wrapping_add(dx), z.wrapping_add(dz))
                            .max(sea),
                    )
                });
                assert_eq!(diagnostics.ride_surface_probes_raw, expected);
            }
            values.push((
                manager.player().unwrap().position_raw(),
                manager.player().unwrap().velocity_raw(),
                outcome.body_angle_words_after_environment,
            ));
        }
        assert_eq!(
            values[0], values[1],
            "calm-water lift must not animate: hover={hover}"
        );
    }
}

#[v2k_test_support::retail_test]
fn all_authored_wind_vectors_reach_both_native_player_modes_once() {
    // Controlled flat water isolates the environment response from each
    // world's terrain, retaining its actual native player and descriptor.
    let terrain = TerrainGrid {
        header: [0; 5],
        cells: vec![
            TerrainCell {
                height: (-128i8) as u8,
                attribute: 0,
                terrain_type: 0
            };
            GRID_SIZE * GRID_SIZE
        ],
    };
    for (world, mode, vector) in [
        (22, 1, [1000, 0, 0]),
        (32, 1, [-300, 0, -300]),
        (33, 2, [0, 0, 20000]),
        (45, 1, [0, 0, 32000]),
        (46, 1, [1000, 1000, 1000]),
        (47, 2, [2000, 0, 0]),
    ] {
        let session = session(world);
        for hover in [false, true] {
            let mut fx = WorldFx::new();
            let mut manager = native(&session, world, &mut fx);
            let environment = manager.common_environment_physics();
            assert_eq!(
                (
                    environment.runtime_wind_mode,
                    environment.configured_wind_raw
                ),
                (mode, vector)
            );
            manager.advance_environment_frame(&mut fx, 32_000);
            let environment = manager.common_environment_physics();
            assert_eq!(
                environment.current_wind_raw,
                if mode == 2 {
                    gust_vector_raw(vector, INITIAL_WIND_PHASE_RAW + 32_000)
                } else {
                    vector
                }
            );
            let wind = CommonWindDrag::from_level(session.cache.level_desc().unwrap(), environment)
                .unwrap();
            let position = [
                128,
                if environment.wind_above_sea {
                    1024
                } else {
                    -1024
                },
                128,
            ];
            manager
                .player_mut()
                .unwrap()
                .set_motion_raw(position, [300, -40, -500]);
            let angles = [manager.player().unwrap().heading_raw() as i16, 1234, -2222];
            let basis = Type9BodyBasis::from_angle_words(angles[0], angles[1], angles[2]);
            let mass = NonZeroU16::new(manager.player().unwrap().mass_raw).unwrap();
            // Isolated transaction fork supplies the exact same C/B/A or
            // VTOL + gravity prefix, with only EC60 disabled in the control.
            let mut control = manager.fork_for_main_base_abort_transaction();
            control.environment_physics.runtime_wind_mode = 0;
            control.environment_physics.drag_strength = 0;
            control.update_with_player_surface_contacts(
                request(&terrain, hover),
                false,
                |_, phase| match phase {
                    PlayerSurfaceContactPhase::Terrain => PlayerSurfaceContactDispatch::Terrain(()),
                    PlayerSurfaceContactPhase::Water { .. } => {
                        PlayerSurfaceContactDispatch::WaterComplete
                    }
                },
            );
            let mut expected_velocity = control.player().unwrap().velocity_raw();
            let mut expected_angles = angles;
            apply_common_wind_drag_raw(
                &mut expected_velocity,
                &mut expected_angles,
                wind,
                CommonWindDragFrame {
                    terrain: &terrain,
                    position_raw: position,
                    basis,
                    callback_mass_raw: mass,
                    elapsed_micros: 32_000,
                },
            );
            assert_ne!(
                expected_velocity,
                control.player().unwrap().velocity_raw(),
                "world{world} hover={hover} must exercise EC60"
            );
            assert_ne!(
                expected_angles, angles,
                "world{world} must exercise angular wind"
            );
            let (outcome, _) = manager.update_with_player_surface_contacts(
                request(&terrain, hover),
                false,
                |_, phase| match phase {
                    PlayerSurfaceContactPhase::Terrain => PlayerSurfaceContactDispatch::Terrain(()),
                    PlayerSurfaceContactPhase::Water { .. } => {
                        PlayerSurfaceContactDispatch::WaterComplete
                    }
                },
            );
            assert_eq!(
                manager.player().unwrap().velocity_raw(),
                expected_velocity,
                "world{world} hover={hover}"
            );
            assert_eq!(
                outcome.body_angle_words_after_environment,
                Some([expected_angles[1], expected_angles[2]])
            );
            assert_eq!(
                manager.player().unwrap().rotation_heading_pitch_roll_raw(),
                expected_angles
            );
            assert_eq!(
                manager.player().unwrap().physical_body_basis_q31(),
                RetailRuntimeValue::Known(basis),
                "EC60 must not rebuild F70 again"
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn gust_phase_survives_world_changes_and_effect_clear_without_rng_draws() {
    let mut fx = WorldFx::new();
    let session33 = session(33);
    let mut first = native(&session33, 33, &mut fx);
    first.advance_environment_frame(&mut fx, 32_000);
    let session46 = session(46);
    let mut steady = native(&session46, 46, &mut fx);
    steady.advance_environment_frame(&mut fx, 90_000); // Mode1 never advances C8.
    fx.clear();
    let session47 = session(47);
    let mut next = native(&session47, 47, &mut fx);
    assert_eq!(
        next.common_environment_physics().current_wind_raw,
        [2000, 0, 0],
        "44EB40 initially copies the authored vector"
    );
    let mut rng_control = fx.fork_for_main_base_abort_transaction();
    next.advance_environment_frame(&mut fx, 40_000);
    assert_eq!(
        next.common_environment_physics().current_wind_raw,
        gust_vector_raw([2000, 0, 0], INITIAL_WIND_PHASE_RAW + 72_000)
    );
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        rng_control.next_shared_retail_random_u16()
    );
    let mut fork = fx.fork_for_main_base_abort_transaction();
    assert_eq!(fork.advance_wind_phase(17), fx.advance_wind_phase(17));
}
