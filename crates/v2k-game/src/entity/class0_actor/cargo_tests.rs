//! Real later-world beam/drop/settle through the native class0 callback owner.

use super::*;
use crate::{
    actor_task_owner::ActorTaskId,
    gameplay_notifications::GameplayNotifications,
    session::GameSession,
    specialized_actor_task_production::{
        SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
        SpecializedActorTaskScheduler,
    },
    static_damage::StaticDamageScheduler,
    terrain_crater::terrain_aligned_actor_basis,
};

struct World {
    session: GameSession,
    manager: EntityManager,
    scheduler: SpecializedActorTaskScheduler,
    fx: WorldFx,
    notifications: GameplayNotifications,
    tick: u32,
    weight: u32,
}

impl World {
    fn load(level_id: u32) -> Self {
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
            .map(|(id, _)| {
                EntityTypeRuntimeMetadata::from_section12(
                    session.cache.global_entity_type(id).unwrap(),
                )
            })
            .collect();
        let mut fx = WorldFx::new();
        let mut manager = EntityManager::from_authored_world(
            AuthoredWorldConstruction {
                logical_world_index: (level_id - 12) as i32,
                level: session.cache.level_desc().unwrap(),
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
                retail_tick: 0,
            },
            &mut fx,
        )
        .unwrap();
        let weight = manager
            .iter_all()
            .find(|entity| entity.entity_type == 68)
            .unwrap()
            .id;
        let position = manager.entity_mut(weight).unwrap().position;
        manager.player_mut().unwrap().position =
            [position[0] + 0.25, position[1] - 1., position[2]];
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert!(scheduler.adopt_class0_actors(&manager) > 0);
        Self {
            session,
            manager,
            scheduler,
            fx,
            notifications: GameplayNotifications::new(),
            tick: 1,
            weight,
        }
    }

    fn timer(&self) -> (ActorTaskId, u32) {
        let entity = self
            .manager
            .iter_all()
            .find(|entity| entity.id == self.weight)
            .unwrap();
        let id = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        let Some(ActorTaskRuntime::Class0Timer(timer)) = entity.actor_tasks.task_state(id) else {
            panic!("Type68 retains the real class0 timer, with no Sub-I carrying task");
        };
        (id, timer.elapsed_ms())
    }

    fn cargo_frame(&mut self) -> PlayerCargoFrameOutcome {
        let pass = self.manager.update_player_cargo(PlayerCargoFrame {
            elapsed_micros: 20_000,
            drop_context: Some(CargoDropContext {
                carrier_orientation: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
                terrain: self.session.cache.terrain().unwrap(),
            }),
            retail_tick: self.tick,
            scheduler: &mut self.scheduler,
            world_fx: &mut self.fx,
            notifications: &mut self.notifications,
        });
        self.tick += 1;
        pass
    }

    fn tick_actors(&mut self) -> Class0ActorOutcome {
        let mut static_damage = StaticDamageScheduler::new();
        let pass = self.scheduler.tick(
            &mut self.manager,
            SpecializedActorTaskProductionFrame {
                world:
                    crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                hive_components: None,
                notification_phase:
                    crate::gameplay_notifications::GameplayNotificationPhase::Playing,
                resources: &mut self.session.cache,
                world_fx: &mut self.fx,
                static_damage: &mut static_damage,
                elapsed_micros: 20_000,
                global_elapsed_micros: self.tick * 20_000,
                retail_tick: self.tick,
                main_base_abort_active: false,
            },
            &mut self.notifications,
        );
        assert!(pass.block.is_none(), "{:?}", pass.block);
        pass.outcomes
            .into_iter()
            .find_map(|outcome| match outcome {
                SpecializedActorTaskProductionOutcome::Class0Actor(outcome)
                    if outcome.entity_id() == self.weight =>
                {
                    Some(outcome)
                }
                _ => None,
            })
            .expect("the production scheduler retains the weight owner")
    }

    fn collect(&mut self) {
        self.manager.queue_beam(BeamCommand::Collect);
        for _ in 0..30 {
            let pass = self.cargo_frame();
            assert!(pass.blocked.is_empty(), "{:?}", pass.blocked);
            if pass.beam
                == Some(BeamOutcome::Collected {
                    entity_id: self.weight,
                })
            {
                return;
            }
        }
        panic!("actual authored weight must be collected");
    }

    fn place_for_drop(&mut self) {
        let position = self.manager.entity_mut(self.weight).unwrap().position;
        let x = position[0] + 2.;
        let z = position[2];
        let height = self.session.cache.terrain().unwrap().height_at(x, z);
        self.manager.player_mut().unwrap().position = [x, height - 1., z + 400. / 256.];
    }

    fn drop(&mut self) -> u32 {
        self.place_for_drop();
        self.manager.queue_beam(BeamCommand::Drop);
        for _ in 0..30 {
            let pass = self.cargo_frame();
            assert!(pass.blocked.is_empty(), "{:?}", pass.blocked);
            if let Some(BeamOutcome::DropStarted { cargo_id, proxy_id }) = pass.beam {
                assert_eq!(cargo_id, self.weight);
                return proxy_id;
            }
        }
        panic!("actual weight drop must construct and attach Type93");
    }

    fn assert_rng_words(&mut self, mut expected: WorldFx, count: usize) {
        for _ in 0..count {
            expected.next_shared_retail_random_u16();
        }
        assert_eq!(
            self.fx.next_shared_retail_random_u16(),
            expected.next_shared_retail_random_u16()
        );
    }
}

#[v2k_test_support::retail_test]
fn later_weight_keeps_timer_through_carry_and_reselects_each_normal_release() {
    for level_id in [14, 39] {
        let mut world = World::load(level_id);
        let before = world.timer();
        let original_stamp = world
            .manager
            .entity_mut(world.weight)
            .unwrap()
            .construction_stamp_at_0xb4;
        let before_drop_ordinal = world.manager.next_common_body_ordinal();
        let original_anchor = world.manager.native_class0_actors[&world.weight].anchor_raw;
        let original_angles = world
            .manager
            .entity_mut(world.weight)
            .unwrap()
            .rotation_heading_pitch_roll_raw();
        let expected_rng = world.fx.fork_for_main_base_abort_transaction();
        world.collect();
        assert_eq!(
            world.manager.next_common_body_ordinal(),
            before_drop_ordinal,
            "attaching an existing body cannot allocate a stamp"
        );
        world.assert_rng_words(expected_rng, 0);
        assert_eq!(
            world.timer(),
            before,
            "null DBF0 attach retains Primary and age"
        );
        assert_eq!(world.manager.attached_cargo_mass(), 200);
        assert!(world
            .scheduler
            .prepare_class0_actor_external_mutation(&world.manager, world.weight));
        assert!(matches!(
            world.tick_actors(),
            Class0ActorOutcome::Advanced { .. } | Class0ActorOutcome::Waiting { .. }
        ));
        assert_eq!(
            world.timer(),
            before,
            "authored callback gate stays dormant while carried"
        );

        let expected_rng = world.fx.fork_for_main_base_abort_transaction();
        let proxy = world.drop();
        let RetailRuntimeValue::Known(ordinal) = before_drop_ordinal else {
            panic!("ordinary native load owns explicit stamp lineage")
        };
        assert_eq!(
            world.manager.next_common_body_ordinal(),
            RetailRuntimeValue::Known(ordinal.wrapping_add(1))
        );
        assert_eq!(
            world
                .manager
                .entity_mut(proxy)
                .unwrap()
                .construction_stamp_at_0xb4,
            RetailRuntimeValue::Known(((level_id as i32 - 12) * 0x400 + i32::from(ordinal)) as u16)
        );
        assert_eq!(
            world
                .manager
                .entity_mut(world.weight)
                .unwrap()
                .construction_stamp_at_0xb4,
            original_stamp,
            "release/reselection retains the child's original body identity"
        );
        world.assert_rng_words(expected_rng, 2); // Type93 birth, then living D1C0/AC60.
        let proxy_timer = world.timer();
        assert_ne!(proxy_timer.0, before.0);
        assert_eq!(proxy_timer.1, 0);
        assert_eq!(
            world.manager.native_class0_actors[&world.weight].anchor_raw,
            original_anchor
        );
        let weight = world.manager.entity_mut(world.weight).unwrap();
        assert_eq!(weight.attached_to, Some(proxy));
        assert_eq!(weight.rotation_heading_pitch_roll_raw(), original_angles);
        assert_eq!(
            weight.collision.state_flags_at_0x08.masked(0x800 | 0x40000),
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            weight.physical_body_basis_q31,
            RetailRuntimeValue::Known(terrain_aligned_actor_basis(
                world.session.cache.terrain().unwrap(),
                weight.position_raw()
            ))
        );
        assert!(world
            .scheduler
            .prepare_class0_actor_external_mutation(&world.manager, world.weight));
        assert!(matches!(
            world.tick_actors(),
            Class0ActorOutcome::Advanced { .. } | Class0ActorOutcome::Waiting { .. }
        ));
        assert_eq!(
            world.timer(),
            proxy_timer,
            "null Type93 attach retains the newly selected timer"
        );

        world.manager.take_cargo_proxy_events();
        let expected_rng = world.fx.fork_for_main_base_abort_transaction();
        let mut released = None;
        for _ in 0..50 {
            let retail_tick = world.tick;
            let pass = world.cargo_frame();
            assert!(pass.blocked.is_empty(), "{:?}", pass.blocked);
            let blocked = world
                .manager
                .update_late_tail_materialisers(LateTailMaterialiserFrame {
                    elapsed_micros: 20_000,
                    terrain: world.session.cache.terrain().unwrap(),
                    world_fx: &mut world.fx,
                    scheduler: &mut world.scheduler,
                    notifications: &mut world.notifications,
                    retail_tick,
                });
            assert!(blocked.is_empty(), "{blocked:?}");
            for event in world.manager.take_cargo_proxy_events() {
                if let CargoProxyEvent::Released {
                    proxy_id,
                    cargo_id,
                    position,
                } = event
                {
                    assert_eq!((proxy_id, cargo_id), (proxy, world.weight));
                    released = Some(world_position_raw(position));
                }
            }
            if released.is_some() {
                break;
            }
        }
        let released = released.expect("strict Type93 settling tail releases the weight");
        world.assert_rng_words(expected_rng, 1); // 409030 ->16750 ->D1C0.
        assert_ne!(world.timer().0, proxy_timer.0);
        assert_eq!(world.timer().1, 0);
        assert_eq!(
            world.manager.native_class0_actors[&world.weight].anchor_raw,
            released
        );
        let weight = world.manager.entity_mut(world.weight).unwrap();
        assert_eq!(weight.position_raw(), released);
        assert_eq!(weight.attached_to, None);
        assert_eq!(weight.rotation_heading_pitch_roll_raw(), original_angles);
        assert_eq!(
            weight.physical_body_basis_q31,
            RetailRuntimeValue::Known(terrain_aligned_actor_basis(
                world.session.cache.terrain().unwrap(),
                released
            ))
        );
        assert!(world.manager.iter_all().all(|entity| entity.id != proxy));
        assert_eq!(
            world.manager.next_common_body_ordinal(),
            RetailRuntimeValue::Known(ordinal.wrapping_add(1)),
            "settling and removing the proxy do not allocate another stamp"
        );
        assert!(world
            .scheduler
            .prepare_class0_actor_external_mutation(&world.manager, world.weight));
        let age_before = world
            .manager
            .entity_mut(world.weight)
            .unwrap()
            .collision
            .recent_relation_elapsed_us_at_0x68;
        assert!(matches!(
            world.tick_actors(),
            Class0ActorOutcome::Advanced { .. } | Class0ActorOutcome::Waiting { .. }
        ));
        assert_ne!(
            world
                .manager
                .entity_mut(world.weight)
                .unwrap()
                .collision
                .recent_relation_elapsed_us_at_0x68,
            age_before
        );
        assert_eq!(
            world.scheduler.adopt_class0_actors(&world.manager),
            0,
            "release already registered the exact new owner"
        );
    }
}

#[v2k_test_support::retail_test]
fn genuinely_parked_class0_prefix_rejects_collection_drop_and_settle() {
    for phase in 0..3 {
        let mut world = World::load(14);
        let mut proxy = None;
        if phase >= 1 {
            world.collect();
            world.place_for_drop();
        }
        if phase == 2 {
            proxy = Some(world.drop());
        }
        world
            .manager
            .entity_mut(world.weight)
            .unwrap()
            .collision
            .constructor_sound_attachment_id_at_0x8c = RetailRuntimeValue::Unresolved;
        assert!(matches!(
            world.tick_actors(),
            Class0ActorOutcome::Blocked {
                prefix_committed: true,
                ..
            }
        ));
        assert!(world.manager.native_class0_has_pending_prefix(world.weight));
        let weight = world.manager.entity_mut(world.weight).unwrap();
        let collision = weight.collision.clone();
        let relation = weight.attached_to;
        let timer = world.timer();
        let next_id = world.manager.next_entity_id;
        let next_ordinal = world.manager.next_common_body_ordinal();
        let mut expected_rng = world.fx.fork_for_main_base_abort_transaction();
        if phase < 2 {
            world.manager.queue_beam(if phase == 0 {
                BeamCommand::Collect
            } else {
                BeamCommand::Drop
            });
        }
        let mut rejected = false;
        for _ in 0..50 {
            let retail_tick = world.tick;
            let pass = world.cargo_frame();
            rejected |= pass.blocked.iter().any(|(id, _)| *id == world.weight);
            let blocked = world
                .manager
                .update_late_tail_materialisers(LateTailMaterialiserFrame {
                    elapsed_micros: 20_000,
                    terrain: world.session.cache.terrain().unwrap(),
                    world_fx: &mut world.fx,
                    scheduler: &mut world.scheduler,
                    notifications: &mut world.notifications,
                    retail_tick,
                });
            rejected |= blocked.iter().any(|(id, _)| *id == world.weight);
        }
        assert!(rejected, "phase {phase} must surface the parked callback");
        assert_eq!(world.timer(), timer);
        let weight = world.manager.entity_mut(world.weight).unwrap();
        assert_eq!(weight.attached_to, relation);
        assert_eq!(weight.collision, collision);
        assert_eq!(
            world.manager.next_entity_id, next_id,
            "blocked drop cannot allocate/retry a proxy"
        );
        assert_eq!(
            world.manager.next_common_body_ordinal(),
            next_ordinal,
            "pending callback preflight never enters a Type93 body attempt"
        );
        assert_eq!(
            world.fx.next_shared_retail_random_u16(),
            expected_rng.next_shared_retail_random_u16()
        );
        if let Some(proxy) = proxy {
            assert!(world.manager.iter_all().any(|entity| entity.id == proxy));
            assert!(world
                .manager
                .take_cargo_proxy_events()
                .into_iter()
                .all(|event| !matches!(event, CargoProxyEvent::Released { .. })));
        }
    }
}

#[v2k_test_support::retail_test]
fn native_weight_release_requires_the_actual_parent_attachment_row() {
    let mut world = World::load(14);
    world.collect();
    let parent = world.manager.player_id.unwrap();
    let RetailRuntimeValue::Known(Some(runtime)) =
        &mut world.manager.player_mut().unwrap().sub_j_attachment_runtime
    else {
        panic!("real player Sub-J");
    };
    assert_eq!(runtime.pop_last(), Some(world.weight));
    let timer = world.timer();
    let collision = world
        .manager
        .entity_mut(world.weight)
        .unwrap()
        .collision
        .clone();
    let result = cargo::prepare_release(
        &world.manager,
        world.weight,
        parent,
        crate::ordinary_type9_cargo::Type9CargoReleasePosition::Retained,
        world.session.cache.terrain().unwrap(),
    );
    assert!(matches!(
        result,
        Err(Class0ActorError::Runtime("class0 release relation"))
    ));
    assert_eq!(world.timer(), timer);
    assert_eq!(
        world.manager.entity_mut(world.weight).unwrap().collision,
        collision
    );
}
