use super::*;
use crate::{
    entity::{AuthoredWorldConstruction, EntityConstructionResources},
    entity_view_detail::{RetailViewDetailContext, VIEW_DETAIL_STATE_MASK},
    main_base_abort::MainBaseAbortQuietDeathBlock,
    session::GameSession,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
};

struct World {
    session: GameSession,
    manager: EntityManager,
    fx: WorldFx,
}

impl World {
    fn load(level_id: u32) -> Self {
        Self::load_with_exit_marker_bits(level_id, 0)
    }

    fn load_with_exit_marker_bits(level_id: u32, exit_marker_bits: u32) -> Self {
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
        let manager = EntityManager::from_authored_world(
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
                player_arrival: None,
                retail_tick: 0,
            }
            .with_exit_marker_bits(exit_marker_bits),
            &mut fx,
        )
        .unwrap();
        Self {
            session,
            manager,
            fx,
        }
    }

    fn first(&self, entity_type: u32) -> u32 {
        self.manager
            .iter_all()
            .find(|entity| entity.entity_type == entity_type)
            .unwrap()
            .id
    }

    fn tick(&mut self, owner: Class0ActorOwner, elapsed_micros: u32) -> Class0ActorTick {
        tick_class0_actor_owner(
            &mut self.manager,
            owner,
            Class0ActorFrame {
                resources: &self.session.cache,
                world_fx: &mut self.fx,
                elapsed_micros,
                retail_tick: 77,
            },
        )
    }

    // A source-callback oracle deliberately invokes DCA0/E870 after the outer
    // callback gate. It does not claim the authored defaults enable that gate.
    fn callback(&mut self, owner: Class0ActorOwner, dt: u32, detailed: bool) {
        let entity_type = self
            .manager
            .entity_mut(owner.entity_id())
            .unwrap()
            .entity_type;
        let metadata = self
            .manager
            .type_runtime_metadata(entity_type)
            .unwrap()
            .clone();
        run_callback(
            &mut self.manager,
            owner,
            &metadata,
            &mut Class0ActorFrame {
                resources: &self.session.cache,
                world_fx: &mut self.fx,
                elapsed_micros: dt,
                retail_tick: 77,
            },
            dt,
            detailed,
        )
        .unwrap();
    }
}

#[v2k_test_support::retail_test]
fn native_type111_ticks_both_relation_variants_and_follows_sound_across_callback_modes() {
    for visited in [false, true] {
        for detailed in [false, true] {
            for callback_enabled in [false, true] {
                let mut world =
                    World::load_with_exit_marker_bits(13, if visited { 0x1f0 } else { 0 });
                world.manager.cleanup_pending_actor_deferred_destroys();
                let id = world.first(111);
                assert_eq!(
                    world.manager.native_gate_self_relation(id),
                    (!visited).then_some(id)
                );
                let owner = Class0ActorOwner::adopt(&world.manager, id).unwrap();
                let entity = world.manager.entity_mut(id).unwrap();
                assert_eq!(entity.attached_to, None);
                assert_eq!(
                    bits(entity, 0x1000).unwrap(),
                    if visited { 0 } else { 0x1000 }
                );
                let old_position = entity.position_raw();
                let moved = [
                    old_position[0].wrapping_add(256),
                    old_position[1].wrapping_add(23),
                    old_position[2],
                ];
                entity.set_position_raw(moved);
                entity.collision.constructor_sound_follow_published = false;
                entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(5);
                entity.collision.state_flags_at_0x08.overwrite(
                    COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT
                        | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
                    if callback_enabled {
                        COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT
                    } else {
                        0
                    } | if detailed {
                        SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT
                    } else {
                        0
                    },
                );
                // Close both scheduler waits without a random draw so this
                // tests the DCA0/E870/callback-disabled dispatch directly.
                entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(250_000);
                entity.collision.callback_scheduler_accumulator_us_at_0x6c =
                    RetailRuntimeValue::Known(0);
                let mut expected_rng = world.fx.fork_for_main_base_abort_transaction();
                let tick = world.tick(owner, 125_001);
                assert_eq!(
                    tick.outcome,
                    Class0ActorOutcome::Advanced {
                        entity_id: id,
                        callback_enabled,
                        detailed,
                        callback_elapsed_micros: 125_000,
                    }
                );
                assert_eq!(tick.retained_owner, Some(owner));
                let entity = world.manager.entity_mut(id).unwrap();
                assert_eq!(
                    entity.position_raw(),
                    moved,
                    "Type111 never regrounds at DCA0"
                );
                assert_eq!(
                    entity.collision.constructor_sound_follow_position_raw,
                    RetailRuntimeValue::Known(moved)
                );
                assert!(entity.collision.constructor_sound_follow_published);
                assert_eq!(
                    entity.collision.animation_offset_at_0xb2,
                    RetailRuntimeValue::Known(0)
                );
                assert!(matches!(entity.actor_task_state(ActorTaskSlot::Primary),
                    Some(ActorTaskRuntime::Class0Timer(task)) if task.elapsed_ms() == if detailed && callback_enabled { 125 } else { 0 }));
                assert!(!world.manager.native_class0_has_pending_prefix(id));

                // At a genuine detailed timeout, +80=self keeps its timer;
                // a visited gate without the relation reselects and draws once.
                let timer_before = match world
                    .manager
                    .entity_mut(id)
                    .unwrap()
                    .actor_task_state(ActorTaskSlot::Primary)
                {
                    Some(ActorTaskRuntime::Class0Timer(task)) => task.elapsed_ms(),
                    _ => panic!("native class0 timer"),
                };
                world.callback(owner, (9_001 - timer_before) * 1_000, true);
                let current = Class0ActorOwner::adopt(&world.manager, id).unwrap();
                if visited {
                    assert_ne!(current.primary, owner.primary);
                    expected_rng.next_shared_retail_random_u16();
                } else {
                    assert_eq!(current, owner);
                    assert!(
                        matches!(world.manager.entity_mut(id).unwrap().actor_task_state(ActorTaskSlot::Primary),
                        Some(ActorTaskRuntime::Class0Timer(task)) if task.elapsed_ms() == 9_001)
                    );
                }
                assert_eq!(
                    world.fx.next_shared_retail_random_u16(),
                    expected_rng.next_shared_retail_random_u16()
                );
            }
        }
    }
}

#[v2k_test_support::retail_test]
fn native_authored_gate_keeps_class0_timer_dormant_and_clears_transient_mass() {
    let mut world = World::load(14);
    let mut scheduler = SpecializedActorTaskScheduler::new();
    let expected = world
        .manager
        .iter_all()
        .filter(|entity| world.manager.native_class0_construction_present(entity.id))
        .count();
    assert!(expected > 0);
    assert_eq!(scheduler.adopt_class0_actors(&world.manager), expected);
    assert_eq!(scheduler.adopt_class0_actors(&world.manager), 0);
    for entity_type in [52, 68] {
        let id = world.first(entity_type);
        let owner = Class0ActorOwner::adopt(&world.manager, id).unwrap();
        let entity = world.manager.entity_mut(id).unwrap();
        assert_eq!(
            bits(entity, COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT).unwrap(),
            0
        );
        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(123);
        let primary_id = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
        let Some(ActorTaskRuntime::Class0Timer(primary)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("class0 timer");
        };
        let primary = *primary;
        let before_position = entity.position_raw();
        let mass = entity.mass_raw;
        let tick = world.tick(owner, 125_001);
        assert!(matches!(
            tick.outcome,
            Class0ActorOutcome::Advanced {
                callback_enabled: false,
                ..
            }
        ));
        assert_eq!(tick.retained_owner, Some(owner));
        let entity = world.manager.entity_mut(id).unwrap();
        assert_eq!(
            entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
            primary_id
        );
        assert_eq!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(&ActorTaskRuntime::Class0Timer(primary))
        );
        assert_eq!(entity.position_raw(), before_position);
        assert_eq!(entity.mass_raw, mass);
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0)
        );
        assert!(!world.manager.native_class0_has_pending_prefix(id));
    }
}

#[v2k_test_support::retail_test]
fn coarse_callback_owns_no_timer_or_surface_tail() {
    let mut world = World::load(14);
    for entity_type in [52, 68] {
        let id = world.first(entity_type);
        let owner = Class0ActorOwner::adopt(&world.manager, id).unwrap();
        let entity = world.manager.entity_mut(id).unwrap();
        entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(77);
        let before_position = entity.position_raw();
        let mut expected_rng = world.fx.fork_for_main_base_abort_transaction();
        world.callback(owner, 9_001_000, false);
        let entity = world.manager.entity_mut(id).unwrap();
        assert!(
            matches!(entity.actor_task_state(ActorTaskSlot::Primary), Some(ActorTaskRuntime::Class0Timer(task)) if task.elapsed_ms() == 0)
        );
        assert_eq!(entity.position_raw(), before_position);
        assert_eq!(
            entity.surface_lifetime_timer_ms_at_0x48,
            RetailRuntimeValue::Known(77)
        );
        assert_eq!(bits(entity, 0x40000).unwrap(), 0);
        assert_eq!(
            world.fx.next_shared_retail_random_u16(),
            expected_rng.next_shared_retail_random_u16()
        );
    }
}

#[v2k_test_support::retail_test]
fn detailed_callback_reselects_living_singleton_once_and_preserves_wet_profile() {
    // Canonical L33 owns both profiles below its sea plane: Type68 spawn4
    // grounds at4000 and Type52 spawn5 at416+header08(200), with sea4096.
    let mut world = World::load(33);
    for entity_type in [52, 68] {
        let id = world.first(entity_type);
        let owner = Class0ActorOwner::adopt(&world.manager, id).unwrap();
        let entity = world.manager.entity_mut(id).unwrap();
        let position = entity.position_raw();
        assert!(
            position[1] < world.session.cache.terrain().unwrap().sea_level_raw(),
            "the authored callback oracle must retain a wet object"
        );
        entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(10_000);
        let model_extent = world
            .session
            .cache
            .global_model(entity.model_index.unwrap())
            .unwrap()
            .radius;
        let mut expected_rng = world.fx.fork_for_main_base_abort_transaction();
        world.callback(owner, 9_000_999, true);
        let entity = world.manager.entity_mut(id).unwrap();
        assert_eq!(
            entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
            Some(owner.primary)
        );
        assert_eq!(
            entity.surface_lifetime_timer_ms_at_0x48,
            RetailRuntimeValue::Known(1_000)
        );
        let expected_y = if entity_type == 52 {
            world
                .session
                .cache
                .terrain()
                .unwrap()
                .bilinear_height_raw(position[0], position[2])
                .wrapping_add(model_extent as i16)
        } else {
            position[1]
        };
        assert_eq!(entity.position_raw()[1], expected_y);
        world.callback(owner, 1_000, true);
        let current = Class0ActorOwner::adopt(&world.manager, id).unwrap();
        assert_ne!(current.primary, owner.primary);
        assert!(
            matches!(world.manager.entity_mut(id).unwrap().actor_task_state(ActorTaskSlot::Primary), Some(ActorTaskRuntime::Class0Timer(task)) if task.elapsed_ms() == 0)
        );
        let _singleton = expected_rng.next_shared_retail_random_u16();
        assert_eq!(
            world.fx.next_shared_retail_random_u16(),
            expected_rng.next_shared_retail_random_u16()
        );
    }
}

#[v2k_test_support::retail_test]
fn committed_live_prefix_parks_repeated_visit_and_quiet_abort() {
    let mut world = World::load(14);
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler.adopt_class0_actors(&world.manager);
    let id = world.first(68);
    let owner = Class0ActorOwner::adopt(&world.manager, id).unwrap();
    let entity = world.manager.entity_mut(id).unwrap();
    let before_age = entity.collision.recent_relation_elapsed_us_at_0x68;
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(99);
    entity.collision.constructor_sound_attachment_id_at_0x8c = RetailRuntimeValue::Unresolved;
    let tick = world.tick(owner, 125_001);
    assert!(matches!(
        tick.outcome,
        Class0ActorOutcome::Blocked {
            prefix_committed: true,
            reason: Class0ActorError::Runtime("positional sound attachment"),
            ..
        }
    ));
    assert!(world.manager.native_class0_has_pending_prefix(id));
    let entity = world.manager.entity_mut(id).unwrap();
    assert_ne!(
        entity.collision.recent_relation_elapsed_us_at_0x68,
        before_age
    );
    assert_eq!(
        entity.collision.animation_offset_at_0xb2,
        RetailRuntimeValue::Known(0)
    );
    let collision = entity.collision.clone();
    let position = entity.position_raw();
    let mut expected_rng = world.fx.fork_for_main_base_abort_transaction();
    scheduler.publish_presented_view_detail(
        &mut world.manager,
        RetailViewDetailContext::from_raw(
            [
                i32::from(position[0].wrapping_add(0x4000)),
                i32::from(position[1]),
                i32::from(position[2]),
            ],
            0,
            (52, 30),
        ),
    );
    assert_eq!(world.manager.entity_mut(id).unwrap().collision, collision);
    assert!(Class0ActorOwner::adopt(&world.manager, id).is_err());
    assert_eq!(
        world.tick(tick.retained_owner.unwrap(), 125_001).outcome,
        Class0ActorOutcome::Pending { entity_id: id }
    );
    assert_eq!(
        world
            .manager
            .apply_main_base_abort_quiet_death(owner.allocation(), &mut world.fx),
        Err(MainBaseAbortQuietDeathBlock::PendingNativePrefix)
    );
    assert_eq!(world.manager.entity_mut(id).unwrap().collision, collision);
    assert_eq!(
        world.fx.next_shared_retail_random_u16(),
        expected_rng.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn foreign_allocation_drops_without_consuming_shared_rng() {
    let first = World::load(14);
    let owner = Class0ActorOwner::adopt(&first.manager, first.first(68)).unwrap();
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler.adopt_class0_actors(&first.manager);
    let mut other = World::load(14);
    let entity = other.manager.entity_mut(owner.entity_id()).unwrap();
    let collision = entity.collision.clone();
    let position = entity.position_raw();
    scheduler.publish_presented_view_detail(
        &mut other.manager,
        RetailViewDetailContext::from_raw(
            [
                i32::from(position[0].wrapping_add(0x4000)),
                i32::from(position[1]),
                i32::from(position[2]),
            ],
            0,
            (52, 30),
        ),
    );
    assert_eq!(
        other
            .manager
            .entity_mut(owner.entity_id())
            .unwrap()
            .collision,
        collision
    );
    let mut expected_rng = other.fx.fork_for_main_base_abort_transaction();
    let tick = other.tick(owner, 125_001);
    assert_eq!(
        tick.outcome,
        Class0ActorOutcome::Dropped {
            entity_id: owner.entity_id()
        }
    );
    assert_eq!(tick.retained_owner, None);
    assert_eq!(
        other.fx.next_shared_retail_random_u16(),
        expected_rng.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn presented_weight_detail_controls_next_scheduler_wait_without_enabling_callback() {
    let mut world = World::load(14);
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler.adopt_class0_actors(&world.manager);
    let id = world.first(68);
    let owner = Class0ActorOwner::adopt(&world.manager, id).unwrap();
    let position = world.manager.entity_mut(id).unwrap().position_raw();
    let decoration_id = world.first(52);
    let decoration_state = world
        .manager
        .entity_mut(decoration_id)
        .unwrap()
        .collision
        .state_flags_at_0x08;
    let far = RetailViewDetailContext::from_raw(
        [
            i32::from(position[0].wrapping_add(0x4000)),
            i32::from(position[1]),
            i32::from(position[2]),
        ],
        0,
        (52, 30),
    );
    scheduler.publish_presented_view_detail(&mut world.manager, far);
    assert_eq!(
        bits(
            world.manager.entity_mut(id).unwrap(),
            VIEW_DETAIL_STATE_MASK
        )
        .unwrap(),
        0
    );
    assert_eq!(
        world
            .manager
            .entity_mut(decoration_id)
            .unwrap()
            .collision
            .state_flags_at_0x08,
        decoration_state,
        "Type52's authored 0x100 clears the independent 11400 admission bit"
    );
    let mut expected_rng = world.fx.fork_for_main_base_abort_transaction();
    let _subject_gate = expected_rng.next_shared_retail_random_u16();
    let tick = world.tick(owner, 125_001);
    assert!(matches!(
        tick.outcome,
        Class0ActorOutcome::Advanced {
            callback_enabled: false,
            detailed: false,
            ..
        }
    ));
    assert_eq!(world.fx.next_shared_retail_random_u16(), expected_rng.next_shared_retail_random_u16(),
        "the far visit owns one subject-gate draw; dt above125000 closes the callback wait without a draw");

    scheduler.publish_presented_view_detail(
        &mut world.manager,
        RetailViewDetailContext::from_raw(position.map(i32::from), 0, (52, 30)),
    );
    assert_eq!(
        bits(
            world.manager.entity_mut(id).unwrap(),
            VIEW_DETAIL_STATE_MASK
        )
        .unwrap(),
        VIEW_DETAIL_STATE_MASK
    );
    let mut expected_rng = world.fx.fork_for_main_base_abort_transaction();
    let tick = world.tick(tick.retained_owner.unwrap(), 1);
    assert!(matches!(
        tick.outcome,
        Class0ActorOutcome::Advanced {
            callback_enabled: false,
            detailed: true,
            ..
        }
    ));
    assert_eq!(
        world.fx.next_shared_retail_random_u16(),
        expected_rng.next_shared_retail_random_u16(),
        "near presentation disables both random waits without enabling the class0 callback"
    );
}
