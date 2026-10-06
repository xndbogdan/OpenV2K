//! Actual normal-tier dynamic constructors, not an isolated receipt fixture.

use super::*;
use crate::{
    entity::MainBaseConversionDestroyQueueOutcome,
    factory_production_live::{
        FactoryEntitySpawnRequest, FactoryProductionEntityVersion, FactoryRequestedEntityHandle,
    },
    gameplay_notifications::GameplayNotifications,
    main_base_conversion::MainBaseReplacementSpawn,
    main_base_conversion_runtime::{
        apply_prepared_first_world_main_base_replacement, prepare_first_world_main_base_replacement,
    },
    session::GameSession,
    specialized_actor_task_production::{
        SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
        SpecializedActorTaskScheduler,
    },
    static_damage::StaticDamageScheduler,
};

fn assert_birth(manager: &EntityManager, id: u32, seed: u8) {
    let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
    assert!(intro2_type8_manager_allocation_authenticates(manager, id));
    let receipt = entity.intro2_type8_runtime.unwrap();
    assert_eq!(receipt.origin, Type8ConstructionOrigin::Native);
    assert_eq!(receipt.spawn_index, None);
    assert_eq!(receipt.sub_d_seed, seed);
    assert_eq!(entity.model_slots, [Some(559); 4]);
    assert_eq!(entity.capability_flags, 0x1404);
    assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(1500));
    assert_eq!(
        entity.collision.animation_offset_at_0xb2,
        RetailRuntimeValue::Known(0)
    );
    assert!(
        matches!(entity.actor_animation_runtime, RetailRuntimeValue::Known(Some(animation))
        if manager.type_runtime_metadata(8).unwrap().actor_animation_descriptor
            == RetailRuntimeValue::Known(Some(animation.descriptor())))
    );
    assert!(
        matches!(entity.sub_a_propulsion_runtime, RetailRuntimeValue::Known(Some(sub_a))
        if matches!(sub_a.target_speed_raw(), RetailRuntimeValue::Known(_)))
    );
    assert!(matches!(
        Intro2Type8Owner::published_graph(entity),
        Some((_, _, TaskKind::Wander | TaskKind::GoToJob))
    ));
}

fn tick(
    session: &mut GameSession,
    manager: &mut EntityManager,
    fx: &mut WorldFx,
    scheduler: &mut SpecializedActorTaskScheduler,
    id: u32,
    step: u32,
) -> Intro2Type8Outcome {
    let pass = scheduler.tick(
        manager,
        SpecializedActorTaskProductionFrame {
            world: crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
            hive_components: None,
            notification_phase: crate::gameplay_notifications::GameplayNotificationPhase::Playing,
            resources: &mut session.cache,
            world_fx: fx,
            static_damage: &mut StaticDamageScheduler::default(),
            elapsed_micros: 20_000,
            global_elapsed_micros: 20_000,
            retail_tick: step,
            main_base_abort_active: false,
        },
        &mut GameplayNotifications::new(),
    );
    assert!(pass.block.is_none(), "{:?}", pass.block);
    let result = pass
        .outcomes
        .into_iter()
        .find_map(|outcome| match outcome {
            SpecializedActorTaskProductionOutcome::Intro2Type8(outcome)
                if outcome.entity_id() == id =>
            {
                Some(outcome)
            }
            _ => None,
        })
        .expect("the dynamic worker has exactly one actual scheduler owner");
    assert!(
        !matches!(
            result,
            Intro2Type8Outcome::Blocked { .. }
                | Intro2Type8Outcome::Pending { .. }
                | Intro2Type8Outcome::Dropped { .. }
        ),
        "{result:?}"
    );
    result
}

#[v2k_test_support::retail_test]
fn later_world_main_base_conversion_constructs_and_adopts_a_real_worker() {
    let mut fx = WorldFx::new();
    let Some((mut session, mut manager)) = authored_tests::load(14, &mut fx) else {
        return;
    };
    let source = manager
        .iter_all()
        .find(|entity| entity.entity_type == 9)
        .unwrap()
        .id;
    let base = manager
        .iter_all()
        .find(|entity| entity.entity_type == 6)
        .unwrap();
    let request = MainBaseReplacementSpawn {
        source_entity_id: source,
        main_base_entity_id: base.id,
        replacement_type: 8,
        position_raw: base.position_raw(),
    };
    let prepared =
        prepare_first_world_main_base_replacement(&manager, session.cache.terrain(), request, 0)
            .expect("canonical later-world conversion closes every constructor gate");
    let seed = fx.next_sub_d_allocation_seed();
    assert_eq!(
        manager.queue_main_base_conversion_destroy(source),
        MainBaseConversionDestroyQueueOutcome::Queued { source_id: source }
    );
    let spawned = apply_prepared_first_world_main_base_replacement(
        &mut manager,
        session.cache.terrain(),
        &mut fx,
        prepared,
    );
    let id = spawned.replacement_id;
    assert_birth(&manager, id, seed);
    assert_eq!(fx.next_sub_d_allocation_seed(), seed.wrapping_add(1));
    let initial = manager.entity_mut(id).unwrap().position_raw();
    let mut scheduler = SpecializedActorTaskScheduler::default();
    assert_eq!(scheduler.adopt_intro2_type8(&mut manager), 1);
    assert_eq!(scheduler.adopt_intro2_type8(&mut manager), 0);
    assert_eq!(scheduler.adopt_live_type8_go_to_job(&manager), 0);
    for step in 1..=150 {
        tick(
            &mut session,
            &mut manager,
            &mut fx,
            &mut scheduler,
            id,
            step,
        );
    }
    assert_ne!(manager.entity_mut(id).unwrap().position_raw(), initial);
    assert!(scheduler.begin_intro2_type8_external_mutation(&manager, id));
}

#[v2k_test_support::retail_test]
fn real_factory_worker_runs_carrying_callbacks_and_reselects_at_the_released_anchor() {
    // Factory output can attach before first adoption or between two visits.
    // Both paths must retain one allocation and one scheduler owner.
    for (adopt_before_attach, lethal_while_carried) in [(false, false), (true, false), (true, true)]
    {
        let mut fx = WorldFx::new();
        let Some((mut session, mut manager)) = authored_tests::load(14, &mut fx) else {
            return;
        };
        let factory = manager
            .iter_all()
            .find(|entity| entity.entity_type == 66)
            .unwrap();
        let RetailRuntimeValue::Known(Some(base)) = factory.base_factory_runtime else {
            panic!("native factory");
        };
        let live = base.live_owner.unwrap();
        let source = FactoryProductionEntityVersion {
            entity_id: factory.id,
            allocation_identity: live.allocation_identity,
            state_version: live.state_version,
        };
        let mut position = factory.position_raw();
        position[2] = position[2].wrapping_sub(250);
        let request = FactoryEntitySpawnRequest {
            requested_handle: FactoryRequestedEntityHandle::Allocate,
            entity_type: 8,
            position_raw: position,
            spawn_parameter_6: 0,
        };
        let seed = fx.next_sub_d_allocation_seed();
        let output = manager
            .append_factory_converted_output(
                source,
                request,
                0,
                session.cache.terrain().unwrap(),
                &mut fx,
            )
            .expect("real canonical factory output closes late native retain preflight");
        let id = output.entity_id.get();
        assert_birth(&manager, id, seed);
        assert!(manager.link_factory_converted_output_owner(output, source));
        let mut scheduler = SpecializedActorTaskScheduler::default();
        if adopt_before_attach {
            assert_eq!(scheduler.adopt_intro2_type8(&mut manager), 1);
        }
        let proxy = manager
            .append_factory_output_materialiser(
                source,
                FactoryEntitySpawnRequest {
                    entity_type: 93,
                    ..request
                },
                session.cache.terrain().unwrap(),
                &mut fx,
            )
            .unwrap();
        assert_eq!(
            fx.next_sub_d_allocation_seed(),
            seed.wrapping_add(1),
            "Type93's canonical descriptor has no Sub-D allocation"
        );
        assert!(manager.link_factory_materialiser_output(proxy, output, &mut fx));
        let worker = manager.entity_mut(id).unwrap();
        assert_eq!(worker.attached_to, Some(proxy.entity_id.get()));
        assert!(matches!(
            Intro2Type8Owner::published_graph(worker),
            Some((_, _, TaskKind::Carried))
        ));
        let sub_d = (worker.type8_sub_d_frame_owner, worker.type8_sub_d_runtime);
        let before_animation = worker.actor_animation_runtime;
        let before_age = worker.collision.recent_relation_elapsed_us_at_0x68;
        let before_position = worker.position_raw();
        let before_anchor = worker.type8_wander_anchor_raw_at_0x90;
        if !adopt_before_attach {
            assert_eq!(scheduler.adopt_intro2_type8(&mut manager), 1);
        }
        assert_eq!(scheduler.adopt_intro2_type8(&mut manager), 0);
        assert_eq!(scheduler.adopt_live_type8_go_to_job(&manager), 0);
        // Hold the proxy phase while exercising the child's real12DA0 visit.
        for step in 1..=20 {
            tick(
                &mut session,
                &mut manager,
                &mut fx,
                &mut scheduler,
                id,
                step,
            );
        }
        let worker = manager.entity_mut(id).unwrap();
        assert_eq!(
            worker.position_raw(),
            before_position,
            "08F00 disables master motion"
        );
        assert_eq!(
            (worker.type8_sub_d_frame_owner, worker.type8_sub_d_runtime),
            sub_d,
            "None invokes Sub-I without touching Sub-D"
        );
        assert_ne!(worker.actor_animation_runtime, before_animation);
        assert_ne!(
            worker.collision.recent_relation_elapsed_us_at_0x68, before_age,
            "attached scheduler still advances age/waits"
        );
        if lethal_while_carried {
            let carried_graph = Intro2Type8Owner::published_graph(worker);
            let carried_health = worker.collision.health_raw;
            assert_eq!(
                worker.collision.state_flags_at_0x08.masked(0x8000),
                RetailRuntimeValue::Known(0),
                "CD70 disables checked damage while carrying"
            );
            let hit = impact::apply_intro2_type8_particle_hit(
                &mut manager,
                &mut fx,
                &mut scheduler,
                crate::world_fx::ParticleEntityImpact {
                    source_particle_class: 38,
                    impact_position_argument_va: 0,
                    target_entity_id: id,
                    position_world: [0.; 3],
                    velocity_raw: [0; 3],
                    damage: Some(crate::world_fx::BallisticDamageRequest {
                        packet: crate::damage::DamagePacket {
                            channels: [2, 3],
                            amounts_raw: [500, 6000],
                        },
                        // Complete accepted dragon38 delivery metadata; the
                        // shooter need not survive until this hit boundary.
                        source_entity_type_at_birth: Some(10),
                        source_owner_id: Some(0x045f_0001),
                    }),
                },
                20,
            );
            assert!(
                matches!(hit, impact::Intro2Type8ImpactOutcome::Applied(ref result)
                    if result.filtered_damage_raw == 0
                        && result.damage_after_buffer_raw == 0
                        && result.death_publication.is_none()),
                "{hit:?}"
            );
            let worker = manager.entity_mut(id).unwrap();
            assert_eq!(worker.collision.health_raw, carried_health);
            assert_eq!(Intro2Type8Owner::published_graph(worker), carried_graph);
            assert_eq!(worker.attached_to, Some(proxy.entity_id.get()));
            // 415060 rejects checked damage before filtering when8000 is clear.
            // Exercise the independent10C10 entry, which has no8000 gate, to
            // cover class14's null release without undoing carrying protection.
            let death = impact::run_intro2_type8_standard_death(&mut manager, id, &mut fx).unwrap();
            scheduler.register_intro2_type8(death.publication.unwrap());
            let corpse = manager.entity_mut(id).unwrap();
            assert_eq!(corpse.attached_to, Some(proxy.entity_id.get()));
            assert!(matches!(
                Intro2Type8Owner::published_graph(corpse),
                Some((_, _, TaskKind::Exploding))
            ));
        }
        // Controlled physical relocation of the real proxy isolates409030's
        // mutable anchor writer without altering the child's graph or receipt.
        let mut landing = before_position;
        landing[0] = landing[0].wrapping_add(256);
        landing[1] = session
            .cache
            .terrain()
            .unwrap()
            .bilinear_height_raw(landing[0], landing[2]);
        manager
            .entity_mut(proxy.entity_id.get())
            .unwrap()
            .set_motion_raw(landing, [0; 3]);
        let mut corpse_row_retired = false;
        let mut materialiser_release_observed = false;
        for step in 21..=60 {
            tick(
                &mut session,
                &mut manager,
                &mut fx,
                &mut scheduler,
                id,
                step,
            );
            let worker = manager.entity_mut(id).unwrap();
            let child_before_tail = (
                worker.position_raw(),
                worker.type8_wander_anchor_raw_at_0x90,
                worker.attached_to,
            );
            let mut release_rng = fx.fork_for_main_base_abort_transaction();
            assert!(manager
                .update_late_tail_materialisers(crate::entity::LateTailMaterialiserFrame {
                    elapsed_micros: 20_000,
                    terrain: session.cache.terrain().unwrap(),
                    world_fx: &mut fx,
                    scheduler: &mut scheduler,
                    notifications: &mut GameplayNotifications::new(),
                    retail_tick: step,
                })
                .is_empty());
            for event in manager.take_cargo_proxy_events() {
                if let crate::entity::CargoProxyEvent::Released { cargo_id, .. } = event {
                    assert_eq!(cargo_id, id);
                    assert!(
                        !lethal_while_carried,
                        "18640 retired the dying row before18500"
                    );
                    materialiser_release_observed = true;
                }
            }
            if lethal_while_carried {
                // 18640 retires a dying row before copying the proxy pose.
                // Only the child's next continuing12DA0 sees the absent row
                // and calls16750 through class14's null release callback.
                let worker = manager.entity_mut(id).unwrap();
                assert_eq!(
                    (
                        worker.position_raw(),
                        worker.type8_wander_anchor_raw_at_0x90,
                        worker.attached_to,
                    ),
                    child_before_tail,
                    "row retirement does not run409030 or release the child"
                );
                if !corpse_row_retired {
                    assert_eq!(worker.attached_to, Some(proxy.entity_id.get()));
                }
                let parent = manager.entity_mut(proxy.entity_id.get()).unwrap();
                assert!(matches!(&parent.sub_j_attachment_runtime,
                    RetailRuntimeValue::Known(Some(rows)) if rows.ordered_entity_ids().is_empty()));
                corpse_row_retired = true;
            }
            if manager.entity_mut(id).unwrap().attached_to.is_none() {
                for _ in 0..if lethal_while_carried { 0 } else { 2 } {
                    release_rng.next_shared_retail_random_u16();
                }
                assert_eq!(
                    fx.fork_for_main_base_abort_transaction()
                        .next_shared_retail_random_u16(),
                    release_rng.next_shared_retail_random_u16(),
                    "CE90 selects/constructs once; class14 has no release callback or RNG"
                );
                break;
            }
        }
        let worker = manager.entity_mut(id).unwrap();
        assert_eq!(worker.attached_to, None);
        if lethal_while_carried {
            assert!(corpse_row_retired);
            assert!(!materialiser_release_observed);
            assert_eq!(worker.position_raw(), before_position);
            assert_eq!(worker.type8_wander_anchor_raw_at_0x90, before_anchor);
        } else {
            assert!(materialiser_release_observed);
            assert_eq!(worker.position_raw(), landing);
            assert_eq!(
                worker.type8_wander_anchor_raw_at_0x90,
                RetailRuntimeValue::Known(landing)
            );
        }
        assert_eq!(
            worker.intro2_type8_runtime.unwrap().anchor_raw,
            before_position,
            "birth evidence stays immutable while live +90 changes"
        );
        if lethal_while_carried {
            assert!(matches!(
                Intro2Type8Owner::published_graph(worker),
                Some((_, _, TaskKind::Exploding))
            ));
        } else {
            assert!(matches!(
                Intro2Type8Owner::published_graph(worker),
                Some((_, _, TaskKind::Wander | TaskKind::GoToJob))
            ));
        }
        assert!(intro2_type8_manager_allocation_authenticates(&manager, id));
        assert!(scheduler.begin_intro2_type8_external_mutation(&manager, id));
        let mut removed = false;
        for step in 61..=160 {
            tick(
                &mut session,
                &mut manager,
                &mut fx,
                &mut scheduler,
                id,
                step,
            );
            removed |= manager
                .cleanup_pending_actor_deferred_destroys()
                .contains(&id);
            if removed {
                break;
            }
        }
        if lethal_while_carried {
            assert!(
                removed,
                "attached class14 resumes its terminal transition after release"
            );
        } else {
            assert_ne!(
                manager.entity_mut(id).unwrap().position_raw(),
                landing,
                "released worker resumes genuine D/I/A/B motion"
            );
        }
        assert_eq!(scheduler.adopt_intro2_type8(&mut manager), 0);
    }
}

#[v2k_test_support::retail_test]
fn real_converted_worker_can_be_beamed_dropped_and_rejects_a_parked_owner() {
    use crate::entity::{
        AuthoredPlayerArrival, BeamCommand, BeamOutcome, CargoDropContext, PlayerCargoFrame,
    };
    for lethal_while_carried in [false, true] {
        let mut fx = WorldFx::new();
        let Some((mut session, mut manager)) = authored_tests::load_with_arrival(
            14,
            &mut fx,
            Some(AuthoredPlayerArrival {
                position_raw: [19_712, -500, 14_848],
                heading_raw: 0x4000,
            }),
        ) else {
            return;
        };
        let source = manager
            .iter_all()
            .find(|entity| entity.entity_type == 9)
            .unwrap()
            .id;
        let base = manager
            .iter_all()
            .find(|entity| entity.entity_type == 6)
            .unwrap();
        let request = MainBaseReplacementSpawn {
            source_entity_id: source,
            main_base_entity_id: base.id,
            replacement_type: 8,
            position_raw: base.position_raw(),
        };
        let prepared = prepare_first_world_main_base_replacement(
            &manager,
            session.cache.terrain(),
            request,
            0,
        )
        .unwrap();
        assert_eq!(
            manager.queue_main_base_conversion_destroy(source),
            MainBaseConversionDestroyQueueOutcome::Queued { source_id: source }
        );
        let id = apply_prepared_first_world_main_base_replacement(
            &mut manager,
            session.cache.terrain(),
            &mut fx,
            prepared,
        )
        .replacement_id;
        let mut scheduler = SpecializedActorTaskScheduler::default();
        assert_eq!(scheduler.adopt_intro2_type8(&mut manager), 1);
        let worker_position = manager.entity_mut(id).unwrap().position;
        manager.player_mut().unwrap().position = [
            worker_position[0] + 0.25,
            worker_position[1] - 1.,
            worker_position[2],
        ];
        manager.queue_beam(BeamCommand::Collect);
        let mut collected = false;
        for step in 1..=30 {
            let result = manager.update_player_cargo(PlayerCargoFrame {
                elapsed_micros: 20_000,
                drop_context: Some(CargoDropContext {
                    carrier_orientation: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
                    terrain: session.cache.terrain().unwrap(),
                }),
                retail_tick: step,
                scheduler: &mut scheduler,
                world_fx: &mut fx,
                notifications: &mut GameplayNotifications::new(),
            });
            assert!(result.blocked.is_empty(), "{:?}", result.blocked);
            if let Some(beam) = result.beam {
                assert_eq!(beam, BeamOutcome::Collected { entity_id: id });
                collected = true;
            }
            tick(
                &mut session,
                &mut manager,
                &mut fx,
                &mut scheduler,
                id,
                step,
            );
        }
        assert!(collected);
        let player = manager.player().unwrap().id;
        assert_eq!(manager.entity_mut(id).unwrap().attached_to, Some(player));
        assert_eq!(
            manager
                .entity_mut(id)
                .unwrap()
                .collision
                .state_flags_at_0x08
                .masked(0x800),
            RetailRuntimeValue::Known(0),
            "the player's policy0 row hides the worker body"
        );
        assert!(matches!(
            Intro2Type8Owner::published_graph(manager.entity_mut(id).unwrap()),
            Some((_, _, TaskKind::Carried))
        ));
        let x = worker_position[0] + 2.;
        if lethal_while_carried {
            // Source10C10 leaves physical attachment while replacing the task.
            // Drop releases through Class14's null DC50 callback, then CD50
            // reattachment to the materialiser selects terminal C470.
            let death = impact::run_intro2_type8_standard_death(&mut manager, id, &mut fx).unwrap();
            scheduler.register_intro2_type8(death.publication.unwrap());
        }
        let z = worker_position[2];
        let height = session.cache.terrain().unwrap().height_at(x, z);
        manager.player_mut().unwrap().position = [x, height - 1., z + 400. / 256.];
        manager.queue_beam(BeamCommand::Drop);
        let mut proxy = None;
        let mut released = None;
        let pending_before_drop = manager.pending_actor_deferred_destroy_ids().to_vec();
        for step in 31..=130 {
            let result = manager.update_player_cargo(PlayerCargoFrame {
                elapsed_micros: 20_000,
                drop_context: Some(CargoDropContext {
                    carrier_orientation: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
                    terrain: session.cache.terrain().unwrap(),
                }),
                retail_tick: step,
                scheduler: &mut scheduler,
                world_fx: &mut fx,
                notifications: &mut GameplayNotifications::new(),
            });
            assert!(result.blocked.is_empty(), "{:?}", result.blocked);
            if let Some(beam) = result.beam {
                let BeamOutcome::DropStarted { cargo_id, proxy_id } = beam else {
                    panic!("{beam:?}");
                };
                assert_eq!(cargo_id, id);
                assert!(manager
                    .entity_mut(id)
                    .unwrap()
                    .intro2_type8_runtime
                    .unwrap()
                    .is_native_construction());
                assert_eq!(
                    manager
                        .entity_mut(id)
                        .unwrap()
                        .collision
                        .state_flags_at_0x08
                        .masked(0x40800),
                    RetailRuntimeValue::Known(0),
                    "08F00 hides and suspends the released worker under Type93"
                );
                assert_eq!(
                    manager
                        .entity_mut(proxy_id)
                        .unwrap()
                        .collision
                        .state_flags_at_0x08
                        .masked(0x800),
                    RetailRuntimeValue::Known(0),
                    "only the materialiser presentation submits the copied model"
                );
                proxy = Some(proxy_id);
            }
            if lethal_while_carried && proxy.is_some() {
                break;
            }
            tick(
                &mut session,
                &mut manager,
                &mut fx,
                &mut scheduler,
                id,
                step,
            );
            let blocked =
                manager.update_late_tail_materialisers(crate::entity::LateTailMaterialiserFrame {
                    elapsed_micros: 20_000,
                    terrain: session.cache.terrain().unwrap(),
                    world_fx: &mut fx,
                    scheduler: &mut scheduler,
                    notifications: &mut GameplayNotifications::new(),
                    retail_tick: step,
                });
            assert!(blocked.is_empty(), "{blocked:?}");
            if proxy.is_some() && manager.entity_mut(id).unwrap().attached_to.is_none() {
                released = Some(manager.entity_mut(id).unwrap().position_raw());
                break;
            }
        }
        if lethal_while_carried {
            let proxy = proxy.expect("corpse drop still constructs and links the materialiser");
            assert_eq!(scheduler.family_for(id), None);
            let worker = manager.entity_mut(id).unwrap();
            assert_eq!(worker.attached_to, Some(proxy));
            assert!(worker.active);
            for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
                assert!(worker.actor_tasks.task_in_slot(slot).is_none());
            }
            let mut pending_after_drop = pending_before_drop;
            pending_after_drop.push(id);
            assert_eq!(
                manager.pending_actor_deferred_destroy_ids(),
                pending_after_drop
            );
            assert!(manager
                .cleanup_pending_actor_deferred_destroys()
                .contains(&id));
            assert!(!manager.iter_all().any(|entity| entity.id == id));
            assert_eq!(scheduler.adopt_intro2_type8(&mut manager), 0);
            continue;
        }
        let landing = released.expect("actual player Type93 releases its Type8 carrying callback");
        assert_eq!(
            manager
                .entity_mut(id)
                .unwrap()
                .collision
                .state_flags_at_0x08
                .masked(0x800),
            RetailRuntimeValue::Known(0x800),
            "16750 restores the worker body after materialisation"
        );
        assert!(intro2_type8_manager_allocation_authenticates(&manager, id));
        for step in 131..=230 {
            tick(
                &mut session,
                &mut manager,
                &mut fx,
                &mut scheduler,
                id,
                step,
            );
        }
        assert_ne!(manager.entity_mut(id).unwrap().position_raw(), landing);

        // A real interrupted owner is not interchangeable with the same entity's
        // apparently complete graph. The beam must reject before relation writes.
        scheduler.park_intro2_type8_external_prefix(id);
        let worker = manager.entity_mut(id).unwrap();
        let position = worker.position;
        let before_context = worker.current_behavior_context;
        let before_task = worker.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
        manager.player_mut().unwrap().position =
            [position[0] + 0.25, position[1] - 1., position[2]];
        manager.queue_beam(BeamCommand::Collect);
        let mut blocked = false;
        for step in 231..=250 {
            let result = manager.update_player_cargo(PlayerCargoFrame {
                elapsed_micros: 20_000,
                drop_context: None,
                retail_tick: step,
                scheduler: &mut scheduler,
                world_fx: &mut fx,
                notifications: &mut GameplayNotifications::new(),
            });
            blocked |= result.blocked.iter().any(|(cargo, _)| *cargo == id);
        }
        assert!(blocked);
        let worker = manager.entity_mut(id).unwrap();
        assert_eq!(worker.attached_to, None);
        assert_eq!(worker.current_behavior_context, before_context);
        assert_eq!(
            worker.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
            before_task
        );
    }
}
