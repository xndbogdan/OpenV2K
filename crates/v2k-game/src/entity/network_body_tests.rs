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

/// A player (type 46) with two cargo slots, the first drawn in place (its
/// first byte nonzero), and a peer's Type68.
fn cargo_manager() -> (EntityManager, u32, u32) {
    use v2k_formats::collision::{SubJAttachmentDescriptor, SubJAttachmentSlotDescriptor};
    let descriptor = SubJAttachmentDescriptor {
        reserved_at_0x01: 0,
        slots: vec![
            SubJAttachmentSlotDescriptor {
                policy_word_raw: 0x0101,
                local_offset_raw: [0; 3],
            },
            SubJAttachmentSlotDescriptor {
                policy_word_raw: 0x0100,
                local_offset_raw: [0; 3],
            },
        ]
        .into_boxed_slice(),
    };
    let mut metadata = vec![EntityTypeRuntimeMetadata::default(); 70];
    metadata[46].sub_j_attachment_descriptor = RetailRuntimeValue::Known(Some(descriptor.clone()));
    let mut player = Entity::unresolved_port_entity(1, EntityKind::Player, 46);
    player.set_position_raw([0x1200, 0x300, -0x0400]);
    player.sub_j_attachment_runtime = RetailRuntimeValue::Known(Some(
        crate::sub_j_attachment::SubJAttachmentRuntime::from_descriptor(&descriptor).unwrap(),
    ));
    let mut em =
        EntityManager::from_entities_with_type_metadata_for_test(vec![player], metadata, false);
    let cargo = em.append_network_body(68).unwrap();
    (em, 1, cargo)
}

fn sync(flags: u16) -> BodySync {
    BodySync {
        flags,
        position: [0x700, 0x80, 0x700],
        handle: 0x0800_0001,
        parent: 0x0400_0001,
        ..BodySync::default()
    }
}

#[test]
fn a_peer_body_attached_to_a_player_that_does_not_carry_it_is_dropped_by_kind_15() {
    let (mut em, player, cargo) = cargo_manager();
    // No position of its own: it takes the player's, and the player's X and
    // Z go back to its owner.
    let repair = em.apply_network_sync(cargo, &sync(0x1000 | 0x800), Some(player));
    assert_eq!(
        repair,
        Some(NetworkRelationRepair::Dropped {
            x_raw: 0x1200,
            z_raw: -0x0400
        })
    );
    let body = em.network_entity(cargo).unwrap();
    assert_eq!(body.attached_to, Some(player));
    assert_eq!(body.position_raw(), [0x1200, 0x300, -0x0400]);
    // Once the player carries it, nothing is sent.
    em.player_sub_j_runtime_mut()
        .unwrap()
        .append(cargo)
        .unwrap();
    assert_eq!(
        em.apply_network_sync(cargo, &sync(0x1000 | 0x820), Some(player)),
        None
    );
    assert_eq!(
        em.network_entity(cargo).unwrap().position_raw(),
        [0x700, 0x80, 0x700]
    );
}

#[test]
fn a_carried_peer_body_its_owner_released_is_hidden_and_asked_for_again() {
    let (mut em, player, cargo) = cargo_manager();
    em.player_sub_j_runtime_mut()
        .unwrap()
        .append(cargo)
        .unwrap();
    em.entity_mut(cargo).unwrap().attached_to = Some(player);
    assert_eq!(em.apply_network_sync(cargo, &sync(0x820), None), None);
    let body = em.network_entity(cargo).unwrap();
    assert_eq!(body.attached_to, None);
    assert_eq!(
        body.collision.state_flags_at_0x08.masked(0x800),
        RetailRuntimeValue::Known(0)
    );
    // Its slot is drawn in place: flags 2, the drawn bit untouched.
    em.entity_mut(cargo)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(0x800, 0x800);
    assert_eq!(em.network_player_cargo_rows(), vec![(cargo, 2)]);
    assert_eq!(
        em.network_entity(cargo)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .masked(0x800),
        RetailRuntimeValue::Known(0x800)
    );
    // Attached by its owner: no request, and the row stays.
    em.entity_mut(cargo).unwrap().attached_to = Some(player);
    assert!(em.network_player_cargo_rows().is_empty());
    assert_eq!(
        em.player_sub_j_runtime().unwrap().ordered_entity_ids(),
        &[cargo]
    );
}

#[test]
fn cargo_rows_compact_and_the_next_peer_body_takes_the_freed_slot() {
    let (mut em, player, cargo) = cargo_manager();
    let other = em.append_network_body(68).unwrap();
    for id in [cargo, other] {
        em.player_sub_j_runtime_mut().unwrap().append(id).unwrap();
        em.entity_mut(id)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .overwrite(0x800, 0x800);
    }
    // The first body is attached to another body: its row goes, and the
    // second, unattached, is asked for in slot 0 (drawn, flags 2).
    em.entity_mut(cargo).unwrap().attached_to = Some(other);
    assert_eq!(em.network_player_cargo_rows(), vec![(other, 2)]);
    assert_eq!(
        em.player_sub_j_runtime().unwrap().ordered_entity_ids(),
        &[other]
    );
    // In slot 1 (its first byte zero) a body loses its drawn bit; a dying
    // one is asked for once more, then loses its row.
    em.entity_mut(cargo).unwrap().attached_to = None;
    em.player_sub_j_runtime_mut()
        .unwrap()
        .append(cargo)
        .unwrap();
    em.entity_mut(other).unwrap().attached_to = Some(player);
    em.entity_mut(cargo)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(0x4000, 0x4000);
    assert_eq!(em.network_player_cargo_rows(), vec![(cargo, 0)]);
    assert_eq!(
        em.network_entity(cargo)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .masked(0x800),
        RetailRuntimeValue::Known(0)
    );
    assert_eq!(
        em.player_sub_j_runtime().unwrap().ordered_entity_ids(),
        &[other]
    );
}

#[test]
fn dropping_a_peer_body_sends_it_to_its_owner_without_a_materialiser() {
    let (mut em, player, cargo) = cargo_manager();
    em.player_sub_j_runtime_mut()
        .unwrap()
        .append(cargo)
        .unwrap();
    // Its owner has not attached it: the row still names it.
    em.entity_mut(cargo).unwrap().attached_to = None;
    let terrain = TerrainGrid {
        header: [0; 5],
        cells: vec![
            v2k_formats::terrain::TerrainCell {
                height: 2,
                attribute: 0,
                terrain_type: 0,
            };
            v2k_formats::terrain::GRID_SIZE * v2k_formats::terrain::GRID_SIZE
        ],
    };
    let live = em.iter_all().count();
    em.queue_beam(BeamCommand::Drop);
    let outcome = em
        .tick_beam_with_drop_context(
            81_000,
            Some(CargoDropContext {
                carrier_orientation: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                terrain: &terrain,
            }),
        )
        .unwrap();
    assert_eq!(outcome, BeamOutcome::Dropped { entity_id: cargo });
    assert_eq!(em.iter_all().count(), live, "no Type93");
    assert!(em.player_sub_j_runtime().unwrap().is_empty());
    let events = em.take_cargo_proxy_events();
    let [CargoProxyEvent::PeerDropped {
        parent_id,
        cargo_id,
        target_raw,
        sound_id,
        ..
    }] = events.as_slice()
    else {
        panic!("{events:?}");
    };
    assert_eq!((*parent_id, *cargo_id, *sound_id), (player, cargo, 8));
    // The terrain under the drop point (height 2, 0x40 raw).
    assert_eq!(target_raw[1], 0x40);
}
