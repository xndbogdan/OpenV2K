use v2k_game::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    entity::{EntityConstructionResources, EntityManager},
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue, RetailStateWord},
    intro2_meteors::{
        finish_intro2_meteor_death, publish_intro2_meteor, resolve_intro2_meteor_terrain_contact,
        tick_intro2_meteor, Intro2MeteorFrame, Intro2MeteorOutcome, Intro2MeteorOwner,
        INTRO2_METEOR_SPAWN_INDICES,
    },
    session::GameSession,
    world_fx::WorldFx,
};

fn fixture() -> (GameSession, EntityManager, Vec<Intro2MeteorOwner>) {
    let path = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&path).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(50, 1).unwrap();
    let metadata: Vec<_> = session
        .cache
        .global_entity_model_table()
        .iter()
        .enumerate()
        .map(|(kind, slots)| {
            session
                .cache
                .global_entity_type(kind)
                .map(EntityTypeRuntimeMetadata::from_section12)
                .unwrap_or(EntityTypeRuntimeMetadata {
                    model_slots: *slots,
                    ..Default::default()
                })
        })
        .collect();
    let mut manager = EntityManager::from_level_with_type_metadata(
        session.cache.level_desc().unwrap(),
        &metadata,
        EntityConstructionResources {
            terrain: session.cache.terrain(),
            terrain_objects: session.cache.terrain_objects(),
            model_extent_raw: Some(&|id| session.cache.global_model(id).map(|model| model.radius)),
        },
    );
    let mut draws = 0;
    let owners = INTRO2_METEOR_SPAWN_INDICES
        .into_iter()
        .map(|spawn| {
            let id = manager
                .iter_all()
                .find(|e| e.authored_spawn_index == Some(spawn))
                .unwrap()
                .id;
            publish_intro2_meteor(manager.entity_mut(id).unwrap(), &metadata[34], &mut || {
                draws += 1;
                0
            })
            .unwrap()
        })
        .collect();
    assert_eq!(
        draws, 4,
        "each singleton selector still consumes its own word"
    );
    (session, manager, owners)
}

#[v2k_test_support::retail_test]
fn corpus_births_retain_prelaunch_private_position_and_park_tasks() {
    let (_session, manager, owners) = fixture();
    for owner in owners {
        let entity = manager
            .iter_all()
            .find(|e| e.id == owner.entity_id())
            .unwrap();
        let ActorTaskRuntime::BoulderRolling(state) =
            entity.actor_task_state(ActorTaskSlot::Primary).unwrap()
        else {
            panic!()
        };
        let mut original = entity.position_raw();
        original[0] = original[0].wrapping_add(0x500);
        original[2] = original[2].wrapping_sub(0xa00);
        assert_eq!(state.previous_position_raw, original);
        assert_eq!(entity.velocity_raw(), [1500, -400, -3000]);
        assert_eq!(
            entity.collision.state_flags_at_0x08.masked(0x68000),
            RetailRuntimeValue::Known(0)
        );
        let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
            panic!()
        };
        assert_eq!(basis.lateral, [0, 0, -2147352576]);
        assert_eq!(basis.up, [0, 2147352576, 0]);
        assert_eq!(basis.forward, [2147352576, 0, 0]);
    }
}

#[v2k_test_support::retail_test]
fn corpus_48ms_callback_matches_original_first_motion_interval() {
    let (session, mut manager, owners) = fixture();
    let owner = owners[0];
    let id = owner.entity_id();
    manager
        .entity_mut(id)
        .unwrap()
        .collision
        .state_flags_at_0x08 = RetailStateWord::exact(0x01478805);
    let mut fx = WorldFx::new();
    // Common scheduler's cap-crossing branch is deterministic and consumes no
    // random callback wait. It supplies 48ms through the retained carry below.
    let entity = manager.entity_mut(id).unwrap();
    entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(250000);
    // Detailed disables waits but changes rolling; the motion formula is shared.
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(0x02000000, 0x02000000);
    let tick = tick_intro2_meteor(
        &mut manager,
        owner,
        Intro2MeteorFrame {
            resources: &session.cache,
            world_fx: &mut fx,
            elapsed_micros: 48000,
            retail_tick: 151,
        },
    );
    assert!(
        matches!(
            tick.outcome,
            Intro2MeteorOutcome::Advanced {
                elapsed_micros: 48000,
                trail_attempts: 2,
                ..
            }
        ),
        "{:?}",
        tick.outcome
    );
    let entity = manager.iter_all().find(|e| e.id == id).unwrap();
    assert_eq!(entity.position_raw(), [-32188, 6378, -26249]);
    assert_eq!(entity.velocity_raw(), [1487, -467, -2978]);
}

#[v2k_test_support::retail_test]
fn real_model560_terrain_contact_enters_class1_then_defers_once() {
    let (session, mut manager, owners) = fixture();
    let id = owners[0].entity_id();
    let entity = manager.entity_mut(id).unwrap();
    entity.collision.state_flags_at_0x08 = RetailStateWord::exact(0x07478825);
    entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
    let [x, _, z] = entity.position_raw();
    let y = session
        .cache
        .level_terrain()
        .unwrap()
        .bilinear_height_raw(x, z);
    entity.set_motion_raw([x, y.wrapping_sub(300), z], [0, -10000, 0]);
    let mut fx = WorldFx::new();
    let receipt = resolve_intro2_meteor_terrain_contact(&mut manager, id, &session.cache, &mut fx)
        .unwrap()
        .expect("real oriented model sphere hits terrain");
    assert_eq!(receipt.radial_damage.inner_radius_raw, 512);
    assert_eq!(receipt.radial_damage.outer_radius_raw, 1024);
    assert_eq!(receipt.radial_damage.packet.channels, [1, 3]);
    assert_eq!(receipt.radial_damage.packet.amounts_raw, [4000, 4000]);
    let entity = manager.iter_all().find(|e| e.id == id).unwrap();
    assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
    assert!(
        entity.actor_task_state(ActorTaskSlot::Primary).is_some(),
        "BAF0 radial precedes A860 clear"
    );
    assert!(manager.pending_actor_deferred_destroy_ids().is_empty());
    // The caller must complete the receipt's radial work at this boundary.
    assert!(
        !finish_intro2_meteor_death(&mut manager, receipt),
        "unclaimed radial work cannot be skipped"
    );
    let mut altered = receipt;
    altered.radial_damage.outer_radius_raw += 1;
    assert!(!v2k_game::intro2_meteors::claim_intro2_meteor_radial(
        &mut manager,
        &altered
    ));
    assert!(v2k_game::intro2_meteors::claim_intro2_meteor_radial(
        &mut manager,
        &receipt
    ));
    assert!(
        !v2k_game::intro2_meteors::claim_intro2_meteor_radial(&mut manager, &receipt),
        "a claimed callback cannot replay even before finalization"
    );
    assert!(finish_intro2_meteor_death(&mut manager, receipt));
    assert!(!finish_intro2_meteor_death(&mut manager, receipt));
    assert_eq!(manager.pending_actor_deferred_destroy_ids(), [id]);
    assert!(
        manager.iter_all().find(|e| e.id == id).unwrap().active,
        "10B70 waits for the next sweep"
    );
}

#[v2k_test_support::retail_test]
fn four_authored_meteors_complete_live_scheduler_contacts_and_radial_without_replay() {
    use v2k_game::{
        gameplay_notifications::GameplayNotifications,
        intro2_contacts::{resolve_intro2_contacts, Intro2ContactFrame, Intro2ContactReport},
        intro2_radial::Intro2MeteorDeathReport,
        specialized_actor_task_production::{
            SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
            SpecializedActorTaskScheduler,
        },
        static_damage::StaticDamageScheduler,
        world_fx::{ParticleUpdateRequest, TerrainCollisionContext},
    };
    let (mut session, _, _) = fixture();
    let metadata: Vec<_> = session
        .cache
        .global_entity_model_table()
        .iter()
        .enumerate()
        .map(|(kind, slots)| {
            session
                .cache
                .global_entity_type(kind)
                .map(EntityTypeRuntimeMetadata::from_section12)
                .unwrap_or(EntityTypeRuntimeMetadata {
                    model_slots: *slots,
                    ..Default::default()
                })
        })
        .collect();
    let mut fx = WorldFx::new();
    let mut manager = EntityManager::from_captured_intro2_frontend_with_type_metadata(
        session.cache.level_desc().unwrap(),
        &metadata,
        EntityConstructionResources {
            terrain: session.cache.terrain(),
            terrain_objects: session.cache.terrain_objects(),
            model_extent_raw: Some(&|id| session.cache.global_model(id).map(|model| model.radius)),
        },
        v2k_game::entity::Intro2BirthSelection::default(),
        &mut || u32::from(fx.next_shared_retail_random_u16()),
    )
    .unwrap();
    // 2E570 initializes the persistent radar after all native births. The
    // hut's later 37390 crater refresh consumes this same retained owner.
    session
        .cache
        .initialize_level_terrain_radar(&mut || fx.next_shared_retail_random_u16())
        .unwrap();
    let ids: Vec<_> = INTRO2_METEOR_SPAWN_INDICES
        .into_iter()
        .map(|spawn| {
            (
                spawn,
                manager
                    .iter_all()
                    .find(|e| e.authored_spawn_index == Some(spawn))
                    .unwrap()
                    .id,
            )
        })
        .collect();
    let mut scheduler = SpecializedActorTaskScheduler::default();
    assert_eq!(scheduler.adopt_intro2_meteors(&manager), 4);
    assert_eq!(scheduler.adopt_intro2_type9(&mut manager), 13);
    assert_eq!(scheduler.adopt_intro2_type66(&manager), 2);
    // These authored radial targets now retain their actual mutable task
    // owners. A meteor cannot borrow an unregistered insect's body custody.
    assert_eq!(scheduler.adopt_intro2_type53(&manager), 4);
    assert_eq!(scheduler.adopt_intro2_type94(&manager), 1);
    assert_eq!(
        scheduler.adopt_intro2_type66(&manager),
        0,
        "radial targets retain one exact factory task owner"
    );
    assert_eq!(
        scheduler.adopt_intro2_meteors(&manager),
        0,
        "adoption is not a second callback owner"
    );
    let mut static_damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    let mut removed = Vec::new();
    let mut impacts = Vec::new();
    let mut commands =
        v2k_game::intro2_commands::Intro2Commands::from_cache(&session.cache).unwrap();
    commands.present(&mut manager, 0);
    let camera_anchor = commands.camera_subject(&manager);
    assert!(camera_anchor.is_some());
    // Controlled detailed camera admission isolates physics/collision from
    // unrelated camera interpolation. Other authored actors remain present
    // as real radial targets; no hypothetical target profiles are installed.
    for tick in 0..400 {
        for &(spawn, id) in &ids {
            if let Some(entity) = manager.entity_mut(id) {
                entity
                    .collision
                    .state_flags_at_0x08
                    .overwrite(0x06000000, 0x06000000);
                let activation = match spawn {
                    31 => 150,
                    33 => 25,
                    34 => 50,
                    35 => 75,
                    _ => unreachable!(),
                };
                if tick == activation {
                    entity
                        .collision
                        .state_flags_at_0x08
                        .overwrite(0x68000, 0x68000);
                }
            }
        }
        let pass = scheduler.tick(
            &mut manager,
            SpecializedActorTaskProductionFrame {
                world: v2k_game::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                hive_components: None,
                notification_phase:
                    v2k_game::gameplay_notifications::GameplayNotificationPhase::NonGameplay,
                resources: &mut session.cache,
                world_fx: &mut fx,
                static_damage: &mut static_damage,
                elapsed_micros: 20000,
                global_elapsed_micros: 20000,
                retail_tick: tick,
                main_base_abort_active: false,
            },
            &mut notifications,
        );
        assert_eq!(pass.block, None);
        for outcome in pass.outcomes {
            if let SpecializedActorTaskProductionOutcome::Intro2Type66(ref outcome) = outcome {
                assert!(
                    !matches!(
                        outcome,
                        v2k_game::intro2_type66::Intro2Type66Outcome::Blocked { .. }
                            | v2k_game::intro2_type66::Intro2Type66Outcome::Pending { .. }
                            | v2k_game::intro2_type66::Intro2Type66Outcome::Dropped { .. }
                    ),
                    "factory tick{tick}: {outcome:?}"
                );
            }
            if let SpecializedActorTaskProductionOutcome::Intro2Meteor { outcome, death } = outcome
            {
                assert!(
                    !matches!(
                        outcome,
                        Intro2MeteorOutcome::Blocked { .. } | Intro2MeteorOutcome::Dropped { .. }
                    ),
                    "tick{tick}: {outcome:?}"
                );
                if let Some(report) = death {
                    assert!(
                        matches!(
                            report,
                            Intro2MeteorDeathReport::Applied {
                                finalized: true,
                                ..
                            }
                        ),
                        "task death tick{tick}: {report:?}"
                    );
                }
            }
        }
        // 13500 ends with 14990. Contact deaths below remain linked until the
        // next world actor pass; radial victim task deaths can also join this
        // sweep now that their native lifecycle owners execute.
        for &id in manager.pending_actor_deferred_destroy_ids() {
            let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
            assert!(
                matches!(entity.collision.health_raw, RetailRuntimeValue::Known(health) if health <= 0)
            );
        }
        removed.extend(manager.cleanup_pending_actor_deferred_destroys());
        let terrain = TerrainCollisionContext::from_current_level_cache(&session.cache).unwrap();
        fx.update(ParticleUpdateRequest::terrain(20000, tick, terrain));
        let contacts = resolve_intro2_contacts(Intro2ContactFrame {
            entities: &mut manager,
            resources: &mut session.cache,
            world_fx: &mut fx,
            static_damage: &mut static_damage,
            notifications: &mut notifications,
            retail_tick: tick,
            actor_tasks: &mut scheduler,
        });
        for contact in contacts {
            let Intro2ContactReport::Meteor {
                entity_id: id,
                result,
            } = contact
            else {
                continue;
            };
            if let Some(report) = result.unwrap() {
                let spawn = ids
                    .iter()
                    .find(|(_, meteor_id)| *meteor_id == id)
                    .unwrap()
                    .0;
                let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
                let position = entity.position_raw();
                assert!(
                    impacts.iter().all(|(previous, _, _)| *previous != spawn),
                    "a completed contact cannot publish another meteor death"
                );
                impacts.push((spawn, tick, position));
                assert!(
                    matches!(
                        report,
                        Intro2MeteorDeathReport::Applied {
                            finalized: true,
                            ..
                        }
                    ),
                    "spawn{spawn} tick{tick} at{position:?}: {report:?}"
                );
                assert!(
                    Intro2MeteorOwner::adopt_published(entity).is_err(),
                    "the completed callback clears contact and scheduler admission"
                );
            }
        }
        assert_eq!(
            commands.camera_subject(&manager),
            camera_anchor,
            "removing earlier spawns must not redirect the authored camera handle",
        );
    }
    eprintln!("controlled detailed Intro2 meteor impacts: {impacts:?}");
    assert_eq!(
        impacts.len(),
        4,
        "all four lifetimes must be collision-driven"
    );
    removed.sort_unstable();
    assert!(
        removed.windows(2).all(|ids| ids[0] != ids[1]),
        "no deferred allocation may be spliced twice"
    );
    let mut expected: Vec<_> = ids.into_iter().map(|(_, id)| id).collect();
    expected.sort_unstable();
    let removed_meteors: Vec<_> = removed
        .into_iter()
        .filter(|id| expected.contains(id))
        .collect();
    assert_eq!(
        removed_meteors, expected,
        "each meteor allocation is spliced exactly once"
    );
}
