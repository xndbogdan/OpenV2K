use super::*;

fn manager() -> EntityManager {
    let mut metadata = vec![EntityTypeRuntimeMetadata::default(); 60];
    metadata[REMOTE_CRAFT_TYPE as usize].model_slots = [301, 301, 301, 301];
    metadata[REMOTE_CRAFT_TYPE as usize].capability_flags = 5;
    EntityManager::from_entities_with_type_metadata_for_test(Vec::new(), metadata, false)
}

#[test]
fn a_peer_body_takes_its_types_models_and_the_remote_bit() {
    let mut em = manager();
    let id = em.append_network_body(REMOTE_CRAFT_TYPE).unwrap();
    let body = em.network_entity(id).unwrap();
    assert!(body.is_network_remote());
    assert_eq!(body.model_index, Some(301));
    assert_eq!(body.capability_flags, 5);
    assert_eq!(
        body.collision.state_flags_at_0x08.masked(REMOTE_STATE_BIT),
        RetailRuntimeValue::Known(REMOTE_STATE_BIT)
    );
    assert!(em.network_local_ids().is_empty());
    assert!(
        em.append_network_body(59).is_some(),
        "every type with a row"
    );
}

#[test]
fn a_sync_record_moves_the_body_and_its_velocity_carries_it_on() {
    let mut em = manager();
    let id = em.append_network_body(REMOTE_CRAFT_TYPE).unwrap();
    em.apply_network_sync(
        id,
        &BodySync {
            flags: 0x20 | 0x10 | 0x80 | 0x800,
            position: [256, 512, -256],
            velocity: [3200, 0, 0],
            model_variables: vec![1, 2, 3],
            ..BodySync::default()
        },
        None,
    );
    let body = em.network_entity(id).unwrap();
    assert_eq!(body.position_raw(), [256, 512, -256]);
    assert_eq!(body.network_body().unwrap().model_variables, vec![1, 2, 3]);
    // 20 ms: (20000 >> 5) * 3200 >> 15 = 61.
    assert!(em.advance_network_bodies(20_000).is_empty());
    assert_eq!(
        em.network_entity(id).unwrap().position_raw(),
        [317, 512, -256]
    );
}

#[test]
fn a_silent_body_lapses_and_is_removed() {
    let mut em = manager();
    let id = em.append_network_body(REMOTE_CRAFT_TYPE).unwrap();
    assert!(em.advance_network_bodies(REMOTE_TIMEOUT_US - 1).is_empty());
    assert_eq!(em.advance_network_bodies(1), vec![id]);
    assert!(em.remove_network_body(id));
    assert!(em.network_entity(id).is_none());
}

#[test]
fn the_basis_rows_survive_the_round_trip() {
    let basis = Type9BodyBasis::from_angle_words(0x1234, 0x0300, -0x0200);
    let rebuilt = basis_from_rows(basis_rows(&basis));
    for (a, b) in [
        (basis.forward, rebuilt.forward),
        (basis.lateral, rebuilt.lateral),
        (basis.up, rebuilt.up),
    ] {
        for axis in 0..3 {
            assert!((a[axis] - b[axis]).abs() < 1 << 21, "{a:?} vs {b:?}");
        }
    }
}
