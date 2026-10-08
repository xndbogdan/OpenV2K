//! The retained player's 20450 constructor consumes the process stream once.

use v2k_formats::{levels::LevelDescriptor, terrain::TerrainGrid};
use v2k_game::{
    common_mover::shared_initializer_target_speed_raw,
    entity::{
        AuthoredPlayerArrival, AuthoredWorldConstruction, EntityConstructionResources,
        EntityManager, NativeSaveWorldState,
    },
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    player::PlayerCraft,
    save::{NativeCompatibilityPreview, NativeSaveRestore},
    session::GameSession,
    static_damage::StaticDamageScheduler,
    world_fx::WorldFx,
};

struct Fixture {
    session: GameSession,
    level: LevelDescriptor,
    terrain: TerrainGrid,
    metadata: Vec<EntityTypeRuntimeMetadata>,
}

impl Fixture {
    fn new() -> Self {
        let dir = v2k_test_support::retail_dir();
        let mut session = GameSession::init(&dir).expect("canonical retail data required");
        session.load_auxiliary_ovl(3, 1).unwrap();
        session.load_level_by_id(14, 1).unwrap();
        let metadata = session
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
        let mut level = Self::authored_level();
        level.entities.clear();
        let mut terrain = session.cache.terrain().unwrap().clone();
        // Isolate 443560's process draw from the separate 33BD0 helper loop and
        // authored entity constructors, which own their own later draws.
        for cell in &mut terrain.cells {
            cell.attribute = 0;
            cell.terrain_type = 0;
        }
        Self {
            session,
            level,
            terrain,
            metadata,
        }
    }

    fn authored_level() -> LevelDescriptor {
        let path = v2k_test_support::retail_dir().join("Overlay/1X14XX.OVL");
        let bytes = std::fs::read(path).unwrap();
        let overlay = v2k_formats::ovl::OvlFile::parse(&bytes).unwrap();
        v2k_formats::levels::parse_level(&overlay.section(13).unwrap().data).unwrap()
    }

    fn request(&self, player: bool) -> AuthoredWorldConstruction<'_> {
        AuthoredWorldConstruction {
            level: &self.level,
            logical_world_index: 2,
            type_metadata: &self.metadata,
            resources: EntityConstructionResources::new(
                Some(&self.terrain),
                self.session.cache.terrain_objects(),
            ),
            player_arrival: player.then_some(AuthoredPlayerArrival {
                position_raw: [1_000, -500, 2_000],
                heading_raw: 0x4000,
            }),
            retail_tick: 0,
        }
    }

    fn expected_target(&self, word: u16) -> i32 {
        let RetailRuntimeValue::Known(Some(a)) = self.metadata[46].sub_a_propulsion_descriptor
        else {
            panic!("actual player Sub-A")
        };
        shared_initializer_target_speed_raw(a.target_speed_base_raw, word)
    }
}

fn target(manager: &EntityManager) -> i32 {
    let RetailRuntimeValue::Known(Some(runtime)) =
        manager.player().unwrap().sub_a_propulsion_runtime
    else {
        panic!("constructed player Sub-A")
    };
    let RetailRuntimeValue::Known(target) = runtime.target_speed_raw() else {
        panic!("completed process RNG draw")
    };
    target
}

#[v2k_test_support::retail_test]
fn native_player_constructor_preserves_process_history_and_is_idempotent() {
    let fixture = Fixture::new();
    for history in [0, 7, 41] {
        let mut fx = WorldFx::new();
        let mut oracle = WorldFx::new();
        for _ in 0..history {
            assert_eq!(
                fx.next_shared_retail_random_u16(),
                oracle.next_shared_retail_random_u16()
            );
        }
        let expected = fixture.expected_target(oracle.next_shared_retail_random_u16());
        let mut manager =
            EntityManager::from_authored_world(fixture.request(true), &mut fx).unwrap();
        assert_eq!(target(&manager), expected);
        manager.construct_player_propulsion(&mut fx).unwrap();
        manager.construct_player_propulsion(&mut fx).unwrap();
        assert_eq!(target(&manager), expected);
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            oracle.next_shared_retail_random_u16(),
            "exactly one 20450 draw, with no replay on binding"
        );
        let physics = v2k_game::hover::HoverPhysicsConfig::from_record(
            fixture.session.cache.global_entity_type(46).unwrap(),
        )
        .unwrap();
        let craft = PlayerCraft::with_hover_target(physics, target(&manager));
        assert_eq!(craft.hover_target_speed_raw(), expected);
    }
}

#[v2k_test_support::retail_test]
fn no_player_does_not_consume_a_propulsion_draw() {
    let fixture = Fixture::new();
    let mut fx = WorldFx::new();
    let mut oracle = WorldFx::new();
    let mut manager = EntityManager::from_authored_world(fixture.request(false), &mut fx).unwrap();
    assert!(manager.player().is_none());
    manager.construct_player_propulsion(&mut fx).unwrap();
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        oracle.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn player_draw_precedes_real_marker_helpers_and_authored_actor_constructors() {
    let mut fixture = Fixture::new();
    fixture.level = Fixture::authored_level();
    fixture.terrain = fixture.session.cache.terrain().unwrap().clone();
    assert!(!fixture.level.entities.is_empty());
    let mut fx = WorldFx::new();
    let mut oracle = WorldFx::new();
    for _ in 0..19 {
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            oracle.next_shared_retail_random_u16()
        );
    }
    let expected = fixture.expected_target(oracle.next_shared_retail_random_u16());
    let extent = |id| {
        fixture
            .session
            .cache
            .global_model(id)
            .map(|model| model.radius)
    };
    let mut request = fixture.request(true);
    request.resources.model_extent_raw = Some(&extent);
    let manager = EntityManager::from_authored_world(request, &mut fx).unwrap();
    assert!(manager
        .iter_all()
        .any(|entity| entity.authored_spawn_index.is_some()));
    assert_eq!(target(&manager), expected, "player 20450 precedes 33BD0 helpers and the first authored body, despite their later shared-stream consumption");
}

#[v2k_test_support::retail_test]
fn native_save_keeps_restored_pose_but_constructs_a_fresh_process_owned_target() {
    let fixture = Fixture::new();
    let mut payload = [0; 0x248];
    payload[0x20..0x24].copy_from_slice(&2_u32.to_le_bytes());
    payload[0x24..0x26].copy_from_slice(&1234_i16.to_le_bytes());
    payload[0x2a..0x2c].copy_from_slice(&17_i16.to_le_bytes());
    payload[0x3c..0x40].copy_from_slice(&30_000_i32.to_le_bytes());
    payload[0x40..0x44].copy_from_slice(&1_u32.to_le_bytes());
    payload[0x140..0x144].copy_from_slice(&46_u32.to_le_bytes());
    let restore = NativeSaveRestore::decode(&NativeCompatibilityPreview {
        logical_level_id: 2,
        state_payload: payload,
    })
    .unwrap();
    let mut terrain = fixture.terrain.clone();
    let mut static_damage = StaticDamageScheduler::new();
    let mut fx = WorldFx::new();
    let mut oracle = WorldFx::new();
    for _ in 0..11 {
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            oracle.next_shared_retail_random_u16()
        );
    }
    let expected = fixture.expected_target(oracle.next_shared_retail_random_u16());
    let mut manager = EntityManager::from_authored_world(
        fixture
            .request(false)
            .with_native_save(NativeSaveWorldState {
                restore: &restore,
                terrain: &mut terrain,
                resources: &fixture.session.cache,
                static_damage: &mut static_damage,
            }),
        &mut fx,
    )
    .unwrap();
    assert_eq!(
        manager.player().unwrap().position_raw(),
        restore.player.position_raw
    );
    assert_eq!(
        manager.player().unwrap().velocity_raw(),
        restore.player.velocity_raw
    );
    assert_eq!(target(&manager), expected);
    manager.construct_player_propulsion(&mut fx).unwrap();
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        oracle.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn portable_reconstruction_requires_explicit_completion_from_the_current_stream() {
    let fixture = Fixture::new();
    let mut manager = EntityManager::from_level_with_type_metadata(
        &fixture.level,
        &fixture.metadata,
        fixture.request(false).resources,
    );
    manager.place_or_spawn_player_at_campaign_arrival(
        Some(&fixture.metadata[46]),
        [1_000, -500, 2_000],
        0x4000,
        Some(&fixture.terrain),
    );
    let RetailRuntimeValue::Known(Some(runtime)) =
        manager.player().unwrap().sub_a_propulsion_runtime
    else {
        panic!("retained allocation")
    };
    assert_eq!(runtime.target_speed_raw(), RetailRuntimeValue::Unresolved);
    let mut fx = WorldFx::new();
    let mut oracle = WorldFx::new();
    for _ in 0..5 {
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            oracle.next_shared_retail_random_u16()
        );
    }
    let expected = fixture.expected_target(oracle.next_shared_retail_random_u16());
    manager.construct_player_propulsion(&mut fx).unwrap();
    assert_eq!(target(&manager), expected);
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        oracle.next_shared_retail_random_u16()
    );
}
