//! Real style2/4 worker allocations through public live owners. Controlled
//! contact/cargo poses retain the authored body, metadata and task graph.
use v2k_game::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    damage::DamagePacket,
    entity::{
        AuthoredPlayerArrival, AuthoredWorldConstruction, BeamCommand, BeamOutcome,
        CargoDropContext, Entity, EntityConstructionResources, EntityManager,
        LateTailMaterialiserFrame, PlayerCargoFrame,
    },
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue, DYING_STATE_BIT},
    entity_scheduler::{
        COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT, SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
    },
    gameplay_notifications::{GameplayNotificationPhase, GameplayNotifications},
    intro2_type8::{impact::Intro2Type8ImpactOutcome, Intro2Type8Outcome},
    session::GameSession,
    shared_actor_impact::{
        apply_shared_actor_particle_hit, SharedActorImpactFrame, SharedActorImpactOutcome,
    },
    specialized_actor_task_production::{
        PlayingRadialFrame, SpecializedActorTaskProductionFrame,
        SpecializedActorTaskProductionOutcome, SpecializedActorTaskScheduler,
    },
    static_damage::{StaticDamageScheduler, KIND_27_RADIAL_TEMPLATE},
    world_fx::{BallisticDamageRequest, ParticleEntityImpact, WorldFx},
};

struct World {
    session: GameSession,
    entities: EntityManager,
    fx: WorldFx,
    tasks: SpecializedActorTaskScheduler,
    notifications: GameplayNotifications,
    static_damage: StaticDamageScheduler,
}
impl World {
    fn load(level: u32, player: bool) -> Self {
        let data = v2k_test_support::retail_dir();
        assert!(data.join("PRELOAD.DAT").is_file(), "retail corpus required");
        let mut session = GameSession::init(&data).unwrap();
        session.load_auxiliary_ovl(3, 1).unwrap();
        session.load_level_by_id(level, 1).unwrap();
        let metadata: Vec<_> = session
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
        let mut fx = WorldFx::new();
        let entities = EntityManager::from_authored_world(
            AuthoredWorldConstruction {
                level: session.cache.level_desc().unwrap(),
                logical_world_index: (level - 12) as i32,
                type_metadata: &metadata,
                resources: EntityConstructionResources {
                    terrain: session.cache.terrain(),
                    terrain_objects: session.cache.terrain_objects(),
                    model_extent_raw: Some(&|id| session.cache.global_model(id).map(|m| m.radius)),
                },
                player_arrival: player.then_some(AuthoredPlayerArrival {
                    position_raw: [0; 3],
                    heading_raw: 0,
                }),
                retail_tick: 0,
            },
            &mut fx,
        )
        .unwrap();
        Self {
            session,
            entities,
            fx,
            tasks: SpecializedActorTaskScheduler::new(),
            notifications: GameplayNotifications::new(),
            static_damage: StaticDamageScheduler::new(),
        }
    }
    fn actor(&self, id: u32) -> &Entity {
        self.entities.iter_all().find(|e| e.id == id).unwrap()
    }
    fn ids(&self, kind: u32) -> Vec<u32> {
        self.entities
            .iter_all()
            .filter(|e| e.entity_type == kind)
            .map(|e| e.id)
            .collect()
    }
    fn adopt(&mut self, person: bool) -> usize {
        let ids: Vec<_> = self
            .entities
            .iter_all()
            .filter(|e| {
                if person {
                    e.native_type86_runtime.is_some()
                } else {
                    e.intro2_type8_runtime.is_some()
                }
            })
            .map(|e| e.id)
            .collect();
        for id in &ids {
            detailed(self.entities.entity_mut(*id).unwrap());
        }
        let count = if person {
            self.tasks.adopt_native_type86(&mut self.entities)
        } else {
            self.tasks.adopt_intro2_type8(&mut self.entities)
        };
        assert_eq!(count, ids.len());
        assert_eq!(
            if person {
                self.tasks.adopt_native_type86(&mut self.entities)
            } else {
                self.tasks.adopt_intro2_type8(&mut self.entities)
            },
            0
        );
        count
    }
    fn step(&mut self, tick: u32, elapsed: u32) {
        let pass = self.tasks.tick(
            &mut self.entities,
            SpecializedActorTaskProductionFrame {
                world: v2k_game::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                hive_components: None,
                notification_phase: GameplayNotificationPhase::Playing,
                resources: &mut self.session.cache,
                world_fx: &mut self.fx,
                static_damage: &mut self.static_damage,
                elapsed_micros: elapsed,
                global_elapsed_micros: elapsed,
                retail_tick: tick,
                main_base_abort_active: false,
            },
            &mut self.notifications,
        );
        assert!(pass.block.is_none(), "{:?}", pass.block);
        for outcome in pass.outcomes {
            match outcome {
                SpecializedActorTaskProductionOutcome::Intro2Type8(outcome) => assert!(
                    !matches!(
                        outcome,
                        Intro2Type8Outcome::Blocked { .. }
                            | Intro2Type8Outcome::Pending { .. }
                            | Intro2Type8Outcome::Dropped { .. }
                    ),
                    "{outcome:?}"
                ),
                SpecializedActorTaskProductionOutcome::NativeType86(outcome) => assert!(
                    !matches!(
                        outcome,
                        v2k_game::native_type86::Type86Outcome::Blocked { .. }
                            | v2k_game::native_type86::Type86Outcome::Pending { .. }
                            | v2k_game::native_type86::Type86Outcome::Dropped { .. }
                    ),
                    "{outcome:?}"
                ),
                _ => {}
            }
        }
        let blocked = self
            .entities
            .update_late_tail_materialisers(LateTailMaterialiserFrame {
                elapsed_micros: elapsed,
                terrain: self.session.cache.terrain().unwrap(),
                world_fx: &mut self.fx,
                scheduler: &mut self.tasks,
                notifications: &mut self.notifications,
                retail_tick: tick,
            });
        assert!(blocked.is_empty(), "{blocked:?}");
    }
    fn hit(&mut self, id: u32, amounts: [i32; 2], tick: u32) -> SharedActorImpactOutcome {
        apply_shared_actor_particle_hit(
            SharedActorImpactFrame {
                resources: &self.session.cache,
                entities: &mut self.entities,
                world_fx: &mut self.fx,
                scheduler: &mut self.tasks,
                notifications: &mut self.notifications,
                retail_tick: tick,
            },
            ParticleEntityImpact {
                source_particle_class: 38,
                impact_position_argument_va: 0,
                target_entity_id: id,
                position_world: [0.; 3],
                velocity_raw: [0; 3],
                damage: Some(BallisticDamageRequest {
                    packet: DamagePacket {
                        channels: [2, 3],
                        amounts_raw: amounts,
                    },
                    source_entity_type_at_birth: Some(10),
                    source_owner_id: Some(0x045f_0001),
                }),
            },
        )
        .expect("retained worker must select its public impact owner")
    }
}
fn detailed(entity: &mut Entity) {
    let flags =
        COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT;
    entity.collision.state_flags_at_0x08.overwrite(flags, flags);
    entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
    entity.collision.callback_scheduler_accumulator_us_at_0x6c = RetailRuntimeValue::Known(0);
}

#[v2k_test_support::retail_test]
fn all_nine_authored_workers_retain_own_components_and_move() {
    for (level, kind, model, spawns) in [
        (19, 79, 979, &[15, 16, 17, 18][..]),
        (38, 79, 979, &[5, 10, 11, 12][..]),
        (40, 91, 662, &[21][..]),
    ] {
        let mut world = World::load(level, false);
        let ids = world.ids(kind);
        assert_eq!(ids.len(), spawns.len());
        assert_eq!(
            ids.iter()
                .map(|id| world.actor(*id).authored_spawn_index.unwrap())
                .collect::<Vec<_>>(),
            spawns
        );
        assert_eq!(
            world.session.cache.level_desc().unwrap().raw_u32(0x90),
            Some(0)
        );
        let metadata = EntityTypeRuntimeMetadata::from_section12(
            world
                .session
                .cache
                .global_entity_type(kind as usize)
                .unwrap(),
        );
        assert_eq!(world.adopt(false), ids.len());
        let before: Vec<_> = ids
            .iter()
            .map(|id| {
                let e = world.actor(*id);
                assert!(e.intro2_type8_runtime.is_some());
                assert_eq!(e.model_slots, [Some(model); 4]);
                assert_eq!(e.collision.health_raw, RetailRuntimeValue::Known(2000));
                assert_eq!(e.capability_flags, metadata.capability_flags);
                let RetailRuntimeValue::Known(Some(animation)) = e.actor_animation_runtime else {
                    panic!("Sub-I")
                };
                assert_eq!(
                    RetailRuntimeValue::Known(Some(animation.descriptor())),
                    metadata.actor_animation_descriptor
                );
                assert!(e.type8_sub_d_runtime.is_some() && e.type8_sub_d_frame_owner.is_some());
                assert!(matches!(
                    e.actor_task_state(ActorTaskSlot::Primary),
                    Some(ActorTaskRuntime::OrdinaryType9Wander(_))
                ));
                (
                    *id,
                    e.position_raw(),
                    e.intro2_type8_runtime,
                    e.construction_stamp_at_0xb4,
                )
            })
            .collect();
        let mut moved = vec![false; ids.len()];
        for tick in 1..=30 {
            world.step(tick * 5, 100_000);
            for (index, (id, position, _, _)) in before.iter().enumerate() {
                moved[index] |= world.actor(*id).position_raw() != *position;
            }
        }
        assert!(moved.iter().all(|m| *m), "world{level}: moved={moved:?}");
        for (id, _, receipt, stamp) in before {
            assert_eq!(world.actor(id).intro2_type8_runtime, receipt);
            assert_eq!(world.actor(id).construction_stamp_at_0xb4, stamp);
        }
    }
}

#[v2k_test_support::retail_test]
fn public_worker_hits_reselect_then_publish_and_complete_class14() {
    for (level, kind) in [(19, 79), (38, 79), (40, 91)] {
        let mut world = World::load(level, false);
        let id = world.ids(kind)[0];
        world.adopt(false);
        world.step(5, 100_000);
        let receipt = world.actor(id).intro2_type8_runtime;
        let hit = world.hit(id, [500, 0], 10);
        assert!(
            matches!(hit, SharedActorImpactOutcome::Worker(Intro2Type8ImpactOutcome::Applied(ref r))
            if r.filtered_damage_raw == 300 && r.death_publication.is_none()),
            "{hit:?}"
        );
        assert_eq!(
            world.actor(id).collision.health_raw,
            RetailRuntimeValue::Known(1700)
        );
        let hit = world.hit(id, [500, 6000], 11);
        assert!(
            matches!(hit, SharedActorImpactOutcome::Worker(Intro2Type8ImpactOutcome::Applied(ref r))
            if r.filtered_damage_raw == 12300 && r.death_publication.is_some()),
            "{hit:?}"
        );
        assert_eq!(
            world.actor(id).collision.health_raw,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            world
                .actor(id)
                .collision
                .state_flags_at_0x08
                .masked(DYING_STATE_BIT),
            RetailRuntimeValue::Known(DYING_STATE_BIT)
        );
        let task = world
            .actor(id)
            .actor_task_state(ActorTaskSlot::Primary)
            .copied();
        let repeat = world.hit(id, [500, 6000], 12);
        assert!(
            matches!(repeat, SharedActorImpactOutcome::Worker(Intro2Type8ImpactOutcome::Applied(ref r)) if r.death_publication.is_none()),
            "{repeat:?}"
        );
        assert_eq!(
            world
                .actor(id)
                .actor_task_state(ActorTaskSlot::Primary)
                .copied(),
            task
        );
        assert_eq!(world.actor(id).intro2_type8_runtime, receipt);
        let mut removed = false;
        for tick in 3..=63 {
            world.step(tick * 5, 100_000);
            if world
                .entities
                .cleanup_pending_actor_deferred_destroys()
                .contains(&id)
            {
                removed = true;
                break;
            }
        }
        assert!(removed, "world{level} class14 must finish");
    }
}

#[v2k_test_support::retail_test]
fn foreign_worker_receipts_reject_public_hit_before_rng_or_body_writes() {
    for (level, kind) in [(19, 79), (40, 91)] {
        let mut world = World::load(level, false);
        let mut foreign = World::load(level, false);
        let id = world.ids(kind)[0];
        world.adopt(false);
        foreign.adopt(false);
        world.step(5, 100_000);
        foreign.step(5, 100_000);
        world.entities.entity_mut(id).unwrap().intro2_type8_runtime =
            foreign.actor(id).intro2_type8_runtime;
        let collision = world.actor(id).collision.clone();
        let graph = world
            .actor(id)
            .actor_task_state(ActorTaskSlot::Primary)
            .copied();
        let body = (
            world.actor(id).position_raw(),
            world.actor(id).physical_body_basis_q31(),
        );
        let events = world.fx.pending_event_count();
        let hit = world.hit(id, [500, 6000], 10);
        assert!(
            matches!(
                hit,
                SharedActorImpactOutcome::Worker(Intro2Type8ImpactOutcome::Blocked {
                    committed_prefix: false,
                    ..
                })
            ),
            "{hit:?}"
        );
        assert_eq!(world.actor(id).collision, collision);
        assert_eq!(
            world
                .actor(id)
                .actor_task_state(ActorTaskSlot::Primary)
                .copied(),
            graph
        );
        assert_eq!(
            (
                world.actor(id).position_raw(),
                world.actor(id).physical_body_basis_q31()
            ),
            body
        );
        assert_eq!(world.fx.pending_event_count(), events);
        assert_eq!(
            world.fx.next_shared_retail_random_u16(),
            foreign.fx.next_shared_retail_random_u16()
        );
    }
}

#[v2k_test_support::retail_test]
fn native_worker_and_person_profiles_survive_player_collect_materialise_and_drop() {
    for (level, kind) in [(19, 79), (40, 91), (17, 78), (16, 95)] {
        let mut world = World::load(level, true);
        let id = world.ids(kind)[0];
        let person = matches!(kind, 78 | 95);
        world.adopt(person);
        world.step(1, 20_000);
        let worker_receipt = world.actor(id).intro2_type8_runtime;
        let person_receipt = world.actor(id).native_type86_runtime;
        let position = world.actor(id).position;
        // Keep every real competing collectible out of the beam's small range.
        let others: Vec<_> = world
            .entities
            .iter_all()
            .filter(|e| e.id != id && e.capability_flags & 0x400 != 0)
            .map(|e| e.id)
            .collect();
        for other in others {
            world.entities.entity_mut(other).unwrap().position =
                [position[0] + 64., position[1], position[2]];
        }
        world.entities.player_mut().unwrap().position =
            [position[0] + 0.25, position[1] - 1., position[2]];
        world.entities.queue_beam(BeamCommand::Collect);
        let mut collected = false;
        for tick in 2..=31 {
            let result = world.entities.update_player_cargo(PlayerCargoFrame {
                elapsed_micros: 20_000,
                drop_context: None,
                retail_tick: tick,
                scheduler: &mut world.tasks,
                world_fx: &mut world.fx,
                notifications: &mut world.notifications,
            });
            assert!(
                result.blocked.is_empty(),
                "type{kind}: {:?}",
                result.blocked
            );
            if let Some(beam) = result.beam {
                assert_eq!(beam, BeamOutcome::Collected { entity_id: id });
                collected = true;
            }
            world.step(tick, 20_000);
        }
        assert!(collected, "type{kind}");
        assert_eq!(
            world.actor(id).attached_to,
            Some(world.entities.player().unwrap().id)
        );
        assert!(matches!(
            world.actor(id).actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::None)
        ));
        let x = position[0] + 2.;
        let z = position[2];
        let floor = world.session.cache.terrain().unwrap().height_at(x, z);
        world.entities.player_mut().unwrap().position = [x, floor - 1., z + 400. / 256.];
        world.entities.queue_beam(BeamCommand::Drop);
        let mut proxy = None;
        for tick in 32..=150 {
            let result = world.entities.update_player_cargo(PlayerCargoFrame {
                elapsed_micros: 20_000,
                drop_context: Some(CargoDropContext {
                    carrier_orientation: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
                    terrain: world.session.cache.terrain().unwrap(),
                }),
                retail_tick: tick,
                scheduler: &mut world.tasks,
                world_fx: &mut world.fx,
                notifications: &mut world.notifications,
            });
            assert!(
                result.blocked.is_empty(),
                "type{kind}: {:?}",
                result.blocked
            );
            if let Some(beam) = result.beam {
                let BeamOutcome::DropStarted { cargo_id, proxy_id } = beam else {
                    panic!("{beam:?}")
                };
                assert_eq!(cargo_id, id);
                proxy = Some(proxy_id);
            }
            world.step(tick, 20_000);
            if proxy.is_some() && world.actor(id).attached_to.is_none() {
                break;
            }
        }
        assert!(proxy.is_some(), "type{kind} Type93 creation");
        assert_eq!(
            world.actor(id).attached_to,
            None,
            "type{kind} materialiser release"
        );
        // Relation publication changes private receipt graph state, so compare
        // allocation custody through the scheduler instead of byte equality.
        assert_eq!(
            world.actor(id).intro2_type8_runtime.is_some(),
            worker_receipt.is_some()
        );
        assert_eq!(
            world.actor(id).native_type86_runtime.is_some(),
            person_receipt.is_some()
        );
        let landed = world.actor(id).position_raw();
        let mut moved = false;
        for tick in 151..=180 {
            world.step(tick, 100_000);
            moved |= world.actor(id).position_raw() != landed;
        }
        assert!(moved, "type{kind} released native graph must move");
    }
}

#[v2k_test_support::retail_test]
fn type91_reselects_near_real_factory_and_commits_native_intake_once() {
    use v2k_game::factory_activation_live::{
        apply_prepared_factory_scientist_arrival, prepare_factory_scientist_arrival,
    };
    let mut world = World::load(40, false);
    let id = world.ids(91)[0];
    let factory = world.ids(66)[0];
    world.adopt(false);
    world.step(5, 100_000);
    let position = world.actor(factory).position;
    world.entities.entity_mut(id).unwrap().position = [position[0] + 2., position[1], position[2]];
    let hit = world.hit(id, [0, 0], 10);
    assert!(
        matches!(
            hit,
            SharedActorImpactOutcome::Worker(Intro2Type8ImpactOutcome::Applied(_))
        ),
        "{hit:?}"
    );
    world.step(15, 100_000);
    assert!(
        matches!(world.actor(id).actor_task_state(ActorTaskSlot::Primary),Some(ActorTaskRuntime::GoToJob(state)) if state.target_id()==Some(factory))
    );
    // Controlled final contact pose retains the genuinely selected54 graph.
    world.entities.entity_mut(id).unwrap().position = position;
    let stale = prepare_factory_scientist_arrival(&world.entities, factory, id).unwrap();
    let angles = world.actor(id).rotation_heading_pitch_roll_raw();
    world
        .entities
        .entity_mut(id)
        .unwrap()
        .set_rotation_heading_pitch_roll_raw([angles[0], angles[1].wrapping_add(1), angles[2]]);
    let factory_before = world.actor(factory).base_factory_runtime;
    let events_before = world.fx.pending_event_count();
    let notifications_before = format!("{:?}", world.notifications);
    assert!(
        apply_prepared_factory_scientist_arrival(
            &mut world.entities,
            &mut world.fx,
            &mut world.notifications,
            16,
            stale,
        )
        .is_err(),
        "the prepared native body journal is stale"
    );
    assert_eq!(world.actor(factory).base_factory_runtime, factory_before);
    assert_eq!(world.fx.pending_event_count(), events_before);
    assert_eq!(format!("{:?}", world.notifications), notifications_before);
    assert!(world
        .entities
        .pending_factory_scientist_destroy_ids()
        .is_empty());
    world
        .entities
        .entity_mut(id)
        .unwrap()
        .set_rotation_heading_pitch_roll_raw(angles);
    let prepared = prepare_factory_scientist_arrival(&world.entities, factory, id).unwrap();
    let result = apply_prepared_factory_scientist_arrival(
        &mut world.entities,
        &mut world.fx,
        &mut world.notifications,
        16,
        prepared,
    )
    .unwrap();
    assert_eq!(result.scientist_id, id);
    assert_eq!(result.factory_runtime.current_scientists_raw, 1);
    assert_eq!(world.entities.pending_factory_scientist_destroy_ids(), [id]);
    assert!(prepare_factory_scientist_arrival(&world.entities, factory, id).is_err());
    assert_eq!(
        world.entities.cleanup_pending_factory_scientist_destroys(),
        [id]
    );
}

#[v2k_test_support::retail_test]
fn playing_radial_uses_native_worker_class14_without_particle_hit_prefix() {
    for (level, kind) in [(19, 79), (40, 91)] {
        let mut world = World::load(level, false);
        let id = world.ids(kind)[0];
        world.adopt(false);
        let ids: Vec<_> = world.entities.iter_all().map(|e| e.id).collect();
        for other in ids {
            world
                .entities
                .entity_mut(other)
                .unwrap()
                .collision
                .state_flags_at_0x08
                .overwrite(0x8000, 0);
        }
        let origin = [100, 10_000, 300];
        let actor = world.entities.entity_mut(id).unwrap();
        actor
            .collision
            .state_flags_at_0x08
            .overwrite(0x68000, 0x68000);
        actor.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(0);
        actor.collision.last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(321);
        actor.set_motion_raw(origin, [0; 3]);
        let result = world.tasks.apply_playing_radial_damage(PlayingRadialFrame {
            extra_lives: v2k_game::entity_collision_state::RetailRuntimeValue::Unresolved,
            entities: &mut world.entities,
            player_hull: &mut v2k_game::player_hull::PlayerHull::default(),
            origin_raw: origin,
            template: KIND_27_RADIAL_TEMPLATE,
            world_fx: &mut world.fx,
            notifications: &mut world.notifications,
            retail_tick: 99,
            resources: &mut world.session.cache,
            static_damage: &mut world.static_damage,
            active_terminal_calls: Vec::new(),
        });
        assert!(result.blocked.is_none(), "{result:?}");
        assert!(result.completed_target_ids.contains(&id));
        assert_eq!(
            world.actor(id).collision.health_raw,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            world.actor(id).collision.last_hit_presentation_tick_at_0x34,
            RetailRuntimeValue::Known(321)
        );
        assert!(matches!(
            world.actor(id).actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::SharedRetarget(_))
        ));
    }
}
