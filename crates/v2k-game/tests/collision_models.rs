//! Real-data collision coverage for models observed by the retail pair-runtime census.

use v2k_formats::models::{AnimVars, ModelEntry, ModelMaterializationContext};
use v2k_formats::terrain::{TerrainCell, TerrainGrid, GRID_SIZE};
use v2k_game::damage::PRIMARY_PROJECTILE_DAMAGE_PACKET;
use v2k_game::entity_collision_state::RetailRuntimeValue;
use v2k_game::player::{PlayerCraft, VehicleMode};
use v2k_game::player_hull::PlayerHull;
use v2k_game::session::GameSession;
use v2k_game::static_kind_catalog::{static_damage_profile, static_kind_descriptor};
use v2k_game::terrain_contact::{
    resolve_player_terrain_contact, PlayerTerrainStyleCallbackRequest,
    DEFAULT_PLAYER_TERRAIN_HAPTIC_SCALE_RAW,
};

const IDENTITY: [[f64; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
const QUARTER_TURN_Y: [[f64; 3]; 3] = [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [-1.0, 0.0, 0.0]];

/// One model selected by the 2026-07-19 retail pair-runtime census.
///
/// `contact_slot` is the center of an authored sphere (and, for gated models,
/// a detailed sphere rather than the broad gate). Probes built from it
/// therefore exercise a real primitive and do not manufacture a giant sphere
/// merely to force a hit.
#[derive(Clone, Copy)]
struct CensusModel {
    global_id: usize,
    name: &'static str,
    radius_raw: u16,
    contact_slot: u16,
}

const CENSUS_MODELS: [CensusModel; 8] = [
    CensusModel {
        global_id: 38,
        name: "zebrafsh",
        radius_raw: 120,
        contact_slot: 0x40,
    },
    CensusModel {
        global_id: 82,
        name: "powerup",
        radius_raw: 384,
        contact_slot: 0,
    },
    CensusModel {
        global_id: 138,
        name: "trophy",
        radius_raw: 210,
        contact_slot: 2,
    },
    CensusModel {
        global_id: 256,
        name: "spider",
        radius_raw: 300,
        contact_slot: 0x2c,
    },
    CensusModel {
        global_id: 302,
        name: "newant",
        radius_raw: 140,
        contact_slot: 0x4c,
    },
    CensusModel {
        global_id: 341,
        name: "hive1xa",
        radius_raw: 1049,
        contact_slot: 0,
    },
    CensusModel {
        global_id: 558,
        name: "man2",
        radius_raw: 165,
        contact_slot: 6,
    },
    CensusModel {
        global_id: 560,
        name: "grock",
        radius_raw: 284,
        contact_slot: 0x20,
    },
];

fn census_session() -> GameSession {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).expect("present retail preload must parse");
    // The census uses the common pool plus the first-world biome. Variant 1
    // is the normal 640x480 runtime tier, not the low-resolution 0X data.
    session
        .load_auxiliary_ovl(3, 1)
        .expect("present normal-tier system overlay must parse");
    session
        .load_auxiliary_ovl(6, 1)
        .expect("present normal-tier first-world overlay must parse");
    session
}

fn census_model(session: &GameSession, expected: CensusModel) -> &ModelEntry {
    let model = session
        .cache
        .global_model(expected.global_id)
        .unwrap_or_else(|| panic!("missing census model {}", expected.global_id));
    assert_eq!(
        model.name.as_deref(),
        Some(expected.name),
        "global model {} identity drifted",
        expected.global_id
    );
    assert_eq!(
        model.collision_radius_raw, expected.radius_raw,
        "global model {} collision radius drifted",
        expected.global_id
    );
    assert!(
        !model.collision_program.is_empty(),
        "global model {} has no authored collision program",
        expected.global_id
    );
    model
}

fn apply_basis(basis: [[f64; 3]; 3], point: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|row| {
        basis[row][0] * point[0] + basis[row][1] * point[1] + basis[row][2] * point[2]
    })
}

fn flat_terrain(height: i8) -> TerrainGrid {
    TerrainGrid {
        header: [0; 5],
        cells: vec![
            TerrainCell {
                height: height as u8,
                attribute: 0,
                terrain_type: 0,
            };
            GRID_SIZE * GRID_SIZE
        ],
    }
}

#[v2k_test_support::retail_test]
fn runtime_player_basis_keeps_nose_up_tail_out_of_flat_terrain() {
    let session = census_session();
    let player_model = session
        .cache
        .global_model(41)
        .expect("global player4 model");
    assert_eq!(player_model.name.as_deref(), Some("player4"));
    let terrain = flat_terrain(0);
    let vars = AnimVars::default();
    let identity = IDENTITY;
    let origin = [128, 159, 128];
    assert!(i32::from(origin[1]) - i32::from(player_model.radius / 2) > 0);
    assert!(player_model
        .collide_terrain_raw_oriented(&terrain, origin, identity, &vars)
        .expect("player4 terrain program")
        .is_none());

    // The captured normal mode-10 recurrence rests near -7572 under sustained
    // Down+Space. Feed that attainable signed-angle pose through PlayerCraft's
    // exact Q31 body columns, rather than a hand-authored test matrix.
    let mut craft = PlayerCraft::new();
    craft.body_pitch = -7572.0 / 65536.0 * std::f32::consts::TAU;
    let basis = craft.collision_model_orientation(std::f32::consts::FRAC_PI_2);
    let hit = player_model
        .collide_terrain_raw_oriented(&terrain, origin, basis, &vars)
        .expect("player4 terrain program")
        .expect("the attainable nose-up pose must lower an authored tail sphere");
    assert!(hit.normal[1] > 0.0);

    let mut position_raw = origin;
    let mut velocity_raw = [0, -1000, 0];
    let position_before = position_raw;
    let mut hull = PlayerHull::default();
    let outcome = resolve_player_terrain_contact(
        &terrain,
        player_model,
        basis,
        &vars,
        PlayerTerrainStyleCallbackRequest {
            behavior_context: RetailRuntimeValue::Known(None),
            vehicle_mode: VehicleMode::Hover,
            entity_handle: 1,
            controlled_entity_handle: Some(1),
            ground_response_selectors: [0; 8],
            haptic_scale_raw: DEFAULT_PLAYER_TERRAIN_HAPTIC_SCALE_RAW,
        },
        &mut position_raw,
        &mut velocity_raw,
        &mut hull,
    )
    .expect("terrain response")
    .expect("oriented tail contact");
    assert!(position_raw[1] > position_before[1]);
    assert!(outcome.contact.normal_q12[1] > 0);
    assert!(velocity_raw[1] >= -1);
}

#[v2k_test_support::retail_test]
fn census_models_execute_authored_contact_and_miss_paths() {
    let session = census_session();
    let vars = AnimVars::default();

    for expected in CENSUS_MODELS {
        let model = census_model(&session, expected);
        let contact_center = model
            .resolve_slot_with_context(
                expected.contact_slot,
                ModelMaterializationContext::intrinsic(&vars, None),
            )
            .unwrap_or_else(|| {
                panic!(
                    "model {} cannot resolve authored contact slot {:#x}",
                    expected.global_id, expected.contact_slot
                )
            })
            .position_raw;

        for (label, basis) in [("identity", IDENTITY), ("quarter-turn", QUARTER_TURN_Y)] {
            let oriented_center = apply_basis(basis, contact_center);
            let hit = model
                .collide_sphere_raw_oriented(oriented_center, 1, basis, &vars, &session.cache)
                .unwrap_or_else(|error| {
                    panic!(
                        "model {} ({}) {label} contact program failed: {error:?}",
                        expected.global_id, expected.name
                    )
                });
            assert!(
                hit.is_some(),
                "model {} ({}) missed its authored contact slot in {label}",
                expected.global_id,
                expected.name
            );
        }

        let miss_center = [f64::from(expected.radius_raw) * 3.0, 0.0, 0.0];
        let miss = model
            .collide_sphere_raw_oriented(miss_center, 1, QUARTER_TURN_Y, &vars, &session.cache)
            .unwrap_or_else(|error| {
                panic!(
                    "model {} ({}) miss path failed: {error:?}",
                    expected.global_id, expected.name
                )
            });
        assert_eq!(
            miss, None,
            "model {} ({}) broad miss produced a contact",
            expected.global_id, expected.name
        );
    }
}

#[v2k_test_support::retail_test]
fn census_models_execute_as_both_sides_of_pair_contacts() {
    let session = census_session();
    let vars = AnimVars::default();

    for query_expected in CENSUS_MODELS {
        let query = census_model(&session, query_expected);
        let query_center = query
            .resolve_slot_with_context(
                query_expected.contact_slot,
                ModelMaterializationContext::intrinsic(&vars, None),
            )
            .expect("validated query contact slot")
            .position_raw;
        let oriented_query_center = apply_basis(QUARTER_TURN_Y, query_center);

        for target_expected in CENSUS_MODELS {
            let target = census_model(&session, target_expected);
            let target_center = target
                .resolve_slot_with_context(
                    target_expected.contact_slot,
                    ModelMaterializationContext::intrinsic(&vars, None),
                )
                .expect("validated target contact slot")
                .position_raw;
            // Place the two authored detailed-sphere centers together. This is
            // a data-derived overlap and exercises the query model's gate and
            // detailed spheres against the target model's complete program.
            let query_origin =
                std::array::from_fn(|axis| target_center[axis] - oriented_query_center[axis]);
            let hit = query
                .collide_model_raw_oriented(
                    target,
                    query_origin,
                    QUARTER_TURN_Y,
                    &vars,
                    &vars,
                    &session.cache,
                )
                .unwrap_or_else(|error| {
                    panic!(
                        "query {} ({}) -> target {} ({}) failed: {error:?}",
                        query_expected.global_id,
                        query_expected.name,
                        target_expected.global_id,
                        target_expected.name
                    )
                });
            assert!(
                hit.is_some(),
                "query {} ({}) -> target {} ({}) missed aligned authored primitives",
                query_expected.global_id,
                query_expected.name,
                target_expected.global_id,
                target_expected.name
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn canonical_spider_pen_keeps_intrinsic_collision_slots_and_authored_damage() {
    let mut session = census_session();
    session
        .load_level_by_id(13, 1)
        .expect("present first-level overlay must parse");
    let objects = session.cache.terrain_objects().expect("level object table");
    let terrain = session.cache.terrain().expect("level terrain");
    let vars = AnimVars::default();

    // Every intact spike variant actually placed around level 13's pen,
    // including the three authored x variants. The ordinary fence family
    // (492..499) instead uses convex-plane collision programs.
    for (attribute, global_id, name) in [
        (53, 512, "spikes"),
        (54, 514, "spikes2"),
        (56, 518, "spikes4"),
        (57, 520, "spikes5"),
        (58, 522, "spikes6"),
        (63, 540, "spikes7x"),
        (64, 532, "spikes3x"),
        (68, 542, "spikes8x"),
    ] {
        assert!(terrain.cells.iter().any(|cell| cell.attribute == attribute));
        let descriptor = &objects.records[usize::from(attribute)];
        assert_eq!(descriptor.kind_index, 9);
        assert_eq!(usize::from(descriptor.model_id_for(0)), global_id);
        let model = session.cache.global_model(global_id).expect("pen model");
        assert_eq!(model.name.as_deref(), Some(name));
        assert_eq!(model.collision_radius_raw, 512);

        // 6AF20 always installs the intrinsic table at 4D4A78: its tf12
        // entry 4D4B20 is 6E9F0's XYZ alias, unlike the terrain-grounded
        // render table installed by 33FA0. Slot14 is the midpoint of slot0
        // and slot8; the tf12 -> tf6 dependency therefore retains Y192 in
        // collision even after the renderer correctly lowers the fence.
        let endpoint = model.records[2];
        let start = model.records[1];
        let center = [
            f64::from(endpoint[1] + start[1]) / 2.0,
            192.0,
            f64::from(endpoint[3] + start[3]) / 2.0,
        ];
        let mut probe = center;
        probe[1] -= 263.0;
        let hit = model
            .collide_sphere_raw(probe, 8, &vars, &session.cache)
            .unwrap_or_else(|error| panic!("{name} collision failed: {error:?}"))
            .expect("one raw unit of detailed-sphere penetration");
        assert_eq!(hit.penetration_raw, 1.0, "{name}");
        assert_eq!(hit.normal, [0.0, -1.0, 0.0], "{name}");

        // Still inside the root sphere, but tangent to the detailed sphere:
        // the broad gate cannot become a final contact or damage event.
        probe[1] -= 1.0;
        assert_eq!(
            model.collide_sphere_raw(probe, 8, &vars, &session.cache),
            Ok(None),
            "{name} promoted a broad gate or tangent into a hit"
        );
        let broken = session
            .cache
            .global_model(usize::from(descriptor.model_id_for(8)))
            .expect("broken pen model");
        assert_eq!(broken.collision_radius_raw, 0);
        assert!(broken.collision_program.is_empty());

        // Retail globals 4CBF70/4CBF88 both contain channel2 amount2000;
        // kind9's 4C9C90 profile uses threshold1000 and multiplier512.
        // An admitted hit is destructible: geometry must not be repaired
        // by inventing primary-weapon immunity for this material.
        let kind = static_kind_descriptor(descriptor.kind_index).expect("pen kind");
        let profile = static_damage_profile(kind.damage_profile_va())
            .expect("pen damage profile")
            .decoded();
        assert_eq!(profile.filter(PRIMARY_PROJECTILE_DAMAGE_PACKET), 2000);
    }
}
