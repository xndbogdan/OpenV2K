use v2k_game::{
    entity::{EntityConstructionResources, EntityManager},
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    intro2_commands::{Intro2ActorOperation, Intro2Commands},
    session::GameSession,
};

#[v2k_test_support::retail_test]
fn authored_intro2_commands_retain_camera_and_repeat_enables_only_in_their_window() {
    let path = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&path).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(50, 1).unwrap();
    let metadata: Vec<_> = session
        .cache
        .global_entity_model_table()
        .iter()
        .enumerate()
        .map(|(kind, _)| {
            EntityTypeRuntimeMetadata::from_section12(
                session.cache.global_entity_type(kind).unwrap(),
            )
        })
        .collect();
    let mut entities = EntityManager::from_level_with_type_metadata(
        session.cache.level_desc().unwrap(),
        &metadata,
        EntityConstructionResources::new(session.cache.terrain(), session.cache.terrain_objects()),
    );
    let mut commands = Intro2Commands::from_cache(&session.cache).unwrap();
    assert_eq!(
        commands.camera_subject(&entities),
        None,
        "44F8B0 clears selection before first presentation"
    );
    for (spawn, start, end) in [
        (24, 15000, 15500),
        (26, 15000, 15500),
        (1, 22000, 22500),
        (40, 22000, 22500),
    ] {
        assert!(commands
            .records()
            .iter()
            .any(|record| record.spawn_index == spawn
                && record.start_millis == start
                && record.end_millis == Some(end)
                && record.operation == Intro2ActorOperation::EnableComponents));
    }
    let id = entities
        .iter_all()
        .find(|e| e.authored_spawn_index == Some(33))
        .unwrap()
        .id;
    entities.set_authored_behavior_components_enabled(33, false);
    let enabled = |entities: &EntityManager| {
        entities
            .iter_all()
            .find(|e| e.id == id)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .masked(0x68000)
    };
    commands.present(&mut entities, 24);
    assert_eq!(enabled(&entities), RetailRuntimeValue::Known(0));
    commands.present(&mut entities, 25);
    assert_eq!(enabled(&entities), RetailRuntimeValue::Known(0x68000));
    // A later callback may clear the mask. An active command writes it again,
    // including its exact end; the first tick outside does not re-enable it.
    entities
        .entity_mut(id)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(0x68000, 0);
    commands.present(&mut entities, 100);
    assert_eq!(enabled(&entities), RetailRuntimeValue::Known(0x68000));
    entities
        .entity_mut(id)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(0x68000, 0);
    commands.present(&mut entities, 101);
    assert_eq!(enabled(&entities), RetailRuntimeValue::Known(0));

    let subject = |spawn| {
        entities
            .iter_all()
            .find(|e| e.authored_spawn_index == Some(spawn))
            .unwrap()
            .position_raw()
    };
    let anchor = subject(32);
    let ant = subject(8);
    assert_eq!(
        commands.camera_subject(&entities).unwrap().position_raw,
        anchor
    );
    commands.present(&mut entities, 749);
    assert_eq!(
        commands.camera_subject(&entities).unwrap().position_raw,
        anchor
    );
    commands.present(&mut entities, 750);
    assert_eq!(
        commands.camera_subject(&entities).unwrap().position_raw,
        ant
    );
    commands.present(&mut entities, 1000);
    assert_eq!(
        commands.camera_subject(&entities).unwrap().position_raw,
        ant,
        "gaps retain the selected handle"
    );
}
