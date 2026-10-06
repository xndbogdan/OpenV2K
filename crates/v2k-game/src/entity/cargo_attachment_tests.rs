//! Canonical later-world443B50 controls, independent of campaign identity lookup.

use super::*;
use crate::{
    actor_task_owner::ActorTaskId,
    session::GameSession,
    world_fx::{ParticleBirthContext, ParticleEnvironment, TerrainCollisionContext},
};

struct World {
    session: GameSession,
    manager: EntityManager,
    scheduler: SpecializedActorTaskScheduler,
    fx: WorldFx,
    notifications: GameplayNotifications,
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
        manager
            .player_sub_j_runtime_mut()
            .unwrap()
            .set_capacity_clamped(2);
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert!(scheduler.adopt_class0_actors(&manager) > 0);
        // A real timer with a nonzero age makes a copied/replaced constructor
        // task detectably different from DBF0's null attach callback.
        let entity = manager.entity_mut(weight).unwrap();
        let primary = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        let Some(ActorTaskRuntime::Class0Timer(timer)) = entity.actor_tasks.task_state_mut(primary)
        else {
            panic!("native class0 timer");
        };
        assert!(!timer.advance_prefix(123_000));
        fx.clear();
        Self {
            session,
            manager,
            scheduler,
            fx,
            notifications: GameplayNotifications::new(),
            weight,
        }
    }

    fn timer(&self) -> (ActorTaskId, u32) {
        let entity = self
            .manager
            .iter_all()
            .find(|entity| entity.id == self.weight)
            .unwrap();
        let primary = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        let Some(ActorTaskRuntime::Class0Timer(timer)) = entity.actor_tasks.task_state(primary)
        else {
            panic!("native class0 timer");
        };
        (primary, timer.elapsed_ms())
    }

    fn attach(&mut self, tick: u32) -> Result<(), PlayerCargoAttachmentError> {
        self.manager.attach_player_cargo(
            self.weight,
            PlayerCargoAttachmentFrame {
                scheduler: &mut self.scheduler,
                world_fx: &mut self.fx,
                notifications: &mut self.notifications,
                retail_tick: tick,
            },
        )
    }

    fn child_flags(&self, mask: u32) -> RetailRuntimeValue<u32> {
        self.manager
            .iter_all()
            .find(|entity| entity.id == self.weight)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .masked(mask)
    }

    fn flush(&mut self, tick: u32) {
        self.fx
            .process_deferred_cargo_transfers(ParticleBirthContext {
                environment: ParticleEnvironment::Terrain(
                    TerrainCollisionContext::from_current_level_cache(&self.session.cache).unwrap(),
                ),
                retail_tick: tick,
            });
    }
}

#[v2k_test_support::retail_test]
fn native_weight_attachment_keeps_timer_and_body_stamp_and_commits_success_suffix() {
    for level_id in [14, 39] {
        let mut world = World::load(level_id);
        let timer = world.timer();
        let stamp = world
            .manager
            .entity_mut(world.weight)
            .unwrap()
            .construction_stamp_at_0xb4;
        let ordinal = world.manager.next_common_body_ordinal();
        let mut rng = world.fx.fork_for_main_base_abort_transaction();
        let child = world.manager.entity_mut(world.weight).unwrap();
        child
            .collision
            .state_flags_at_0x08
            .overwrite(0x42800, 0x42800);
        // Runtime capability controls the independently recovered event0 gate.
        child.capability_flags |= 0x800;
        world.attach(71).unwrap();
        assert_eq!(world.timer(), timer);
        assert_eq!(
            world
                .manager
                .entity_mut(world.weight)
                .unwrap()
                .construction_stamp_at_0xb4,
            stamp
        );
        assert_eq!(world.manager.next_common_body_ordinal(), ordinal);
        assert_eq!(
            world.child_flags(0x43800),
            RetailRuntimeValue::Known(0x1000)
        );
        assert_eq!(
            world
                .manager
                .player_sub_j_runtime()
                .unwrap()
                .ordered_entity_ids(),
            &[world.weight]
        );
        let parent = world.manager.player_id;
        assert_eq!(
            world.manager.entity_mut(world.weight).unwrap().attached_to,
            parent
        );
        assert!(world
            .scheduler
            .prepare_class0_actor_external_mutation(&world.manager, world.weight));
        let mut expected = GameplayNotifications::new();
        expected.queue_cargo_collected(71);
        expected.queue_weight_collected(71);
        assert_eq!(
            world.notifications, expected,
            "event15 follows event0 at the same source tick"
        );
        assert_eq!(world.fx.pending_event_count(), 2);
        assert_eq!(world.fx.particle_count(), 0);
        world.fx.process_pending();
        assert_eq!(
            world.fx.particle_count(),
            0,
            "ordinary particle admission must preserve deferred birth"
        );
        world.flush(71);
        assert_eq!(world.fx.particle_count(), 1);
        assert_eq!(world.fx.take_positional_sounds().len(), 1);
        assert_eq!(
            world.fx.next_shared_retail_random_u16(),
            rng.next_shared_retail_random_u16()
        );
    }
}

#[v2k_test_support::retail_test]
fn duplicate_native_weight_reenters_null_callback_but_full_slot_fails_before_duplicate() {
    let mut world = World::load(14);
    world.attach(11).unwrap();
    let timer = world.timer();
    let notifications = world.notifications.clone();
    world.attach(22).unwrap();
    assert_eq!(world.timer(), timer);
    assert_eq!(world.manager.player_sub_j_runtime().unwrap().len(), 1);
    assert_eq!(
        world.notifications, notifications,
        "dedup keeps the first timestamp"
    );
    assert_eq!(
        world.fx.pending_event_count(),
        4,
        "each successful call submits transfer effects"
    );
    world
        .manager
        .player_sub_j_runtime_mut()
        .unwrap()
        .set_capacity_clamped(1);
    assert_eq!(world.attach(33), Err(PlayerCargoAttachmentError::CargoFull));
    assert_eq!(world.fx.pending_event_count(), 4);
    assert_eq!(world.timer(), timer);
}

#[v2k_test_support::retail_test]
fn unavailable_completed_callback_retains_slot_and_relation_without_success_suffix() {
    let mut world = World::load(14);
    let child = world.manager.entity_mut(world.weight).unwrap();
    child
        .collision
        .state_flags_at_0x08
        .overwrite(0x42800, 0x42800);
    child.actor_tasks.clear_slot(ActorTaskSlot::Primary);
    assert!(matches!(
        world.attach(55),
        Err(PlayerCargoAttachmentError::CallbackUnavailable {
            prefix_committed: true,
            ..
        })
    ));
    assert_eq!(
        world
            .manager
            .player_sub_j_runtime()
            .unwrap()
            .ordered_entity_ids(),
        &[world.weight]
    );
    let parent = world.manager.player_id;
    assert_eq!(
        world.manager.entity_mut(world.weight).unwrap().attached_to,
        parent
    );
    assert_eq!(
        world.child_flags(0x43800),
        RetailRuntimeValue::Known(0x43000)
    );
    assert_eq!(world.notifications, GameplayNotifications::new());
    assert_eq!(world.fx.pending_event_count(), 0);
    assert!(world
        .manager
        .entity_mut(world.weight)
        .unwrap()
        .actor_task_state(ActorTaskSlot::Primary)
        .is_none());
}

#[v2k_test_support::retail_test]
fn pending_native_callback_is_a_host_admission_block_before_relation_writes() {
    let mut world = World::load(14);
    let flags = world
        .manager
        .entity_mut(world.weight)
        .unwrap()
        .collision
        .state_flags_at_0x08;
    let timer = world.timer();
    assert!(world
        .manager
        .set_native_class0_pending_prefix(world.weight, true));
    assert_eq!(
        world.attach(66),
        Err(PlayerCargoAttachmentError::PendingCallback)
    );
    assert_eq!(world.manager.player_sub_j_runtime().unwrap().len(), 0);
    assert_eq!(
        world.manager.entity_mut(world.weight).unwrap().attached_to,
        None
    );
    assert_eq!(
        world
            .manager
            .entity_mut(world.weight)
            .unwrap()
            .collision
            .state_flags_at_0x08,
        flags
    );
    assert_eq!(world.timer(), timer);
    assert_eq!(world.fx.pending_event_count(), 0);
    assert_eq!(world.notifications, GameplayNotifications::new());
}

#[v2k_test_support::retail_test]
fn player_without_capability_one_skips_only_transfer_and_child2000_clear() {
    let mut world = World::load(14);
    world.manager.player_mut().unwrap().capability_flags &= !1;
    world
        .manager
        .entity_mut(world.weight)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(0x42000, 0x42000);
    world.attach(77).unwrap();
    assert_eq!(
        world.child_flags(0x42000),
        RetailRuntimeValue::Known(0x2000)
    );
    let mut expected = GameplayNotifications::new();
    expected.queue_weight_collected(77);
    assert_eq!(world.notifications, expected);
    assert_eq!(world.fx.pending_event_count(), 0);
}

#[v2k_test_support::retail_test]
fn production_beam_uses_the_same_transfer_owner_exactly_once() {
    let mut world = World::load(14);
    let position = world.manager.entity_mut(world.weight).unwrap().position;
    world.manager.player_mut().unwrap().position =
        [position[0] + 0.25, position[1] - 1., position[2]];
    let timer = world.timer();
    world.manager.queue_beam(BeamCommand::Collect);
    let mut collected_tick = None;
    for tick in 1..=30 {
        let result = world.manager.update_player_cargo(PlayerCargoFrame {
            elapsed_micros: 20_000,
            drop_context: Some(CargoDropContext {
                carrier_orientation: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
                terrain: world.session.cache.terrain().unwrap(),
            }),
            retail_tick: tick,
            scheduler: &mut world.scheduler,
            world_fx: &mut world.fx,
            notifications: &mut world.notifications,
        });
        assert!(result.blocked.is_empty(), "{:?}", result.blocked);
        if result.beam
            == Some(BeamOutcome::Collected {
                entity_id: world.weight,
            })
        {
            collected_tick = Some(tick);
            break;
        }
    }
    let tick = collected_tick.expect("real authored weight is collected");
    let mut expected = GameplayNotifications::new();
    expected.queue_weight_collected(tick as i32);
    assert_eq!(world.notifications, expected);
    assert_eq!(world.timer(), timer);
    assert_eq!(world.fx.pending_event_count(), 2);
    world.fx.process_pending();
    world.flush(tick);
    world.fx.process_pending();
    world.flush(tick);
    assert_eq!(world.fx.particle_count(), 1);
    assert_eq!(world.fx.take_positional_sounds().len(), 1);
}

#[v2k_test_support::retail_test]
fn real_type93_drop_attachment_does_not_reenter_the_player_collection_suffix() {
    let mut world = World::load(14);
    world.attach(1).unwrap();
    let notifications = world.notifications.clone();
    world.fx.clear();
    let position = world.manager.entity_mut(world.weight).unwrap().position;
    let x = position[0] + 2.;
    let terrain_y = world
        .session
        .cache
        .terrain()
        .unwrap()
        .height_at(x, position[2]);
    world.manager.player_mut().unwrap().position = [x, terrain_y - 1., position[2] + 400. / 256.];
    world.manager.queue_beam(BeamCommand::Drop);
    let mut proxy = None;
    for tick in 2..=31 {
        let result = world.manager.update_player_cargo(PlayerCargoFrame {
            elapsed_micros: 20_000,
            drop_context: Some(CargoDropContext {
                carrier_orientation: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
                terrain: world.session.cache.terrain().unwrap(),
            }),
            retail_tick: tick,
            scheduler: &mut world.scheduler,
            world_fx: &mut world.fx,
            notifications: &mut world.notifications,
        });
        assert!(result.blocked.is_empty(), "{:?}", result.blocked);
        if let Some(BeamOutcome::DropStarted { cargo_id, proxy_id }) = result.beam {
            assert_eq!(cargo_id, world.weight);
            proxy = Some(proxy_id);
            break;
        }
    }
    let proxy = proxy.expect("real native Type93 drop");
    assert_eq!(
        world.manager.entity_mut(world.weight).unwrap().attached_to,
        Some(proxy)
    );
    assert!(world
        .scheduler
        .prepare_class0_actor_external_mutation(&world.manager, world.weight));
    assert_eq!(world.notifications, notifications);
    assert_eq!(
        world.fx.pending_event_count(),
        0,
        "Type93 presentation uses its own proxy events"
    );
    world.flush(32);
    assert_eq!(world.fx.particle_count(), 0);
    assert!(world.fx.take_positional_sounds().is_empty());
}
