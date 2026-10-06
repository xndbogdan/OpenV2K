//! Canonical later-world Type93 construction and abnormal Sub-J teardown.

use super::*;
use crate::{
    gameplay_notifications::GameplayNotifications, session::GameSession,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
};

struct World {
    session: GameSession,
    manager: EntityManager,
    scheduler: SpecializedActorTaskScheduler,
    fx: WorldFx,
    notifications: GameplayNotifications,
    tick: u32,
}

impl World {
    fn load(level: u32) -> Self {
        let data = v2k_test_support::retail_dir();
        assert!(
            data.join("PRELOAD.DAT").is_file(),
            "normal-tier corpus required"
        );
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
        let mut manager = EntityManager::from_authored_world(
            AuthoredWorldConstruction {
                logical_world_index: (level - 12) as i32,
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
        // Complete the load's deferred gate-helper sweep before creating the
        // materialiser whose abnormal teardown this fixture exercises.
        manager.cleanup_pending_actor_deferred_destroys();
        assert!(!manager.fresh_new_game_first_world);
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler
            .adopt_fresh_level1_type9_selected(&mut manager)
            .unwrap();
        scheduler.adopt_intro2_type8(&mut manager);
        scheduler.adopt_class0_actors(&manager);
        Self {
            session,
            manager,
            scheduler,
            fx,
            notifications: GameplayNotifications::new(),
            tick: 1,
        }
    }

    fn cargo_frame(&mut self, elapsed_micros: u32) -> Option<BeamOutcome> {
        let pass = self.manager.update_player_cargo(PlayerCargoFrame {
            elapsed_micros,
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
        assert!(pass.blocked.is_empty(), "{:?}", pass.blocked);
        pass.beam
    }

    fn player_drop(&mut self, entity_type: u32) -> (u32, u32) {
        let cargo = self
            .manager
            .iter_all()
            .find(|entity| entity.entity_type == entity_type)
            .unwrap();
        let id = cargo.id;
        let position = cargo.position;
        self.manager.player_mut().unwrap().position =
            [position[0] + 0.25, position[1] - 1., position[2]];
        self.manager.queue_beam(BeamCommand::Collect);
        let collected = (0..30)
            .any(|_| self.cargo_frame(20_000) == Some(BeamOutcome::Collected { entity_id: id }));
        assert!(collected, "real type{entity_type} beam collection");
        let x = position[0] + 2.;
        let z = position[2];
        let height = self.session.cache.terrain().unwrap().height_at(x, z);
        self.manager.player_mut().unwrap().position = [x, height - 1., z + 400. / 256.];
        self.manager.queue_beam(BeamCommand::Drop);
        for _ in 0..30 {
            if let Some(outcome) = self.cargo_frame(20_000) {
                let BeamOutcome::DropStarted { cargo_id, proxy_id } = outcome else {
                    panic!("real drop constructor: {outcome:?}");
                };
                assert_eq!(cargo_id, id);
                return (proxy_id, id);
            }
        }
        panic!("real player drop must publish its Type93 constructor");
    }

    fn factory_output(&mut self, entity_type: u32) -> (u32, u32, u32) {
        let factory = self
            .manager
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
        let mut position_raw = factory.position_raw();
        position_raw[2] = position_raw[2].wrapping_sub(250);
        let request = FactoryEntitySpawnRequest {
            requested_handle: FactoryRequestedEntityHandle::Allocate,
            entity_type,
            position_raw,
            spawn_parameter_6: 0,
        };
        let output = self
            .manager
            .append_factory_converted_output(
                source,
                request,
                0,
                self.session.cache.terrain().unwrap(),
                &mut self.fx,
            )
            .unwrap();
        assert!(self
            .manager
            .link_factory_converted_output_owner(output, source));
        let proxy = self
            .manager
            .append_factory_output_materialiser(
                source,
                FactoryEntitySpawnRequest {
                    entity_type: 93,
                    ..request
                },
                self.session.cache.terrain().unwrap(),
                &mut self.fx,
            )
            .unwrap();
        let before_link = self
            .manager
            .main_base_abort_actor_observation(proxy.entity_id.get())
            .unwrap();
        assert_eq!(
            self.manager
                .apply_main_base_abort_type93_death(before_link.lease),
            Err(MainBaseType93DeathBlock::SidecarMismatch),
            "the published constructor alone cannot invent a completed 08F00 link"
        );
        assert!(self
            .manager
            .link_factory_materialiser_output(proxy, output, &mut self.fx));
        (
            proxy.entity_id.get(),
            output.entity_id.get(),
            source.entity_id,
        )
    }

    fn assert_abnormal_teardown(&mut self, proxy_id: u32, child_id: u32) {
        self.manager.take_cargo_proxy_events();
        self.fx.process_pending();
        self.fx.take_positional_sounds();
        let mut expected_rng = self.fx.fork_for_main_base_abort_transaction();
        let child = self.manager.entity_mut(child_id).unwrap();
        let child_collision = child.collision.clone();
        let child_context = child.current_behavior_context;
        let child_tasks =
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| child.actor_tasks.task_in_slot(slot));
        let observation = self
            .manager
            .main_base_abort_actor_observation(proxy_id)
            .unwrap();
        assert_eq!(
            self.manager
                .cargo_drop_proxies
                .iter()
                .find(|proxy| proxy.proxy_id == proxy_id)
                .unwrap()
                .construction,
            Type93MaterialiserConstruction::Native(observation.lease),
        );
        let result = self
            .manager
            .apply_main_base_abort_type93_death(observation.lease)
            .unwrap();
        assert!(matches!(result, MainBaseType93DeathAdvance::Advanced {
            outcome: MainBaseType93DeathOutcome::DeferredDestroyStaged { entity_id, cargo_id }, ..
        } if entity_id == proxy_id && cargo_id == child_id));
        let proxy = self.manager.entity_mut(proxy_id).unwrap();
        assert_eq!(proxy.collision.health_raw, RetailRuntimeValue::Known(0));
        assert_eq!(
            proxy.sub_j_attachment_runtime,
            RetailRuntimeValue::Known(Some(type93_attachment_runtime(child_id)))
        );
        let child = self.manager.entity_mut(child_id).unwrap();
        assert_eq!(child.attached_to, Some(proxy_id));
        assert_eq!(child.collision, child_collision);
        assert_eq!(child.current_behavior_context, child_context);
        assert_eq!(
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| child.actor_tasks.task_in_slot(slot)),
            child_tasks
        );
        assert_eq!(
            self.manager.pending_actor_deferred_destroy_ids(),
            [proxy_id]
        );
        let sidecar = *self
            .manager
            .cargo_drop_proxies
            .iter()
            .find(|proxy| proxy.proxy_id == proxy_id)
            .unwrap();
        assert!(!sidecar.task_active);
        // Even a full settling interval cannot run09030 after class2 clears
        // the materialiser task. Abnormal090B0 must not reselect the child.
        assert_eq!(self.cargo_frame(1_000_000), None);
        assert!(self
            .manager
            .update_late_tail_materialisers(LateTailMaterialiserFrame {
                elapsed_micros: 1_000_000,
                terrain: self.session.cache.terrain().unwrap(),
                world_fx: &mut self.fx,
                scheduler: &mut self.scheduler,
                notifications: &mut self.notifications,
                retail_tick: self.tick,
            })
            .is_empty());
        let retained = self
            .manager
            .cargo_drop_proxies
            .iter()
            .find(|proxy| proxy.proxy_id == proxy_id)
            .unwrap();
        assert_eq!(retained.stable_us, sidecar.stable_us);
        assert!(self.manager.take_cargo_proxy_events().is_empty());
        assert_eq!(
            self.fx.next_shared_retail_random_u16(),
            expected_rng.next_shared_retail_random_u16()
        );
        assert!(self.fx.take_positional_sounds().is_empty());
        assert!(matches!(
            self.manager
                .apply_main_base_abort_type93_death(observation.lease)
                .unwrap(),
            MainBaseType93DeathAdvance::Advanced {
                outcome: MainBaseType93DeathOutcome::AlreadyDyingNoOp { .. },
                ..
            }
        ));
        assert_eq!(
            self.manager.cleanup_pending_actor_deferred_destroys(),
            [proxy_id, child_id]
        );
        assert!(self
            .manager
            .iter_all()
            .all(|entity| ![proxy_id, child_id].contains(&entity.id)));
        assert!(self
            .manager
            .cargo_drop_proxies
            .iter()
            .all(|proxy| proxy.proxy_id != proxy_id));
        assert!(self.manager.pending_actor_deferred_destroy_ids().is_empty());
    }
}

#[v2k_test_support::retail_test]
fn later_player_materialisers_abort_shared_constructor_for_peasant_worker_and_weight() {
    for (level, cargo_type) in [(14, 9), (25, 8), (39, 68)] {
        let mut world = World::load(level);
        let (proxy, child) = world.player_drop(cargo_type);
        world.assert_abnormal_teardown(proxy, child);
    }
}

#[v2k_test_support::retail_test]
fn later_factory_materialiser_abort_retains_proof_after_source_factory_death() {
    let mut world = World::load(14);
    let (proxy, child, factory) = world.factory_output(8);
    crate::intro2_type66::death::publish_intro2_type66_standard_death(
        &mut world.manager,
        factory,
        &mut world.fx,
    )
    .unwrap();
    world.assert_abnormal_teardown(proxy, child);
}

#[v2k_test_support::retail_test]
fn type7_factory_materialiser_retains_sub_j_and_child_graph_when_scheduler_is_parked() {
    let mut world = World::load(34);
    let (proxy, child, _) = world.factory_output(7);
    assert_eq!(world.scheduler.adopt_native_type86(&mut world.manager), 6);
    world.scheduler.park_native_type86_external_prefix(child);
    let entity = world.manager.entity_mut(child).unwrap();
    let before = (
        entity.native_type86_runtime,
        entity.current_behavior_context,
        entity
            .actor_tasks
            .task_in_slot(crate::actor_task_owner::ActorTaskSlot::Primary),
        entity.collision.clone(),
        entity.position_raw(),
        entity.actor_animation_runtime,
    );
    let mut oracle = world.fx.fork_for_main_base_abort_transaction();
    let mut blocks = Vec::new();
    for tick in 0..40 {
        blocks.extend(
            world
                .manager
                .update_late_tail_materialisers(LateTailMaterialiserFrame {
                    elapsed_micros: 20_000,
                    terrain: world.session.cache.terrain().unwrap(),
                    world_fx: &mut world.fx,
                    scheduler: &mut world.scheduler,
                    notifications: &mut world.notifications,
                    retail_tick: tick,
                }),
        );
    }
    assert!(!blocks.is_empty());
    assert!(
        blocks
            .iter()
            .all(|(id, reason)| *id == child && reason.contains("four-choice task callback")),
        "{blocks:?}"
    );
    let entity = world.manager.entity_mut(child).unwrap();
    assert_eq!(entity.attached_to, Some(proxy));
    assert_eq!(
        (
            entity.native_type86_runtime,
            entity.current_behavior_context,
            entity
                .actor_tasks
                .task_in_slot(crate::actor_task_owner::ActorTaskSlot::Primary),
            entity.collision.clone(),
            entity.position_raw(),
            entity.actor_animation_runtime
        ),
        before
    );
    assert!(
        matches!(&world.manager.entity_mut(proxy).unwrap().sub_j_attachment_runtime,
        RetailRuntimeValue::Known(Some(rows)) if rows.ordered_entity_ids() == [child])
    );
    assert!(!world
        .manager
        .take_cargo_proxy_events()
        .iter()
        .any(|event| matches!(event, CargoProxyEvent::Released { .. })));
    assert_eq!(
        world.fx.next_shared_retail_random_u16(),
        oracle.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn type93_native_abort_rejects_stale_and_projection_proof_before_any_prefix() {
    let mut world = World::load(14);
    let (proxy, child) = world.player_drop(9);
    let observation = world
        .manager
        .main_base_abort_actor_observation(proxy)
        .unwrap();
    let original = world.manager.cargo_drop_proxies[0].construction;
    let proxy_collision = world.manager.entity_mut(proxy).unwrap().collision.clone();
    let child_collision = world.manager.entity_mut(child).unwrap().collision.clone();
    let proxy_context = world
        .manager
        .entity_mut(proxy)
        .unwrap()
        .current_behavior_context;
    for (construction, expected) in [
        (
            Type93MaterialiserConstruction::Native(MainBaseAbortActorLease {
                allocation_identity: observation.lease.allocation_identity ^ (1_u64 << 32),
                ..observation.lease
            }),
            MainBaseType93DeathBlock::ConstructorReceiptMismatch,
        ),
        (
            Type93MaterialiserConstruction::AttachmentProjection,
            MainBaseType93DeathBlock::UnsupportedConstructionProvenance,
        ),
    ] {
        world.manager.cargo_drop_proxies[0].construction = construction;
        assert_eq!(
            world
                .manager
                .apply_main_base_abort_type93_death(observation.lease),
            Err(expected)
        );
        assert_eq!(
            world.manager.entity_mut(proxy).unwrap().collision,
            proxy_collision
        );
        assert_eq!(
            world.manager.entity_mut(child).unwrap().collision,
            child_collision
        );
        assert_eq!(
            world
                .manager
                .entity_mut(proxy)
                .unwrap()
                .current_behavior_context,
            proxy_context
        );
        assert!(world.manager.cargo_drop_proxies[0].task_active);
        assert!(world
            .manager
            .pending_actor_deferred_destroy_ids()
            .is_empty());
    }
    world.manager.cargo_drop_proxies[0].construction = original;
    world.assert_abnormal_teardown(proxy, child);
}
