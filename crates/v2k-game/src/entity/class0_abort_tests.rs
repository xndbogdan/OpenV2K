//! Native class-0 constructor custody at the shared quiet-death boundary.

use super::*;
use crate::actor_task_owner::ActorTaskWrapperFlags;
use crate::session::GameSession;

struct World {
    manager: EntityManager,
    fx: WorldFx,
}

impl World {
    fn load(level_id: u32) -> Self {
        Self::load_with_spawn_edit(level_id, |_| {})
    }

    fn load_with_spawn_edit(level_id: u32, edit: impl FnOnce(&mut LevelDescriptor)) -> Self {
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
        let bytes =
            std::fs::read(data.join("Overlay").join(format!("1X{level_id}XX.OVL"))).unwrap();
        let overlay = v2k_formats::ovl::OvlFile::parse(&bytes).unwrap();
        let mut level =
            v2k_formats::levels::parse_level(&overlay.section(13).unwrap().data).unwrap();
        edit(&mut level);
        let mut fx = WorldFx::new();
        let mut manager = EntityManager::from_authored_world(
            AuthoredWorldConstruction {
                logical_world_index: (level_id - 12) as i32,
                level: &level,
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
            },
            &mut fx,
        )
        .unwrap();
        // Enter the post-load stage: 42EFB0's duplicate/Hive marker removals
        // precede the quiet-death operation whose queue this fixture checks.
        manager.cleanup_pending_actor_deferred_destroys();
        Self { manager, fx }
    }

    fn first(&self, entity_type: u32) -> u32 {
        self.manager
            .iter_all()
            .find(|entity| entity.entity_type == entity_type)
            .unwrap_or_else(|| panic!("authored type{entity_type} required"))
            .id
    }

    fn assert_blocked(&mut self, id: u32, expected: MainBaseAbortQuietDeathBlock) {
        let observation = self.manager.main_base_abort_actor_observation(id).unwrap();
        let entity = self
            .manager
            .entities
            .iter()
            .find(|entity| entity.id == id)
            .unwrap();
        let collision = entity.collision.clone();
        let context = entity.current_behavior_context;
        let slots = ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| {
            let task = entity.actor_tasks.task_in_slot(slot);
            (
                task,
                entity.actor_task_state(slot).cloned(),
                task.and_then(|task| entity.actor_tasks.wrapper_flags(task)),
            )
        });
        let pending = self.manager.pending_actor_deferred_destroy_ids().to_vec();
        let events = self.fx.pending_event_count();
        let mut expected_rng = self.fx.fork_for_main_base_abort_transaction();
        assert_eq!(
            self.manager
                .apply_main_base_abort_quiet_death(observation.lease, &mut self.fx),
            Err(expected)
        );
        let entity = self
            .manager
            .entities
            .iter()
            .find(|entity| entity.id == id)
            .unwrap();
        assert_eq!(entity.collision, collision);
        assert_eq!(entity.current_behavior_context, context);
        assert_eq!(
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| {
                let task = entity.actor_tasks.task_in_slot(slot);
                (
                    task,
                    entity.actor_task_state(slot).cloned(),
                    task.and_then(|task| entity.actor_tasks.wrapper_flags(task)),
                )
            }),
            slots
        );
        assert_eq!(self.manager.pending_actor_deferred_destroy_ids(), pending);
        assert_eq!(self.fx.pending_event_count(), events);
        assert_eq!(
            self.fx.next_shared_retail_random_u16(),
            expected_rng.next_shared_retail_random_u16()
        );
    }

    fn assert_death(&mut self, id: u32) {
        assert!(self.manager.native_class0_allocation_authenticates(id));
        let entity = self
            .manager
            .entities
            .iter()
            .find(|entity| entity.id == id)
            .unwrap();
        let entity_type = entity.entity_type;
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::Class0Timer(_))
        ));
        let death_sound_id = if entity_type == 68 { Some(62) } else { None };
        let events = self.fx.pending_event_count();
        let mut expected_rng = self.fx.fork_for_main_base_abort_transaction();
        let order: Vec<_> = self.manager.retail_live_order_ids().collect();
        let expected_successor = order
            .iter()
            .position(|candidate| *candidate == id)
            .and_then(|index| order.get(index + 1))
            .copied();
        let observation = self.manager.main_base_abort_actor_observation(id).unwrap();
        let result = self
            .manager
            .apply_main_base_abort_quiet_death(observation.lease, &mut self.fx)
            .unwrap();
        let MainBaseAbortQuietDeathAdvance::Advanced {
            outcome,
            next_actor,
        } = result
        else {
            panic!("post-callback successor remains authenticated");
        };
        assert_eq!(
            outcome,
            MainBaseAbortQuietDeathOutcome::DeferredDestroyStaged {
                entity_id: id,
                entity_type,
                death_sound_id
            }
        );
        assert_eq!(
            next_actor.map(|actor| actor.lease.entity_id),
            expected_successor
        );
        let entity = self
            .manager
            .entities
            .iter()
            .find(|entity| entity.id == id)
            .unwrap();
        assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
        assert_eq!(
            entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT),
            RetailRuntimeValue::Known(DYING_STATE_BIT)
        );
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!("class2 publication")
        };
        assert_eq!(
            context.descriptor(),
            BehaviorDescriptorIdentity::Named(&QUIET_DEATH_BEHAVIOR_PROGRAM)
        );
        assert_eq!(
            context.active_style(),
            ActiveBehaviorStyle::Audited(QUIET_DEATH_STYLE)
        );
        assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .all(|slot| entity.actor_tasks.task_in_slot(slot).is_none()));
        assert!(self
            .manager
            .pending_actor_deferred_destroy_ids()
            .contains(&id));
        assert_eq!(
            self.fx.pending_event_count(),
            events + usize::from(death_sound_id.is_some())
        );
        assert_eq!(
            self.fx.next_shared_retail_random_u16(),
            expected_rng.next_shared_retail_random_u16()
        );
        let again = self
            .manager
            .apply_main_base_abort_quiet_death(observation.lease, &mut self.fx)
            .unwrap();
        assert!(matches!(again, MainBaseAbortQuietDeathAdvance::Advanced {
            outcome: MainBaseAbortQuietDeathOutcome::AlreadyDyingNoOp { entity_id, .. }, ..
        } if entity_id == id));
        assert_eq!(
            self.fx.pending_event_count(),
            events + usize::from(death_sound_id.is_some())
        );
    }
}

#[v2k_test_support::retail_test]
fn later_world_native_class0_actors_commit_quiet_death_and_deferred_unlink() {
    for (level_id, types) in [(14, &[52, 68][..]), (25, &[68][..]), (39, &[68][..])] {
        let mut world = World::load(level_id);
        assert!(!world.manager.fresh_new_game_first_world);
        let ids: Vec<_> = types
            .iter()
            .map(|&entity_type| world.first(entity_type))
            .collect();
        for &id in &ids {
            world.assert_death(id);
        }
        let removed = world.manager.cleanup_pending_actor_deferred_destroys();
        assert_eq!(removed.len(), ids.len());
        assert!(ids.into_iter().all(|id| removed.contains(&id)));
        assert!(world
            .manager
            .pending_actor_deferred_destroy_ids()
            .is_empty());
    }
}

#[v2k_test_support::retail_test]
fn native_quiet_death_preserves_authored_model_overrides_and_pose() {
    let mut world = World::load_with_spawn_edit(14, |level| {
        for entity_type in [52, 68] {
            let spawn = level
                .entities
                .iter_mut()
                .find(|spawn| spawn.entity_type == entity_type)
                .unwrap();
            spawn.model_overrides = [145, 81, 145, 81];
            spawn.rotation = [0x1234, 0x5678, 0x9abc];
        }
    });
    for entity_type in [52, 68] {
        let id = world.first(entity_type);
        let entity = world
            .manager
            .entities
            .iter()
            .find(|entity| entity.id == id)
            .unwrap();
        assert_eq!(
            entity.model_slots,
            [Some(145), Some(81), Some(145), Some(81)]
        );
        let position = entity.position;
        let orientation = entity.rotation_heading_pitch_roll_raw();
        let body_basis = entity.physical_body_basis_q31;
        world.assert_death(id);
        let entity = world
            .manager
            .entities
            .iter()
            .find(|entity| entity.id == id)
            .unwrap();
        assert_eq!(entity.position, position);
        assert_eq!(entity.rotation_heading_pitch_roll_raw(), orientation);
        assert_eq!(entity.physical_body_basis_q31, body_basis);
    }
}

#[v2k_test_support::retail_test]
fn stale_native_class0_receipt_cannot_fall_back_to_first_world_replay() {
    let mut world = World::load(13);
    let id = world.first(52);
    assert!(world.manager.native_class0_allocation_authenticates(id));
    world.manager.allocation_generation = world.manager.allocation_generation.wrapping_add(1);
    assert!(world.manager.native_class0_construction_present(id));
    assert!(!world.manager.native_class0_allocation_authenticates(id));
    world.assert_blocked(
        id,
        MainBaseAbortQuietDeathBlock::NativeConstructorReceiptMismatch,
    );
}

#[v2k_test_support::retail_test]
fn native_class0_quiet_death_requires_exact_live_nonexecuting_task_custody() {
    for case in 0..4 {
        let mut world = World::load(14);
        let id = world.first(68);
        let entity = world.manager.entity_mut(id).unwrap();
        let primary = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        match case {
            0 => entity.actor_tasks.clear_slot(ActorTaskSlot::Primary),
            1 => {
                let state = entity.actor_tasks.task_state(primary).unwrap().clone();
                entity
                    .actor_tasks
                    .replace_prepared(ActorTaskSlot::Secondary, PreparedActorTask::new(state));
            }
            2 => entity.actor_tasks.set_wrapper_flags_for_test(
                primary,
                ActorTaskWrapperFlags {
                    alive: true,
                    in_callback: true,
                },
            ),
            3 => entity.actor_tasks.set_wrapper_flags_for_test(
                primary,
                ActorTaskWrapperFlags {
                    alive: false,
                    in_callback: false,
                },
            ),
            _ => unreachable!(),
        }
        world.assert_blocked(id, MainBaseAbortQuietDeathBlock::UnexpectedPublishedTask);
    }
}

#[v2k_test_support::retail_test]
fn native_class0_quiet_death_rejects_a_parked_prefix_after_wrapper_unwind() {
    let mut world = World::load(14);
    let id = world.first(68);
    // Negative fixture for the manager marker maintained by the live owner.
    // The task is otherwise complete/nonexecuting; wrapper flags alone cannot
    // prove that a consumed outer prefix has finished.
    assert!(world.manager.set_native_class0_pending_prefix(id, true));
    world.assert_blocked(id, MainBaseAbortQuietDeathBlock::PendingNativePrefix);
    assert!(world.manager.set_native_class0_pending_prefix(id, false));
    world.assert_death(id);
}

#[v2k_test_support::retail_test]
fn native_quiet_death_noops_precede_task_custody_and_never_repeat_the_prefix() {
    for (remote, dying) in [(true, true), (false, true)] {
        let mut world = World::load(14);
        let id = world.first(68);
        assert!(world.manager.set_native_class0_pending_prefix(id, true));
        let entity = world.manager.entity_mut(id).unwrap();
        entity.actor_tasks.clear_slot(ActorTaskSlot::Primary);
        entity.collision.state_flags_at_0x08.overwrite(
            REMOTE_OWNED_STATE_BIT | DYING_STATE_BIT,
            u32::from(remote) * REMOTE_OWNED_STATE_BIT | u32::from(dying) * DYING_STATE_BIT,
        );
        let collision = entity.collision.clone();
        let context = entity.current_behavior_context;
        let events = world.fx.pending_event_count();
        let observation = world.manager.main_base_abort_actor_observation(id).unwrap();
        let MainBaseAbortQuietDeathAdvance::Advanced { outcome, .. } = world
            .manager
            .apply_main_base_abort_quiet_death(observation.lease, &mut world.fx)
            .unwrap()
        else {
            panic!("source no-op")
        };
        let expected = if remote {
            MainBaseAbortQuietDeathOutcome::RemoteOwnedNoOp {
                entity_id: id,
                entity_type: 68,
            }
        } else {
            MainBaseAbortQuietDeathOutcome::AlreadyDyingNoOp {
                entity_id: id,
                entity_type: 68,
            }
        };
        assert_eq!(outcome, expected);
        let entity = world.manager.entity_mut(id).unwrap();
        assert_eq!(entity.collision, collision);
        assert_eq!(entity.current_behavior_context, context);
        assert_eq!(world.fx.pending_event_count(), events);
        assert!(world
            .manager
            .pending_actor_deferred_destroy_ids()
            .is_empty());
    }
}

#[v2k_test_support::retail_test]
fn native_class0_quiet_death_rejects_partial_context_metadata_and_sound_state() {
    for case in 0..7 {
        let mut world = World::load(14);
        let id = world.first(68);
        let expected = match case {
            0 => {
                world
                    .manager
                    .entity_mut(id)
                    .unwrap()
                    .current_behavior_context = RetailRuntimeValue::Unresolved;
                MainBaseAbortQuietDeathBlock::CurrentBehaviorContextMismatch
            }
            1 => {
                world.manager.type_metadata[68].common_mover_topology =
                    RetailRuntimeValue::Known(CommonMoverComponentTopology {
                        sub_a: true,
                        ..CommonMoverComponentTopology::default()
                    });
                MainBaseAbortQuietDeathBlock::TypeMetadataMismatch
            }
            2 => {
                world
                    .manager
                    .entity_mut(id)
                    .unwrap()
                    .collision
                    .death_sound_id = RetailRuntimeValue::Known(None);
                MainBaseAbortQuietDeathBlock::DeathSoundStateMismatch
            }
            3 => {
                world.manager.type_metadata[68].constructor_sound_attachment_id =
                    RetailRuntimeValue::Known(Some(9));
                MainBaseAbortQuietDeathBlock::ConstructorSoundAttachmentNotExactNull
            }
            4 => {
                let program = behavior_program(0).unwrap();
                world
                    .manager
                    .entity_mut(id)
                    .unwrap()
                    .current_behavior_context =
                    RetailRuntimeValue::Known(BehaviorContextRuntime::named_audited(
                        program,
                        program.initial_style_table_index_raw,
                        RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
                        RetailRuntimeValue::Unresolved,
                        RetailRuntimeValue::Known(0),
                        program.initial_style,
                    ));
                MainBaseAbortQuietDeathBlock::CurrentBehaviorContextMismatch
            }
            5 | 6 => {
                world
                    .manager
                    .entity_mut(id)
                    .unwrap()
                    .collision
                    .constructor_sound_attachment_id_at_0x8c = if case == 5 {
                    RetailRuntimeValue::Known(Some(9))
                } else {
                    RetailRuntimeValue::Unresolved
                };
                MainBaseAbortQuietDeathBlock::ConstructorSoundAttachmentNotExactNull
            }
            _ => unreachable!(),
        };
        world.assert_blocked(id, expected);
    }
}
