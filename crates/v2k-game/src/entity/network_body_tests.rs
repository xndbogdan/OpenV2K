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
    let id = em
        .append_network_body(REMOTE_CRAFT_TYPE, None, None)
        .unwrap()
        .id();
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
        em.append_network_body(59, None, None).is_some(),
        "every type with a row"
    );
}

#[test]
fn a_sync_record_moves_the_body_and_its_velocity_carries_it_on() {
    let mut em = manager();
    let id = em
        .append_network_body(REMOTE_CRAFT_TYPE, None, None)
        .unwrap()
        .id();
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
    let id = em
        .append_network_body(REMOTE_CRAFT_TYPE, None, None)
        .unwrap()
        .id();
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
    let cargo = em.append_network_body(68, None, None).unwrap().id();
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
    let other = em.append_network_body(68, None, None).unwrap().id();
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

/// Arena 6 (world 32) as a host builds it, with the RNG at a known state.
fn arena_world() -> (crate::session::GameSession, Vec<EntityTypeRuntimeMetadata>) {
    let mut session = crate::session::GameSession::init(&v2k_test_support::retail_dir()).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(44, 1).unwrap();
    let metadata = (0..session.cache.global_entity_model_table().len())
        .map(|id| {
            EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(id).unwrap())
        })
        .collect();
    (session, metadata)
}

fn arena_manager(
    session: &crate::session::GameSession,
    metadata: &[EntityTypeRuntimeMetadata],
    fx: &mut WorldFx,
) -> EntityManager {
    EntityManager::from_authored_world(
        AuthoredWorldConstruction {
            level: session.cache.level_desc().unwrap(),
            logical_world_index: 32,
            type_metadata: metadata,
            resources: EntityConstructionResources {
                terrain: session.cache.terrain(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&|id| session.cache.global_model(id).map(|m| m.radius)),
            },
            player_arrival: Some(AuthoredPlayerArrival {
                position_raw: [0x4000, 0x400, 0x4000],
                heading_raw: 0,
            }),
            retail_tick: 0,
        },
        fx,
    )
    .unwrap()
}

fn peer_craft(
    session: &crate::session::GameSession,
    manager: &mut EntityManager,
    fx: &mut WorldFx,
) -> u32 {
    let birth = manager
        .append_network_body(
            REMOTE_CRAFT_TYPE,
            Some([0x5000, 0x600, 0x5000]),
            Some(PeerBirthWorld {
                terrain: session.cache.terrain().unwrap(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&|id| session.cache.global_model(id).map(|m| m.radius)),
                world_fx: fx,
                retail_tick: 0,
            }),
        )
        .unwrap();
    let PeerBirth::Native(id) = birth else {
        panic!("{birth:?}");
    };
    id
}

fn advanced(mut fx: WorldFx, words: usize) -> u16 {
    for _ in 0..words {
        fx.next_shared_retail_random_u16();
    }
    fx.next_shared_retail_random_u16()
}

/// `FUN_00417680`'s birth of a peer's craft is `FUN_00438080`'s for type 51:
/// type 51's components and class 7 chosen at birth, from four shared words
/// (Sub-G's, the selector, and B6C0's two task constructors' Sub-G words),
/// with state bit 31 beside the constructor's `0x06000000`.
#[v2k_test_support::retail_test]
fn a_peers_craft_is_built_by_its_constructor() {
    let (session, metadata) = arena_world();
    let mut fx = WorldFx::new();
    let mut manager = arena_manager(&session, &metadata, &mut fx);
    fx.set_shared_retail_random_state(0x1234_5678);
    let expected = fx.fork_for_main_base_abort_transaction();
    let id = peer_craft(&session, &mut manager, &mut fx);
    assert_eq!(fx.next_shared_retail_random_u16(), advanced(expected, 4));
    let craft = manager.network_entity(id).unwrap();
    assert_eq!(craft.entity_type, REMOTE_CRAFT_TYPE);
    assert!(craft.is_network_remote());
    assert_eq!(
        craft
            .collision
            .state_flags_at_0x08
            .masked(REMOTE_STATE_BIT | 0x0600_0000),
        RetailRuntimeValue::Known(REMOTE_STATE_BIT | 0x0600_0000)
    );
    assert_eq!(craft.model_slots, [41, 67, 41, 41].map(Some));
    assert_eq!(
        craft.collision.health_raw,
        RetailRuntimeValue::Known(40_000)
    );
    assert_eq!(
        craft.collision.pair_callbacks.damage_modifier_address,
        RetailRuntimeValue::Known(None)
    );
    assert!(craft.intro2_flyer_frame_owner.is_some());
    let RetailRuntimeValue::Known(Some(context)) = craft.current_behavior_context else {
        panic!("class 7 chosen at birth");
    };
    let crate::entity_behavior::ActiveBehaviorStyle::Audited(style) = context.active_style() else {
        panic!("audited style");
    };
    assert_eq!(style.class_id, 7);
}

/// A particle's hit on a peer's craft runs `FUN_00410EB0` on the copy:
/// class 7's `FUN_0040C690` chooses again (three words), `FUN_00411030`
/// jolts the copy (three more) and sends the same deltas (kind 10), then
/// the damage goes to the owner (kind 9).
#[v2k_test_support::retail_test]
fn a_hit_on_a_peers_craft_jolts_its_copy_and_sends_kind_10_then_kind_9() {
    let (session, metadata) = arena_world();
    let mut fx = WorldFx::new();
    let mut manager = arena_manager(&session, &metadata, &mut fx);
    let id = peer_craft(&session, &mut manager, &mut fx);
    let player = manager.player().unwrap().id;
    fx.set_network_sends(true);
    let expected = fx.fork_for_main_base_abort_transaction();
    let impact = crate::world_fx::ParticleEntityImpact {
        source_particle_class: 2,
        impact_position_argument_va: 0,
        target_entity_id: id,
        position_world: [0x50 as f32, 6.0, 0x50 as f32],
        velocity_raw: [0x4000, 0, 0],
        damage: Some(crate::world_fx::BallisticDamageRequest {
            packet: crate::damage::DamagePacket {
                channels: [2, 0],
                amounts_raw: [2000, 0],
            },
            source_entity_type_at_birth: Some(46),
            source_owner_id: Some(player),
        }),
        no_packet: false,
    };
    let mut notifications = crate::gameplay_notifications::GameplayNotifications::new();
    let outcome = crate::intro2_flyer_impact::apply_peer_craft_particle_hit(
        &mut manager,
        &session.cache,
        &mut fx,
        &mut notifications,
        7,
        impact,
    );
    let crate::intro2_flyer_impact::PeerCraftImpactOutcome::Applied {
        filtered_damage_raw,
    } = outcome
    else {
        panic!("{outcome:?}");
    };
    assert!(filtered_damage_raw > 0);
    assert_eq!(fx.next_shared_retail_random_u16(), advanced(expected, 6));
    let sends = fx.take_network_sends();
    let [crate::world_fx::NetworkSend::Impact { entity_id, deltas }, crate::world_fx::NetworkSend::Damage {
        entity_id: damaged,
        amount,
        attacker_type: 46,
    }] = sends[..]
    else {
        panic!("{sends:?}");
    };
    assert_eq!((entity_id, damaged), (id, id));
    assert_eq!(i32::from(amount), filtered_damage_raw);
    let craft = manager.network_entity(id).unwrap();
    // The copy moves by what the owner is sent.
    assert_eq!(
        craft.network_body().unwrap().velocity_raw,
        [deltas[0], deltas[1], deltas[2]]
    );
    assert_eq!(craft.velocity_raw(), [deltas[0], deltas[1], deltas[2]]);
    assert_eq!(
        craft.rotation_heading_pitch_roll_raw(),
        [deltas[3], deltas[4], deltas[5]]
    );
    assert!(deltas[0] > 0);
    assert_eq!(
        craft.collision.last_hit_presentation_tick_at_0x34,
        RetailRuntimeValue::Known(7)
    );
}

/// Arena 1 (world 13) as a client builds it: no authored bodies.
fn client_arena_one(fx: &mut WorldFx) -> (crate::session::GameSession, EntityManager) {
    let mut session = crate::session::GameSession::init(&v2k_test_support::retail_dir()).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(25, 1).unwrap();
    let metadata: Vec<_> = (0..session.cache.global_entity_model_table().len())
        .map(|id| {
            EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(id).unwrap())
        })
        .collect();
    let mut level = session.cache.level_desc().unwrap().clone();
    level.entities.clear();
    let manager = EntityManager::from_authored_world(
        AuthoredWorldConstruction {
            level: &level,
            logical_world_index: 13,
            type_metadata: &metadata,
            resources: EntityConstructionResources {
                terrain: session.cache.terrain(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&|id| session.cache.global_model(id).map(|m| m.radius)),
            },
            player_arrival: Some(AuthoredPlayerArrival {
                position_raw: [0x4000, 0x400, 0x4000],
                heading_raw: 0,
            }),
            retail_tick: 0,
        },
        fx,
    )
    .unwrap();
    (session, manager)
}

/// A type a host sends, at the position of its first authored record in
/// arena 1 (a birth's otherwise).
fn peer_copy(
    session: &crate::session::GameSession,
    manager: &mut EntityManager,
    fx: &mut WorldFx,
    entity_type: u32,
) -> PeerBirth {
    let position = session
        .cache
        .level_desc()
        .unwrap()
        .entities
        .iter()
        .find(|spawn| spawn.entity_type == entity_type)
        .map_or([0x5000, 0x600, 0x5000], |spawn| spawn.position_raw());
    manager
        .append_network_body(
            entity_type,
            Some(position),
            Some(PeerBirthWorld {
                terrain: session.cache.terrain().unwrap(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&|id| session.cache.global_model(id).map(|m| m.radius)),
                world_fx: fx,
                retail_tick: 0,
            }),
        )
        .unwrap()
}

/// `FUN_00417680`'s birth is `FUN_00438080` with a record that holds only
/// the handle, the type and the position: the constructor `FUN_0042E440`
/// runs for an authored record. Every arena-1 type the port builds by its
/// constructor is built so (state bit 31 from `FUN_004104B0`, its own
/// components and behavior); the Hive and the Main Base stay held.
#[v2k_test_support::retail_test]
fn a_peers_copies_are_built_from_a_zeroed_record() {
    let mut fx = WorldFx::new();
    let (session, mut manager) = client_arena_one(&mut fx);
    for entity_type in [
        8, 9, 15, 39, 61, 66, 68, 71, 79, 82, 90, 91, 102, 103, 116, 126, 128,
    ] {
        let mut before = fx.fork_for_main_base_abort_transaction();
        let birth = peer_copy(&session, &mut manager, &mut fx, entity_type);
        let PeerBirth::Native(id) = birth else {
            panic!("Type{entity_type}: {birth:?}");
        };
        let copy = manager.network_entity(id).unwrap();
        assert_eq!(copy.entity_type, entity_type);
        assert!(copy.is_network_remote());
        assert_eq!(
            copy.collision.state_flags_at_0x08.masked(REMOTE_STATE_BIT),
            RetailRuntimeValue::Known(REMOTE_STATE_BIT)
        );
        assert!(copy.authored_spawn_index >= Some(PEER_COPY_RECORD_INDEX));
        assert!(
            copy.current_behavior_context != RetailRuntimeValue::Unresolved,
            "Type{entity_type} behavior"
        );
        // AC60's selector at least: the constructor drew shared words.
        assert_ne!(
            fx.fork_for_main_base_abort_transaction()
                .next_shared_retail_random_u16(),
            before.next_shared_retail_random_u16(),
            "Type{entity_type} draws"
        );
    }
    let hive = peer_copy(&session, &mut manager, &mut fx, 67);
    assert!(
        matches!(&hive, PeerBirth::Hollow { reason, .. } if reason == "its constructor is not ported"),
        "{hive:?}"
    );
}

/// A hollow copy carries retail's zeroed null-template Sub-M instead of
/// unresolved words, so a later worker copy's Job Nearby search skips it
/// the way retail's null template does instead of stopping the whole
/// construction on unknown state.
#[v2k_test_support::retail_test]
fn a_hollow_copy_does_not_poison_a_worker_copy() {
    let mut fx = WorldFx::new();
    let (session, mut manager) = client_arena_one(&mut fx);
    for entity_type in [60, 67] {
        let PeerBirth::Hollow { id, .. } = peer_copy(&session, &mut manager, &mut fx, entity_type)
        else {
            panic!("Type{entity_type} stays hollow");
        };
        let copy = manager.network_entity(id).unwrap();
        assert!(
            matches!(copy.base_factory_runtime, RetailRuntimeValue::Known(_)),
            "Type{entity_type} resolved Sub-M"
        );
    }
    for entity_type in [79, 90, 91, 116] {
        let birth = peer_copy(&session, &mut manager, &mut fx, entity_type);
        let PeerBirth::Native(id) = birth else {
            panic!("Type{entity_type}: {birth:?}");
        };
        let copy = manager.network_entity(id).unwrap();
        assert_eq!(copy.entity_type, entity_type);
        assert!(copy.is_network_remote());
        assert!(
            copy.current_behavior_context != RetailRuntimeValue::Unresolved,
            "Type{entity_type} behavior"
        );
    }
}

/// `FUN_00438080` with the null template for a Main Base: the zeroed Sub-M
/// keeps its words but for +50 (word 20), which becomes the `FUN_0044C830`
/// voice 0x1F at gain 0. The copy carries its Sub-M (control and scientists
/// zero); its behavior is chosen at birth and it draws shared words.
#[v2k_test_support::retail_test]
fn a_peers_main_base_copy_takes_the_null_template() {
    let mut fx = WorldFx::new();
    let (session, mut manager) = client_arena_one(&mut fx);
    let mut before = fx.fork_for_main_base_abort_transaction();
    let birth = peer_copy(&session, &mut manager, &mut fx, 6);
    let PeerBirth::Native(id) = birth else {
        panic!("Type6: {birth:?}");
    };
    let copy = manager.network_entity(id).unwrap();
    assert_eq!(copy.entity_type, 6);
    assert!(copy.is_network_remote());
    assert_eq!(
        copy.collision.state_flags_at_0x08.masked(REMOTE_STATE_BIT),
        RetailRuntimeValue::Known(REMOTE_STATE_BIT)
    );
    assert!(copy.authored_spawn_index >= Some(PEER_COPY_RECORD_INDEX));
    assert!(
        copy.current_behavior_context != RetailRuntimeValue::Unresolved,
        "Type6 behavior"
    );
    assert!(
        copy.main_base_runtime.is_some(),
        "Type6 native Main Base owner"
    );
    let RetailRuntimeValue::Known(Some(sub_m)) = &copy.base_factory_runtime else {
        panic!("Type6 Sub-M");
    };
    assert_eq!((sub_m.control_value_raw, sub_m.required_scientists), (0, 0));
    // AC60's selector at least: the constructor drew shared words.
    assert_ne!(
        fx.fork_for_main_base_abort_transaction()
            .next_shared_retail_random_u16(),
        before.next_shared_retail_random_u16(),
        "Type6 draws"
    );
}

/// `FUN_004381F0` hands `FUN_00418A90` a null template: the zeroed Sub-M
/// keeps no scientists, output or timers, and only the primary
/// `FUN_0044C830` voice (0x1F, at gain 0).
#[v2k_test_support::retail_test]
fn a_peers_factory_copy_takes_the_null_template() {
    let mut fx = WorldFx::new();
    let (session, mut manager) = client_arena_one(&mut fx);
    let PeerBirth::Native(id) = peer_copy(&session, &mut manager, &mut fx, 66) else {
        panic!("Type66");
    };
    let RetailRuntimeValue::Known(Some(factory)) =
        manager.network_entity(id).unwrap().base_factory_runtime
    else {
        panic!("Sub-M");
    };
    assert_eq!(
        (factory.control_value_raw, factory.required_scientists),
        (0, 0)
    );
    let production = factory.production.unwrap();
    assert_eq!(
        production
            .primary_voice
            .map(|voice| (voice.sound_id, voice.gain_raw_16_16)),
        Some((0x1F, 0))
    );
    assert_eq!(production.secondary_voice, None);
}

/// `FUN_00412DA0` runs none of a peer's body's callbacks: no task owner
/// adopts its copy.
#[v2k_test_support::retail_test]
fn no_task_owner_adopts_a_peers_copy() {
    let mut fx = WorldFx::new();
    let (session, mut manager) = client_arena_one(&mut fx);
    for entity_type in [8, 66] {
        assert!(matches!(
            peer_copy(&session, &mut manager, &mut fx, entity_type),
            PeerBirth::Native(_)
        ));
    }
    let mut tasks = crate::specialized_actor_task_production::SpecializedActorTaskScheduler::new();
    assert_eq!(tasks.adopt_intro2_type8(&mut manager), 0);
    assert_eq!(tasks.adopt_live_type8_go_to_job(&manager), 0);
    assert_eq!(tasks.adopt_native_type86(&mut manager), 0);
    assert_eq!(tasks.adopt_level_one_factory_arrival(&manager), 0);
}

/// A hit on a dying peer's craft: class 7's `FUN_0040C690` reaches
/// `FUN_0040AC60` with the dying bit, which installs the craft's alternate,
/// class 25, whose `FUN_00447280` finds no controller for the copy and draws
/// nothing; `FUN_00411030`'s jolt and the damage follow as for a live one.
#[v2k_test_support::retail_test]
fn a_hit_on_a_dying_peers_craft_takes_class_25() {
    let (session, metadata) = arena_world();
    let mut fx = WorldFx::new();
    let mut manager = arena_manager(&session, &metadata, &mut fx);
    let id = peer_craft(&session, &mut manager, &mut fx);
    let player = manager.player().unwrap().id;
    manager.apply_network_sync(
        id,
        &BodySync {
            flags: 0x8000 | 0x4000 | 0x80,
            model_variables: vec![1, 2, 3, 4, 5, 6, 7],
            ..BodySync::default()
        },
        None,
    );
    fx.set_network_sends(true);
    let expected = fx.fork_for_main_base_abort_transaction();
    let impact = crate::world_fx::ParticleEntityImpact {
        source_particle_class: 2,
        impact_position_argument_va: 0,
        target_entity_id: id,
        position_world: [0x50 as f32, 6.0, 0x50 as f32],
        velocity_raw: [0x4000, 0, 0],
        damage: Some(crate::world_fx::BallisticDamageRequest {
            packet: crate::damage::DamagePacket {
                channels: [2, 0],
                amounts_raw: [2000, 0],
            },
            source_entity_type_at_birth: Some(46),
            source_owner_id: Some(player),
        }),
        no_packet: false,
    };
    let mut notifications = crate::gameplay_notifications::GameplayNotifications::new();
    let outcome = crate::intro2_flyer_impact::apply_peer_craft_particle_hit(
        &mut manager,
        &session.cache,
        &mut fx,
        &mut notifications,
        7,
        impact,
    );
    assert!(
        matches!(
            outcome,
            crate::intro2_flyer_impact::PeerCraftImpactOutcome::Applied { .. }
        ),
        "{outcome:?}"
    );
    // Only the jolt's three words: class 25 draws none.
    assert_eq!(fx.next_shared_retail_random_u16(), advanced(expected, 3));
    let craft = manager.network_entity(id).unwrap();
    let RetailRuntimeValue::Known(Some(context)) = craft.current_behavior_context else {
        panic!("class 25");
    };
    let crate::entity_behavior::ActiveBehaviorStyle::Audited(style) = context.active_style() else {
        panic!("audited style");
    };
    assert_eq!(style.class_id, 25);
    assert!(crate::actor_task_owner::ActorTaskSlot::IN_RETAIL_TICK_ORDER
        .into_iter()
        .all(|slot| craft.actor_task_state(slot).is_none()));
    assert_eq!(
        craft.network_body().unwrap().model_variables,
        vec![1, 2, 3, 4, 5, 6, 0]
    );
    let sends = fx.take_network_sends();
    assert!(
        matches!(
            sends[..],
            [
                crate::world_fx::NetworkSend::Impact { .. },
                crate::world_fx::NetworkSend::Damage { .. }
            ]
        ),
        "{sends:?}"
    );
}
