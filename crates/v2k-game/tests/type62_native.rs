//! Native Type62 construction and quiet abort from ordinary authored loads.

use v2k_formats::levels::LevelDescriptor;
use v2k_game::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    entity::{
        AuthoredPlayerArrival, AuthoredWorldConstruction, EntityConstructionResources,
        EntityManager,
    },
    entity_collision_state::EntityTypeRuntimeMetadata,
    main_base_abort::{MainBaseAbortQuietDeathAdvance, MainBaseAbortQuietDeathOutcome},
    session::GameSession,
    world_fx::WorldFx,
};

struct World {
    manager: EntityManager,
    fx: WorldFx,
}

impl World {
    fn load(level_id: u32) -> Self {
        Self::load_with_spawn_edit(level_id, |_| {})
    }

    fn load_with_spawn_edit(level_id: u32, edit: impl FnOnce(&mut LevelDescriptor)) -> Self {
        let data = v2k_test_support::retail_dir();
        assert!(
            data.join("PRELOAD.DAT").is_file(),
            "normal-tier corpus required"
        );
        let mut session = GameSession::init(&data).unwrap();
        session.load_auxiliary_ovl(3, 1).unwrap();
        session.load_level_by_id(level_id, 1).unwrap();
        let metadata: Vec<_> = session
            .cache
            .global_entity_model_table()
            .iter()
            .enumerate()
            .map(|(id, slots)| {
                session
                    .cache
                    .global_entity_type(id)
                    .map(EntityTypeRuntimeMetadata::from_section12)
                    .unwrap_or(EntityTypeRuntimeMetadata {
                        model_slots: *slots,
                        ..Default::default()
                    })
            })
            .collect();
        let bytes =
            std::fs::read(data.join("Overlay").join(format!("1X{level_id}XX.OVL"))).unwrap();
        let overlay = v2k_formats::ovl::OvlFile::parse(&bytes).unwrap();
        let mut level =
            v2k_formats::levels::parse_level(&overlay.section(13).unwrap().data).unwrap();
        edit(&mut level);
        let mut fx = WorldFx::new();
        let manager = EntityManager::from_authored_world(
            AuthoredWorldConstruction {
                logical_world_index: (level_id - 12) as i32,
                level: &level,
                type_metadata: &metadata,
                resources: EntityConstructionResources {
                    terrain: session.cache.terrain(),
                    terrain_objects: session.cache.terrain_objects(),
                    model_extent_raw: Some(&|id| {
                        session.cache.global_model(id).map(|model| model.radius)
                    }),
                },
                player_arrival: Some(AuthoredPlayerArrival {
                    position_raw: [19_712, -500, 14_848],
                    heading_raw: 0x4000,
                }),
                retail_tick: 4793,
            },
            &mut fx,
        )
        .unwrap();
        Self { manager, fx }
    }

    fn first(&self, entity_type: u32) -> u32 {
        self.manager
            .iter_all()
            .find(|entity| entity.entity_type == entity_type)
            .unwrap_or_else(|| panic!("authored type{entity_type} required"))
            .id
    }
}

fn assert_native_type62(world: &World, id: u32) {
    let entity = world
        .manager
        .iter_all()
        .find(|entity| entity.id == id)
        .unwrap();
    assert_eq!(entity.entity_type, 62);
    assert_eq!(entity.model_slots, [Some(38); 4]);
    assert!(matches!(
        entity.current_behavior_context,
        v2k_game::entity_collision_state::RetailRuntimeValue::Known(Some(_))
    ));
    assert!(matches!(
        entity.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::OrdinaryType9Wander(_))
    ));
    assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
    assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
}

fn abort_type62(world: &mut World, id: u32) {
    let observation = world.manager.main_base_abort_actor_observation(id).unwrap();
    let advance = world
        .manager
        .apply_main_base_abort_quiet_death(observation.lease, &mut world.fx)
        .expect("native Type62 quiet abort");
    assert!(matches!(
        advance,
        MainBaseAbortQuietDeathAdvance::Advanced {
            outcome: MainBaseAbortQuietDeathOutcome::DeferredDestroyStaged {
                entity_id,
                entity_type: 62,
                death_sound_id: None,
            },
            ..
        } if entity_id == id
    ));
}

#[v2k_test_support::retail_test]
fn overlay13_type62_constructs_wander_and_aborts_from_receipt() {
    let mut world = World::load(13);
    let id = world.first(62);
    let spawn = world
        .manager
        .iter_all()
        .find(|entity| entity.id == id)
        .unwrap()
        .authored_spawn_index;
    assert_eq!(spawn, Some(4));
    assert_native_type62(&world, id);
    abort_type62(&mut world, id);
}

#[v2k_test_support::retail_test]
fn later_world_type62_constructs_outside_overlay13_spawn_tables() {
    let mut world = World::load(14);
    let id = world.first(62);
    let spawn = world
        .manager
        .iter_all()
        .find(|entity| entity.id == id)
        .unwrap()
        .authored_spawn_index;
    assert_eq!(spawn, Some(1));
    assert_ne!(spawn, Some(4));
    assert_ne!(spawn, Some(25));
    assert_ne!(spawn, Some(26));
    assert_native_type62(&world, id);
    abort_type62(&mut world, id);
}

#[v2k_test_support::retail_test]
fn overlay18_type62_constructs_and_aborts_from_receipt() {
    let mut world = World::load(18);
    let id = world.first(62);
    let spawn = world
        .manager
        .iter_all()
        .find(|entity| entity.id == id)
        .unwrap()
        .authored_spawn_index;
    assert!(spawn.is_some());
    assert_native_type62(&world, id);
    abort_type62(&mut world, id);
}

#[v2k_test_support::retail_test]
fn type62_keeps_authored_y_and_rotation() {
    let mut world = World::load_with_spawn_edit(14, |level| {
        let spawn = level
            .entities
            .iter_mut()
            .find(|spawn| spawn.entity_type == 62)
            .expect("Medaeval authors Type62");
        spawn.rotation = [0x1234, 0x5678, 0x9abc];
    });
    let id = world.first(62);
    let entity = world
        .manager
        .iter_all()
        .find(|entity| entity.id == id)
        .unwrap();
    assert_eq!(
        entity.rotation_heading_pitch_roll_raw(),
        [0x1234, 0x5678_u16 as i16, 0x9abc_u16 as i16]
    );
    assert_native_type62(&world, id);
    abort_type62(&mut world, id);
}

#[v2k_test_support::retail_test]
fn type62_quiet_abort_keeps_allocation_custody_after_swimming_from_birth_anchor() {
    let mut world = World::load(13);
    let id = world.first(62);
    let entity = world.manager.entity_mut(id).unwrap();
    entity.position[0] += 17.0 / 256.0;
    entity.position[2] -= 23.0 / 256.0;
    abort_type62(&mut world, id);
}
