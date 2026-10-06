//!451C00 over real ordinary-world constructors and native Type68 task owners.

use v2k_game::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    common_mover::type9_attitude::Type9BodyBasis,
    entity::{
        AuthoredPlayerArrival, AuthoredWorldConstruction, BeamCommand, BeamOutcome,
        CampaignCargoControllerState, CampaignCargoIdentity, CampaignCargoIdentityUnresolved,
        CampaignCargoRestoreContext, CampaignCargoRestoreError, CargoDropContext, CargoProxyEvent,
        Class0ActorOwner, EntityConstructionResources, EntityManager, LateTailMaterialiserFrame,
        PlayerCargoAttachmentError, PlayerCargoAttachmentFrame, PlayerCargoFrame,
    },
    entity_collision_state::{
        EntityTypeRuntimeMetadata, RetailRuntimeValue, DYING_STATE_BIT, SURFACE_STATE_MASK,
    },
    gameplay_notifications::GameplayNotifications,
    session::GameSession,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    world_fx::{ParticleBirthContext, ParticleEnvironment, TerrainCollisionContext, WorldFx},
};

struct World {
    session: GameSession,
    entities: EntityManager,
    scheduler: SpecializedActorTaskScheduler,
    fx: WorldFx,
    notifications: GameplayNotifications,
}

impl World {
    fn new(level_id: u32) -> Self {
        let data = v2k_test_support::retail_dir();
        assert!(data.join("PRELOAD.DAT").exists(), "retail corpus required");
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
        let mut fx = WorldFx::new();
        let extent = |id| session.cache.global_model(id).map(|model| model.radius);
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
                    position_raw: [19_712, -500, 14_848],
                    heading_raw: 0x4000,
                }),
                retail_tick: 4793,
            },
            &mut fx,
        )
        .unwrap();
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler.adopt_class0_actors(&entities);
        Self {
            session,
            entities,
            scheduler,
            fx,
            notifications: GameplayNotifications::new(),
        }
    }

    fn weight(&self) -> u32 {
        self.entities
            .iter_all()
            .find(|entity| entity.entity_type == 68)
            .unwrap()
            .id
    }

    fn snapshot(&self) -> CampaignCargoControllerState {
        match self.entities.campaign_cargo_controller_state() {
            RetailRuntimeValue::Known(state) => state,
            RetailRuntimeValue::Unresolved => panic!("native controller state"),
        }
    }

    fn attach(&mut self, id: u32) {
        self.entities
            .attach_player_cargo(
                id,
                PlayerCargoAttachmentFrame {
                    scheduler: &mut self.scheduler,
                    world_fx: &mut self.fx,
                    notifications: &mut self.notifications,
                    retail_tick: 4793,
                },
            )
            .unwrap();
    }

    fn restore(
        &mut self,
        state: CampaignCargoControllerState,
    ) -> Result<(), CampaignCargoRestoreError> {
        let extent = |id| {
            self.session
                .cache
                .global_model(id)
                .map(|model| model.radius)
        };
        self.entities.restore_campaign_cargo_controller_state(
            state,
            CampaignCargoRestoreContext {
                level: self.session.cache.level_desc().unwrap(),
                resources: EntityConstructionResources {
                    terrain: self.session.cache.terrain(),
                    terrain_objects: self.session.cache.terrain_objects(),
                    model_extent_raw: Some(&extent),
                },
                retail_tick: 4793,
                scheduler: &mut self.scheduler,
                world_fx: &mut self.fx,
                notifications: &mut self.notifications,
            },
        )
    }

    fn attached(&self) -> Vec<u32> {
        let RetailRuntimeValue::Known(Some(runtime)) =
            &self.entities.player().unwrap().sub_j_attachment_runtime
        else {
            panic!("player Sub-J")
        };
        runtime.ordered_entity_ids().to_vec()
    }
}

fn packed(world: &World, id: u32) -> u32 {
    let entity = world
        .entities
        .iter_all()
        .find(|entity| entity.id == id)
        .unwrap();
    let RetailRuntimeValue::Known(word) = CampaignCargoIdentity::from_entity_parts(
        entity.entity_type,
        entity.construction_stamp_at_0xb4,
    )
    .packed_word() else {
        panic!("native identity")
    };
    word
}

#[v2k_test_support::retail_test]
fn same_world_reuses_authored_identity_and_preserves_mutable_actor_state() {
    let mut source = World::new(13);
    let source_weight = source.weight();
    source.attach(source_weight);
    let state = source.snapshot();
    let mut destination = World::new(13);
    let weight = destination.weight();
    let owner = Class0ActorOwner::adopt(&destination.entities, weight).unwrap();
    destination
        .entities
        .entity_mut(weight)
        .unwrap()
        .collision
        .health_raw = RetailRuntimeValue::Known(12345);
    let before_position = destination
        .entities
        .entity_mut(weight)
        .unwrap()
        .position_raw();
    let before_count = destination.entities.iter_all().count();
    let before_counter = destination.entities.next_common_body_ordinal();
    destination.fx = WorldFx::new();
    let mut no_draw = WorldFx::new();
    destination.restore(state).unwrap();
    assert_eq!(destination.attached(), [weight]);
    assert_eq!(destination.entities.iter_all().count(), before_count);
    assert_eq!(
        destination.entities.next_common_body_ordinal(),
        before_counter
    );
    assert_eq!(
        destination.fx.next_shared_retail_random_u16(),
        no_draw.next_shared_retail_random_u16()
    );
    assert_eq!(
        Class0ActorOwner::adopt(&destination.entities, weight),
        Ok(owner)
    );
    let entity = destination.entities.entity_mut(weight).unwrap();
    assert_eq!(
        entity.collision.health_raw,
        RetailRuntimeValue::Known(12345)
    );
    assert_eq!(entity.position_raw(), before_position);
    assert!(entity.authored_spawn_index.is_some());
    assert!(matches!(
        entity.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::Class0Timer(_))
    ));
}

#[v2k_test_support::retail_test]
fn foreign_world_miss_constructs_at_zero_then_overwrites_only_saved_stamp() {
    let mut source = World::new(13);
    let source_weight = source.weight();
    let saved = packed(&source, source_weight);
    source
        .entities
        .entity_mut(source_weight)
        .unwrap()
        .collision
        .health_raw = RetailRuntimeValue::Known(7);
    source.attach(source_weight);
    let state = source.snapshot();
    let mut destination = World::new(14);
    let authored = destination
        .entities
        .iter_all()
        .map(|entity| entity.id)
        .collect::<Vec<_>>();
    let RetailRuntimeValue::Known(before_counter) = destination.entities.next_common_body_ordinal()
    else {
        panic!()
    };
    destination.fx = WorldFx::new();
    let mut one_draw = WorldFx::new();
    one_draw.next_shared_retail_random_u16();
    destination.restore(state).unwrap();
    let ids = destination.attached();
    assert_eq!(ids.len(), 1);
    assert!(!authored.contains(&ids[0]));
    assert_eq!(
        destination.entities.next_common_body_ordinal(),
        RetailRuntimeValue::Known(before_counter + 1)
    );
    assert_eq!(
        destination.fx.next_shared_retail_random_u16(),
        one_draw.next_shared_retail_random_u16()
    );
    let origin_ground = destination
        .session
        .cache
        .terrain()
        .unwrap()
        .bilinear_height_raw(0, 0);
    let child = destination.entities.entity_mut(ids[0]).unwrap();
    assert_eq!(child.position_raw(), [0, origin_ground, 0]);
    assert_eq!(child.velocity_raw(), [0; 3]);
    assert_eq!(child.rotation_heading_pitch_roll_raw(), [0; 3]);
    assert_eq!(child.authored_spawn_index, None);
    assert_eq!(child.model_slots, [Some(81); 4]);
    assert_eq!(child.collision.health_raw, RetailRuntimeValue::Known(50000));
    assert_eq!(
        child.construction_stamp_at_0xb4,
        RetailRuntimeValue::Known((saved >> 16) as u16)
    );
    assert!(Class0ActorOwner::adopt(&destination.entities, ids[0]).is_ok());
    assert_eq!(
        destination.snapshot().carried_identities()[0].packed_word(),
        RetailRuntimeValue::Known(saved)
    );
    destination
        .fx
        .process_deferred_cargo_transfers(ParticleBirthContext {
            environment: ParticleEnvironment::Terrain(
                TerrainCollisionContext::from_current_level_cache(&destination.session.cache)
                    .unwrap(),
            ),
            retail_tick: 4793,
        });
    destination.fx.process_pending();
    assert_eq!(
        destination
            .fx
            .take_positional_sounds()
            .iter()
            .filter(|sound| sound.sound_id == 8)
            .count(),
        1
    );
}

#[v2k_test_support::retail_test]
fn negative_saved_high_half_cannot_reuse_identical_unsigned_body_stamp() {
    let mut world = World::new(13);
    let original = world.weight();
    world
        .entities
        .entity_mut(original)
        .unwrap()
        .construction_stamp_at_0xb4 = RetailRuntimeValue::Known(0x8001);
    let before_count = world.entities.iter_all().count();
    world
        .restore(CampaignCargoControllerState::from_packed(5, [0x8001_0044]))
        .unwrap();
    assert_eq!(world.entities.iter_all().count(), before_count + 1);
    assert_ne!(world.attached()[0], original);
    assert_eq!(
        world
            .entities
            .entity_mut(world.attached()[0])
            .unwrap()
            .construction_stamp_at_0xb4,
        RetailRuntimeValue::Known(0x8001)
    );
    assert_eq!(
        world.entities.entity_mut(original).unwrap().attached_to,
        None
    );
}

#[v2k_test_support::retail_test]
fn capacity_failure_retains_earlier_attachment_and_committed_newborn() {
    let mut world = World::new(13);
    let existing = world.weight();
    let word = packed(&world, existing);
    let before_count = world.entities.iter_all().count();
    let RetailRuntimeValue::Known(before_counter) = world.entities.next_common_body_ordinal()
    else {
        panic!()
    };
    let error = world
        .restore(CampaignCargoControllerState::from_packed(
            1,
            [word, 0x7777_0044],
        ))
        .unwrap_err();
    let CampaignCargoRestoreError::Attachment {
        entity_id: newborn,
        reason: PlayerCargoAttachmentError::CargoFull,
    } = error
    else {
        panic!("{error:?}")
    };
    assert_eq!(world.attached(), [existing]);
    assert_eq!(world.entities.iter_all().count(), before_count + 1);
    assert_eq!(
        world.entities.next_common_body_ordinal(),
        RetailRuntimeValue::Known(before_counter + 1)
    );
    let child = world.entities.entity_mut(newborn).unwrap();
    assert_eq!(child.attached_to, None);
    assert_eq!(
        child.construction_stamp_at_0xb4,
        RetailRuntimeValue::Known(0x7777)
    );
    assert!(Class0ActorOwner::adopt(&world.entities, newborn).is_ok());
}

#[v2k_test_support::retail_test]
fn duplicate_match_checks_full_before_duplicate_and_never_allocates() {
    for capacity in [1, 5] {
        let mut world = World::new(13);
        let id = world.weight();
        let word = packed(&world, id);
        let counter = world.entities.next_common_body_ordinal();
        let result = world.restore(CampaignCargoControllerState::from_packed(
            capacity,
            [word, word],
        ));
        if capacity == 1 {
            assert!(matches!(
                result,
                Err(CampaignCargoRestoreError::Attachment {
                    reason: PlayerCargoAttachmentError::CargoFull,
                    ..
                })
            ));
        } else {
            result.unwrap();
        }
        assert_eq!(world.attached(), [id]);
        assert_eq!(world.entities.next_common_body_ordinal(), counter);
    }
}

#[v2k_test_support::retail_test]
fn excluded_types_and_first_zero_do_not_enter_constructor_or_match() {
    let mut world = World::new(13);
    let counter = world.entities.next_common_body_ordinal();
    world
        .restore(CampaignCargoControllerState::from_packed(
            1,
            [7, 8, 9, 78, 79, 86, 90, 91, 95, 116, 0xabcd_0000, 68],
        ))
        .unwrap();
    assert!(world.attached().is_empty());
    assert_eq!(world.entities.next_common_body_ordinal(), counter);
}

#[v2k_test_support::retail_test]
fn unknown_identity_and_unimplemented_miss_do_not_guess_births() {
    let mut source = World::new(13);
    let id = source.weight();
    source.attach(id);
    source
        .entities
        .entity_mut(id)
        .unwrap()
        .construction_stamp_at_0xb4 = RetailRuntimeValue::Unresolved;
    let mut world = World::new(14);
    let counter = world.entities.next_common_body_ordinal();
    assert!(matches!(
        world.restore(source.snapshot()),
        Err(CampaignCargoRestoreError::Identity {
            reason: CampaignCargoIdentityUnresolved::SavedStamp,
            ..
        })
    ));
    assert_eq!(world.entities.next_common_body_ordinal(), counter);
    assert!(matches!(
        world.restore(CampaignCargoControllerState::from_packed(5, [0x7777_004a])),
        Err(CampaignCargoRestoreError::UnsupportedConstructor { entity_type: 74 })
    ));
    assert_eq!(world.entities.next_common_body_ordinal(), counter);
    assert!(world.attached().is_empty());
}

#[v2k_test_support::retail_test]
fn surface_flags_precede_arrival_clearance_and_keep_strict_equality() {
    for (delta, expected_bits) in [(-1, 0x200000), (0, 0), (1, 0x400000)] {
        let mut world = World::new(17);
        assert_eq!(
            world.session.cache.level_desc().unwrap().raw_u32(0x84),
            Some(0)
        );
        let terrain = world.session.cache.terrain().unwrap();
        let sea = terrain.sea_level_raw();
        let clearance = terrain.bilinear_height_raw(0, 0) + 512;
        let player = world.entities.player().unwrap().id;
        world.entities.entity_mut(player).unwrap().position =
            [0.0, f32::from(sea + delta) / 256.0, 0.0];
        world
            .restore(CampaignCargoControllerState::from_packed(5, []))
            .unwrap();
        let player = world.entities.player().unwrap();
        assert_eq!(
            player
                .collision
                .state_flags_at_0x08
                .masked(SURFACE_STATE_MASK),
            RetailRuntimeValue::Known(expected_bits)
        );
        assert_eq!(player.position_raw()[1], (sea + delta).max(clearance));
    }
}

#[v2k_test_support::retail_test]
fn style_six_adds_one_native_ballast_even_with_empty_profile() {
    // Locate through the actual descriptor rather than encoding a level
    // number as the world-style condition.
    let mut world = (13..=49)
        .map(World::new)
        .find(|world| world.session.cache.level_desc().unwrap().world_style == 6)
        .unwrap();
    let RetailRuntimeValue::Known(counter) = world.entities.next_common_body_ordinal() else {
        panic!()
    };
    world
        .restore(CampaignCargoControllerState::from_packed(5, []))
        .unwrap();
    let child = world.attached()[0];
    assert_eq!(world.attached().len(), 1);
    assert_eq!(world.entities.entity_mut(child).unwrap().mass_raw, 200);
    assert_eq!(
        world
            .entities
            .entity_mut(child)
            .unwrap()
            .authored_spawn_index,
        None
    );
    assert_eq!(
        world.entities.next_common_body_ordinal(),
        RetailRuntimeValue::Known(counter + 1)
    );
    assert!(Class0ActorOwner::adopt(&world.entities, child).is_ok());
    // Existing200 mass suppresses any second ballast; an empty saved profile
    // does not delete the already committed direct attachment.
    world
        .restore(CampaignCargoControllerState::from_packed(5, []))
        .unwrap();
    assert_eq!(world.attached(), [child]);
    assert_eq!(
        world.entities.next_common_body_ordinal(),
        RetailRuntimeValue::Known(counter + 1)
    );
}

#[v2k_test_support::retail_test]
fn every_ordinary_world_completes_the_native_empty_profile_load_phase() {
    let mut ballast_worlds = Vec::new();
    for level_id in 13..=49 {
        let mut world = World::new(level_id);
        let before_count = world.entities.iter_all().count();
        let RetailRuntimeValue::Known(before_counter) = world.entities.next_common_body_ordinal()
        else {
            panic!("world{level_id} native counter")
        };
        let unlock = world.entities.player_cargo_unlock_raw();
        world
            .restore(CampaignCargoControllerState::from_packed(unlock, []))
            .unwrap_or_else(|error| panic!("world{level_id} load restoration: {error:?}"));
        if world.session.cache.level_desc().unwrap().world_style == 6 {
            ballast_worlds.push(level_id);
            assert_eq!(world.attached().len(), 1, "world{level_id}");
            let ballast = world.attached()[0];
            assert!(Class0ActorOwner::adopt(&world.entities, ballast).is_ok());
            assert_eq!(
                world
                    .entities
                    .entity_mut(ballast)
                    .unwrap()
                    .construction_stamp_at_0xb4,
                RetailRuntimeValue::Known(
                    (((level_id - 12) << 10) as u16).wrapping_add(before_counter)
                )
            );
            assert_eq!(world.entities.iter_all().count(), before_count + 1);
            assert_eq!(
                world.entities.next_common_body_ordinal(),
                RetailRuntimeValue::Known(before_counter.wrapping_add(1))
            );
        } else {
            assert!(world.attached().is_empty(), "world{level_id}");
            assert_eq!(world.entities.iter_all().count(), before_count);
            assert_eq!(
                world.entities.next_common_body_ordinal(),
                RetailRuntimeValue::Known(before_counter)
            );
        }
    }
    assert!(
        !ballast_worlds.is_empty(),
        "normal-tier corpus reaches the ballast path"
    );
}

#[v2k_test_support::retail_test]
fn first_dying_match_retains_callback_prefix_and_never_substitutes_later_match() {
    let mut world = World::new(25);
    let weights = world
        .entities
        .iter_all()
        .filter(|entity| entity.entity_type == 68)
        .map(|entity| entity.id)
        .collect::<Vec<_>>();
    assert!(weights.len() >= 2);
    let word = packed(&world, weights[0]);
    world
        .entities
        .entity_mut(weights[1])
        .unwrap()
        .construction_stamp_at_0xb4 = RetailRuntimeValue::Known((word >> 16) as u16);
    world
        .entities
        .entity_mut(weights[0])
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(DYING_STATE_BIT, DYING_STATE_BIT);
    let counter = world.entities.next_common_body_ordinal();
    let result = world.restore(CampaignCargoControllerState::from_packed(5, [word]));
    assert!(
        matches!(result, Err(CampaignCargoRestoreError::Attachment {
        entity_id,
        reason: PlayerCargoAttachmentError::CallbackUnavailable { prefix_committed: true, .. },
    }) if entity_id == weights[0]),
        "{result:?}"
    );
    assert_eq!(world.attached(), [weights[0]]);
    assert!(world
        .entities
        .entity_mut(weights[1])
        .unwrap()
        .attached_to
        .is_none());
    assert_eq!(world.entities.next_common_body_ordinal(), counter);
}

#[v2k_test_support::retail_test]
fn reconstructed_weight_drops_settles_and_can_be_collected_again() {
    let mut source = World::new(13);
    let weight = source.weight();
    source.attach(weight);
    let saved = source.snapshot();
    let mut world = World::new(14);
    world.restore(saved.clone()).unwrap();
    let weight = world.attached()[0];
    let stamp = world
        .entities
        .entity_mut(weight)
        .unwrap()
        .construction_stamp_at_0xb4;
    let player_id = world.entities.player().unwrap().id;
    let position = world.entities.entity_mut(weight).unwrap().position;
    let x = position[0] + 2.0;
    let ground = world
        .session
        .cache
        .terrain()
        .unwrap()
        .height_at(x, position[2]);
    world.entities.entity_mut(player_id).unwrap().position =
        [x, ground - 1.0, position[2] + 400.0 / 256.0];
    world.entities.queue_beam(BeamCommand::Drop);
    let mut proxy = None;
    let mut released = None;
    for tick in 1..=100 {
        let pass = world.entities.update_player_cargo(PlayerCargoFrame {
            elapsed_micros: 20000,
            drop_context: Some(CargoDropContext {
                terrain: world.session.cache.terrain().unwrap(),
                carrier_orientation: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            }),
            retail_tick: 4793 + tick,
            scheduler: &mut world.scheduler,
            world_fx: &mut world.fx,
            notifications: &mut world.notifications,
        });
        assert!(pass.blocked.is_empty(), "{:?}", pass.blocked);
        if let Some(BeamOutcome::DropStarted { cargo_id, proxy_id }) = pass.beam {
            assert_eq!(cargo_id, weight);
            proxy = Some(proxy_id);
        }
        let blocked = world
            .entities
            .update_late_tail_materialisers(LateTailMaterialiserFrame {
                elapsed_micros: 20000,
                terrain: world.session.cache.terrain().unwrap(),
                world_fx: &mut world.fx,
                scheduler: &mut world.scheduler,
                notifications: &mut world.notifications,
                retail_tick: 4793 + tick,
            });
        assert!(blocked.is_empty(), "{blocked:?}");
        for event in world.entities.take_cargo_proxy_events() {
            if let CargoProxyEvent::Released {
                cargo_id,
                proxy_id,
                position,
            } = event
            {
                assert_eq!(cargo_id, weight);
                assert_eq!(Some(proxy_id), proxy);
                released = Some(position);
            }
        }
        if released.is_some() {
            break;
        }
    }
    let settled = released.expect("native reconstructed weight completes Type93 release");
    assert!(world.attached().is_empty());
    assert!(Class0ActorOwner::adopt(&world.entities, weight).is_ok());
    assert_eq!(
        world
            .entities
            .entity_mut(weight)
            .unwrap()
            .construction_stamp_at_0xb4,
        stamp
    );
    assert_eq!(world.entities.entity_mut(weight).unwrap().attached_to, None);
    world.entities.entity_mut(player_id).unwrap().position =
        [settled[0] + 0.25, settled[1] - 1., settled[2]];
    world.entities.queue_beam(BeamCommand::Collect);
    let mut collected = false;
    for tick in 101..=130 {
        let pass = world.entities.update_player_cargo(PlayerCargoFrame {
            elapsed_micros: 20000,
            drop_context: None,
            retail_tick: 4793 + tick,
            scheduler: &mut world.scheduler,
            world_fx: &mut world.fx,
            notifications: &mut world.notifications,
        });
        assert!(pass.blocked.is_empty(), "{:?}", pass.blocked);
        if let Some(BeamOutcome::Collected { entity_id }) = pass.beam {
            assert_eq!(entity_id, weight);
            collected = true;
            break;
        }
    }
    assert!(
        collected,
        "released native cargo remains usable by the real beam"
    );
    assert_eq!(
        world.snapshot().carried_identities()[0],
        saved.carried_identities()[0]
    );
}

#[v2k_test_support::retail_test]
fn type92_match_attaches_destination_authored_identity() {
    let mut world = World::new(26);
    let tulaz = world
        .entities
        .iter_all()
        .find(|entity| entity.entity_type == 92)
        .expect("overlay 26 authors Type92")
        .id;
    let word = packed(&world, tulaz);
    let before = world.entities.iter_all().count();
    world
        .restore(CampaignCargoControllerState::from_packed(5, [word]))
        .unwrap();
    assert_eq!(world.attached(), [tulaz]);
    assert_eq!(world.entities.iter_all().count(), before);
}

#[v2k_test_support::retail_test]
fn type92_miss_zero_record_constructs_and_attaches() {
    let mut world = World::new(13);
    assert!(world
        .entities
        .iter_all()
        .all(|entity| entity.entity_type != 92));
    let before = world.entities.iter_all().count();
    world
        .restore(CampaignCargoControllerState::from_packed(5, [0x1234_005c]))
        .unwrap();
    let attached = world.attached();
    assert_eq!(attached.len(), 1);
    let cargo = world
        .entities
        .iter_all()
        .find(|entity| entity.id == attached[0])
        .unwrap();
    assert_eq!(cargo.entity_type, 92);
    assert_eq!(
        cargo.construction_stamp_at_0xb4,
        RetailRuntimeValue::Known(0x1234)
    );
    assert!(matches!(
        cargo.actor_task_state(ActorTaskSlot::Tertiary),
        Some(ActorTaskRuntime::Intro2GunTurret(_))
    ));
    assert_eq!(world.entities.iter_all().count(), before + 1);
}

#[v2k_test_support::retail_test]
fn remaining_unrecovered_cargo_and_person_exclusions_stay_explicit() {
    let mut world = World::new(14);
    let counter = world.entities.next_common_body_ordinal();
    assert!(matches!(
        world.restore(CampaignCargoControllerState::from_packed(5, [0x0001_004a])),
        Err(CampaignCargoRestoreError::UnsupportedConstructor { entity_type: 74 })
    ));
    assert!(matches!(
        world.restore(CampaignCargoControllerState::from_packed(5, [0x0001_0053])),
        Err(CampaignCargoRestoreError::UnsupportedConstructor { entity_type: 83 })
    ));
    assert!(matches!(
        world.restore(CampaignCargoControllerState::from_packed(5, [0x0001_006b])),
        Err(CampaignCargoRestoreError::UnsupportedConstructor { entity_type: 107 })
    ));
    assert!(matches!(
        world.restore(CampaignCargoControllerState::from_packed(5, [0x0001_0079])),
        Err(CampaignCargoRestoreError::UnsupportedConstructor { entity_type: 121 })
    ));
    world
        .restore(CampaignCargoControllerState::from_packed(5, [9, 0]))
        .unwrap();
    assert!(world.attached().is_empty());
    assert_eq!(world.entities.next_common_body_ordinal(), counter);
}

#[v2k_test_support::retail_test]
fn auxiliary_0x38_reconstructs_until_whole_zero_without_inspecting_return() {
    let mut world = World::new(14);
    let player = world.entities.player().unwrap().id;
    {
        let player = world.entities.entity_mut(player).unwrap();
        let [heading, _, _] = player.rotation_heading_pitch_roll_raw();
        player.set_rotation_heading_pitch_roll_raw([heading, 0x1111_u16 as i16, 0x2222_u16 as i16]);
        player.set_motion_raw(player.position_raw(), [100, -20, 50]);
    }
    let expected_rotation = world
        .entities
        .player()
        .unwrap()
        .rotation_heading_pitch_roll_raw();
    let expected_velocity = world.entities.player().unwrap().velocity_raw();
    assert_ne!(expected_rotation[1], 0);
    assert_ne!(expected_rotation[2], 0);
    assert_ne!(expected_velocity, [0; 3]);
    let expected_basis = Type9BodyBasis::from_angle_words(
        expected_rotation[0],
        expected_rotation[1],
        expected_rotation[2],
    );
    let before = world.entities.iter_all().count();
    world
        .restore(
            CampaignCargoControllerState::from_packed(5, [0])
                .with_auxiliary_owned_types([52, 49, 0, 92]),
        )
        .unwrap();
    let auxiliary = world.entities.auxiliary_owned_entity_ids();
    assert_eq!(auxiliary.len(), 1);
    let flag = world
        .entities
        .iter_all()
        .find(|entity| entity.id == auxiliary[0])
        .unwrap();
    let player = world.entities.player().unwrap();
    assert_eq!(flag.entity_type, 52);
    assert_eq!(
        flag.collision.recent_relation_id_at_0x60,
        RetailRuntimeValue::Known(Some(player.id))
    );
    assert_eq!(flag.position_raw()[0], player.position_raw()[0]);
    assert_eq!(flag.position_raw()[2], player.position_raw()[2]);
    assert_eq!(flag.rotation_heading_pitch_roll_raw(), expected_rotation);
    assert_eq!(flag.velocity_raw(), expected_velocity);
    assert_eq!(
        flag.physical_body_basis_q31(),
        RetailRuntimeValue::Known(expected_basis)
    );
    assert!(world.attached().is_empty());
    assert_eq!(world.entities.iter_all().count(), before + 1);
}

#[v2k_test_support::retail_test]
fn auxiliary_0x38_enforces_fifteen_object_limit() {
    let mut world = World::new(14);
    // 451C00 runs before the later sweep of 42EFB0's gate-helper removals.
    let pending_before = world.entities.pending_actor_deferred_destroy_ids().to_vec();
    let types: Vec<u32> = std::iter::repeat(52)
        .take(16)
        .chain(std::iter::once(0))
        .collect();
    world
        .restore(
            CampaignCargoControllerState::from_packed(5, [0]).with_auxiliary_owned_types(types),
        )
        .unwrap();
    assert_eq!(world.entities.auxiliary_owned_entity_ids().len(), 15);
    let pending = world.entities.pending_actor_deferred_destroy_ids();
    assert_eq!(&pending[..pending_before.len()], pending_before);
    let [overflow_id] = &pending[pending_before.len()..] else {
        panic!("the sixteenth auxiliary alone must append a deferred removal")
    };
    let overflow = world
        .entities
        .iter_all()
        .find(|entity| entity.id == *overflow_id)
        .unwrap();
    assert_eq!(overflow.entity_type, 52);
    assert!(!world
        .entities
        .auxiliary_owned_entity_ids()
        .contains(overflow_id));
}

fn first_type(world: &World, entity_type: u32) -> u32 {
    world
        .entities
        .iter_all()
        .find(|entity| entity.entity_type == entity_type)
        .unwrap_or_else(|| panic!("authored type{entity_type}"))
        .id
}

#[v2k_test_support::retail_test]
fn type96_match_attaches_destination_authored_identity() {
    let mut world = World::new(26);
    let tulaz = first_type(&world, 96);
    let word = packed(&world, tulaz);
    let before = world.entities.iter_all().count();
    world
        .restore(CampaignCargoControllerState::from_packed(5, [word]))
        .unwrap();
    assert_eq!(world.attached(), [tulaz]);
    assert_eq!(world.entities.iter_all().count(), before);
}

#[v2k_test_support::retail_test]
fn type96_miss_zero_record_constructs_and_attaches() {
    let mut world = World::new(13);
    assert!(world
        .entities
        .iter_all()
        .all(|entity| entity.entity_type != 96));
    let before = world.entities.iter_all().count();
    world
        .restore(CampaignCargoControllerState::from_packed(5, [0x1234_0060]))
        .unwrap();
    let attached = world.attached();
    assert_eq!(attached.len(), 1);
    let cargo = world
        .entities
        .iter_all()
        .find(|entity| entity.id == attached[0])
        .unwrap();
    assert_eq!(cargo.entity_type, 96);
    assert_eq!(
        cargo.construction_stamp_at_0xb4,
        RetailRuntimeValue::Known(0x1234)
    );
    assert_eq!(cargo.model_slots, [Some(175); 4]);
    assert!(matches!(
        cargo.actor_task_state(ActorTaskSlot::Tertiary),
        Some(ActorTaskRuntime::Intro2GunTurret(_))
    ));
    assert_eq!(world.entities.iter_all().count(), before + 1);
}

#[v2k_test_support::retail_test]
fn type100_match_attaches_destination_authored_identity() {
    let mut world = World::new(26);
    let cannon = first_type(&world, 100);
    let word = packed(&world, cannon);
    world
        .restore(CampaignCargoControllerState::from_packed(5, [word]))
        .unwrap();
    assert_eq!(world.attached(), [cannon]);
}

#[v2k_test_support::retail_test]
fn type97_miss_zero_record_constructs_and_attaches() {
    let mut world = World::new(13);
    world
        .restore(CampaignCargoControllerState::from_packed(5, [0x2222_0061]))
        .unwrap();
    let cargo = world
        .entities
        .iter_all()
        .find(|entity| entity.id == world.attached()[0])
        .unwrap();
    assert_eq!(cargo.entity_type, 97);
    assert_eq!(cargo.model_slots, [Some(173); 4]);
    assert_eq!(
        cargo.construction_stamp_at_0xb4,
        RetailRuntimeValue::Known(0x2222)
    );
}

fn restore_123_newborn(world: &mut World) -> u32 {
    world
        .restore(CampaignCargoControllerState::from_packed(5, [0x1234_007b]))
        .unwrap();
    let attached = world.attached();
    assert_eq!(attached.len(), 1);
    let newborn = attached[0];
    let cargo = world
        .entities
        .iter_all()
        .find(|entity| entity.id == newborn)
        .unwrap();
    assert_eq!(cargo.entity_type, 123);
    assert_eq!(
        cargo.construction_stamp_at_0xb4,
        RetailRuntimeValue::Known(0x1234)
    );
    newborn
}

fn assert_type123_living_graph(world: &World, id: u32) {
    let cargo = world
        .entities
        .iter_all()
        .find(|entity| entity.id == id)
        .unwrap();
    // The zero-record birth and CE90 release both run the native
    // Always-weighted root: either the acquiring triple or Wander, never a
    // borrowed graph.
    let primary = cargo.actor_task_state(ActorTaskSlot::Primary);
    let secondary = cargo.actor_task_state(ActorTaskSlot::Secondary);
    let tertiary = cargo.actor_task_state(ActorTaskSlot::Tertiary);
    assert!(
        matches!(
            (primary, secondary, tertiary),
            (
                Some(ActorTaskRuntime::SharedRetarget(_)),
                Some(ActorTaskRuntime::AttractAttentionCandidate(_)),
                Some(ActorTaskRuntime::AttractAttentionCue(_))
            ) | (Some(ActorTaskRuntime::OrdinaryType9Wander(_)), None, None),
        ),
        "{primary:?} {secondary:?} {tertiary:?}"
    );
}

#[v2k_test_support::retail_test]
fn type123_match_attaches_destination_authored_identity() {
    // World-49 authored stamps sit above the signed high-half match bound,
    // so same-world authored matches cannot hit: the first restore misses and
    // reconstructs at a matchable stamp, the beam drops it back to a living
    // graph, and the second restore matches the reconstructed identity.
    // Both halves of the 451C00 cycle run through the same attach owner.
    let mut world = World::new(13);
    assert!(world
        .entities
        .iter_all()
        .all(|entity| entity.entity_type != 123));
    let newborn = restore_123_newborn(&mut world);
    drop_123_cargo(&mut world, newborn);
    let before = world.entities.iter_all().count();
    world
        .restore(CampaignCargoControllerState::from_packed(5, [0x1234_007b]))
        .unwrap();
    assert_eq!(world.attached(), [newborn]);
    assert_eq!(world.entities.iter_all().count(), before);
}

fn drop_123_cargo(world: &mut World, newborn: u32) {
    let player_id = world.entities.player().unwrap().id;
    let position = world.entities.entity_mut(newborn).unwrap().position;
    let x = position[0] + 2.0;
    let ground = world
        .session
        .cache
        .terrain()
        .unwrap()
        .height_at(x, position[2]);
    world.entities.entity_mut(player_id).unwrap().position =
        [x, ground - 1.0, position[2] + 400.0 / 256.0];
    world.entities.queue_beam(BeamCommand::Drop);
    let mut released = None;
    for tick in 1..=100 {
        let pass = world.entities.update_player_cargo(PlayerCargoFrame {
            elapsed_micros: 20000,
            drop_context: Some(CargoDropContext {
                terrain: world.session.cache.terrain().unwrap(),
                carrier_orientation: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            }),
            retail_tick: 4793 + tick,
            scheduler: &mut world.scheduler,
            world_fx: &mut world.fx,
            notifications: &mut world.notifications,
        });
        assert!(pass.blocked.is_empty(), "{:?}", pass.blocked);
        let blocked = world
            .entities
            .update_late_tail_materialisers(LateTailMaterialiserFrame {
                elapsed_micros: 20000,
                terrain: world.session.cache.terrain().unwrap(),
                world_fx: &mut world.fx,
                scheduler: &mut world.scheduler,
                notifications: &mut world.notifications,
                retail_tick: 4793 + tick,
            });
        assert!(blocked.is_empty(), "{blocked:?}");
        for event in world.entities.take_cargo_proxy_events() {
            if let CargoProxyEvent::Released {
                cargo_id, position, ..
            } = event
            {
                assert_eq!(cargo_id, newborn);
                released = Some(position);
            }
        }
        if released.is_some() {
            break;
        }
    }
    released.expect("restored Type123 completes beam release");
    assert!(world.attached().is_empty());
    let cargo = world
        .entities
        .iter_all()
        .find(|entity| entity.id == newborn)
        .unwrap();
    assert_eq!(cargo.attached_to, None);
    assert_type123_living_graph(world, newborn);
}

#[v2k_test_support::retail_test]
fn type123_miss_zero_record_constructs_and_attaches() {
    let mut world = World::new(13);
    assert!(world
        .entities
        .iter_all()
        .all(|entity| entity.entity_type != 123));
    let before = world.entities.iter_all().count();
    let newborn = restore_123_newborn(&mut world);
    let cargo = world
        .entities
        .iter_all()
        .find(|entity| entity.id == newborn)
        .unwrap();
    assert_eq!(cargo.model_slots, [Some(889); 4]);
    assert_eq!(cargo.collision.health_raw, RetailRuntimeValue::Known(1_500));
    // 451C00 attach runs CE70, which only admits a living Wander/Attract
    // graph: the carried None task proves the native root ran first.
    assert!(matches!(
        cargo.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::None)
    ));
    assert_eq!(cargo.actor_task_state(ActorTaskSlot::Secondary), None);
    assert_eq!(cargo.actor_task_state(ActorTaskSlot::Tertiary), None);
    assert_eq!(
        cargo.attached_to,
        world.entities.player().map(|player| player.id)
    );
    assert_eq!(world.entities.iter_all().count(), before + 1);
    // The restore adopts the newborn like the main-flow sweep would, and any
    // BA40 receipt drains with the authored ones below.
    assert_eq!(world.scheduler.adopt_native_type123(&mut world.entities), 0);
    world
        .notifications
        .drain_fresh_level1_type9_attract_attention_receipts(&mut world.entities)
        .unwrap();
}
