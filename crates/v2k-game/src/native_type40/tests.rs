//! Authenticated canonical Type40 records and own class7/9 graph.
use super::*;
#[v2k_test_support::retail_test]
fn canonical_type40_has_own_emitter_bank_and_absent_j() {
    let (session, _) = crate::native_type122::construction_tests::fixture();
    let m =
        EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(40).unwrap());
    authenticate_metadata(&m).unwrap();
    assert_eq!(
        m.projectile_emitter_descriptor,
        RetailRuntimeValue::Known(Some(EMITTER))
    );
    assert_eq!(
        m.sub_j_attachment_descriptor,
        RetailRuntimeValue::Known(None)
    );
}

use crate::{
    actor_task_dispatcher::ActorTaskRuntime as Task,
    actor_task_owner::ActorTaskSlot as Slot,
    entity_collision_state::{DYING_STATE_BIT, REMOTE_OWNED_STATE_BIT},
    specialized_actor_task_production::SpecializedActorTaskScheduler,
};
fn parent(manager: &EntityManager) -> u32 {
    manager.iter_all().find(|e| e.entity_type == 40).unwrap().id
}
#[v2k_test_support::retail_test]
fn all_eight_authored_births_own_class9_and_exact_local_components() {
    let (mut session, metadata) = crate::native_type122::construction_tests::fixture();
    let mut fx = WorldFx::new();
    let mut count = 0;
    for world in [17, 19, 32, 45] {
        session.load_level_by_id(world, 1).unwrap();
        let manager = crate::native_type122::construction_tests::construct(
            &session, &metadata, world, &mut fx,
        );
        for e in manager.iter_all().filter(|e| e.entity_type == 40) {
            count += 1;
            assert!(manager_allocation_authenticates(&manager, e.id));
            assert!(Type40Owner::adopt(&manager, e.id).is_ok());
            let r = e.native_type40_runtime.unwrap();
            assert_eq!(r.allocation.entity_id, e.id);
            assert_eq!(e.authored_spawn_index, Some(r.spawn_index));
            assert_eq!(e.position_raw(), r.anchor_raw);
            assert_eq!(e.collision.health_raw, RetailRuntimeValue::Known(8000));
            assert_eq!(
                e.collision.default_state_flags_at_0xc8,
                RetailRuntimeValue::Known(0x39)
            );
            assert_eq!(e.sub_j_attachment_runtime, RetailRuntimeValue::Known(None));
            assert_eq!(r.sub_e_runtime, emitter_constructor());
            let RetailRuntimeValue::Known(Some(context)) = e.current_behavior_context else {
                panic!("source context")
            };
            assert_eq!(context.active_style().style_address(), 0x004c7ff0);
            assert!(matches!(
                e.actor_task_state(Slot::Primary),
                Some(Task::SharedRetarget(_))
            ));
            assert!(matches!(
                e.actor_task_state(Slot::Secondary),
                Some(Task::TargetAcquisition(_))
            ));
            assert!(e.actor_task_state(Slot::Tertiary).is_none());
            let RetailRuntimeValue::Known(Some(h)) = &e.sub_h_external_frame_runtime else {
                panic!("source H")
            };
            assert_eq!(h.records().len(), 14);
            assert!(h.is_enabled());
            assert!(r.split_terminal.is_none());
            assert!(e.native_type56_runtime.is_none());
            assert!(e.native_type122_runtime.is_none());
        }
    }
    assert_eq!(count, 8);
}
#[v2k_test_support::retail_test]
fn class18_commits_real_children_inline_and_consumes_105_ordered_words_once() {
    let (session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(19);
    let id = parent(&manager);
    let before: Vec<_> = manager.iter_all().map(|e| e.id).collect();
    assert!(before.len() + 8 <= 80);
    let mut expected = fx.fork_for_main_base_abort_transaction();
    for _ in 0..105 {
        expected.next_shared_retail_random_u16();
    }
    let mut tasks = SpecializedActorTaskScheduler::default();
    tasks.adopt_type40(&manager);
    let result = death::begin_type40_standard_death(
        &mut manager,
        id,
        death::Type40Class18Frame {
            resources: &session.cache,
            world_fx: &mut fx,
            retail_tick: 4794,
            tasks: &mut tasks,
        },
    )
    .unwrap();
    assert!(result.returned_nonzero);
    tasks.register_native_ground_terminal(result.publication.unwrap());
    assert!(death::finished_terminal_authenticates(&manager, id));
    assert!(Type40Owner::adopt(&manager, id).is_err());
    let parent = manager.iter_all().find(|e| e.id == id).unwrap();
    let split = parent
        .native_type40_runtime
        .unwrap()
        .split_terminal
        .unwrap();
    assert_eq!(
        split.status,
        death::Type40SplitStatus::Completed(
            crate::split_and_explode::SplitAndExplodeCompletion::ChildrenComplete
        )
    );
    assert_eq!(
        split.progress.native_entity_list_count_raw,
        Some(before.len() as i32)
    );
    assert_eq!(
        (
            split.progress.attempted_children,
            split.progress.created_children,
            split.progress.launch_rng_words
        ),
        (8, 8, 72)
    );
    assert!(split.progress.tasks_cleared && split.progress.source_deferred_destroyed);
    let children: Vec<_> = manager
        .iter_all()
        .filter(|e| !before.contains(&e.id))
        .collect();
    assert_eq!(children.len(), 8);
    assert_eq!(
        tasks.adopt_type56(&manager),
        0,
        "every child already has synchronous task custody"
    );
    for child in children {
        assert_eq!(child.entity_type, 56);
        assert!(crate::native_type56::manager_allocation_authenticates(
            &manager, child.id
        ));
        assert!(crate::native_type56::Type56Owner::adopt(&manager, child.id).is_ok());
        let birth = child.native_type56_runtime.unwrap().birth();
        assert_eq!(birth.requested_entity_handle_raw, 0);
        assert_eq!(
            birth.objective,
            parent.collision.state_flags_at_0x08.masked(0x01000000)
                == RetailRuntimeValue::Known(0x01000000)
        );
        assert_eq!(child.velocity_raw(), birth.velocity_raw);
        assert_eq!(
            child.rotation_heading_pitch_roll_raw(),
            birth.rotation_heading_pitch_roll_raw.map(|w| w as i16)
        );
        assert_eq!(
            child.collision.pre_health_damage_buffer_raw,
            RetailRuntimeValue::Known(0)
        );
    }
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
    let mut no_replay = fx.fork_for_main_base_abort_transaction();
    let linked = manager.iter_all().count();
    let repeat = death::begin_type40_standard_death(
        &mut manager,
        id,
        death::Type40Class18Frame {
            resources: &session.cache,
            world_fx: &mut fx,
            retail_tick: 4795,
            tasks: &mut tasks,
        },
    )
    .unwrap();
    assert!(repeat.returned_nonzero && repeat.publication.is_none());
    assert_eq!(manager.iter_all().count(), linked);
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        no_replay.next_shared_retail_random_u16()
    );
}
#[v2k_test_support::retail_test]
fn native_full_list_cap_includes_parent_and_all_real_allocations_without_extra_sentinel() {
    for (count, children, words) in [(79, 1, 14), (80, 0, 1)] {
        let (session, mut manager, mut fx) =
            crate::native_type122::construction_tests::native_fixture_with_player(19);
        let id = parent(&manager);
        while manager.iter_all().count() < count {
            let request = crate::split_and_explode::SplitChildRequest {
                requested_entity_handle_raw: 0,
                entity_type: 56,
                position_raw: manager.player().unwrap().position_raw(),
                objective: false,
                velocity_raw: [0; 3],
                rotation_heading_pitch_roll_raw: [0; 3],
            };
            manager
                .construct_native_type56(request, &session.cache, &mut fx, 4793)
                .unwrap();
        }
        assert_eq!(manager.iter_all().count(), count);
        let mut expected = fx.fork_for_main_base_abort_transaction();
        for _ in 0..words {
            expected.next_shared_retail_random_u16();
        }
        let mut tasks = SpecializedActorTaskScheduler::default();
        death::begin_type40_standard_death(
            &mut manager,
            id,
            death::Type40Class18Frame {
                resources: &session.cache,
                world_fx: &mut fx,
                retail_tick: 4794,
                tasks: &mut tasks,
            },
        )
        .unwrap();
        assert_eq!(manager.iter_all().count(), count + children);
        let split = manager
            .entity_mut(id)
            .unwrap()
            .native_type40_runtime
            .unwrap()
            .split_terminal
            .unwrap();
        assert_eq!(
            split.progress.native_entity_list_count_raw,
            Some(count as i32)
        );
        assert_eq!(split.progress.created_children, children);
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            expected.next_shared_retail_random_u16()
        );
    }
}
#[v2k_test_support::retail_test]
fn remote_standard_death_returns_zero_before_source_tasks_rng_or_split() {
    let (session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(19);
    let id = parent(&manager);
    let e = manager.entity_mut(id).unwrap();
    e.collision
        .state_flags_at_0x08
        .overwrite(REMOTE_OWNED_STATE_BIT, REMOTE_OWNED_STATE_BIT);
    let context = e.current_behavior_context;
    let tasks_before = Slot::IN_RETAIL_TICK_ORDER.map(|s| e.actor_tasks.task_in_slot(s));
    let mut expected = fx.fork_for_main_base_abort_transaction();
    let mut tasks = SpecializedActorTaskScheduler::default();
    let result = death::begin_type40_standard_death(
        &mut manager,
        id,
        death::Type40Class18Frame {
            resources: &session.cache,
            world_fx: &mut fx,
            retail_tick: 4794,
            tasks: &mut tasks,
        },
    )
    .unwrap();
    assert!(!result.returned_nonzero && result.publication.is_none());
    let e = manager.entity_mut(id).unwrap();
    assert_eq!(e.collision.health_raw, RetailRuntimeValue::Known(8000));
    assert_eq!(
        e.collision.state_flags_at_0x08.masked(DYING_STATE_BIT),
        RetailRuntimeValue::Known(0)
    );
    assert_eq!(e.current_behavior_context, context);
    assert_eq!(
        Slot::IN_RETAIL_TICK_ORDER.map(|s| e.actor_tasks.task_in_slot(s)),
        tasks_before
    );
    assert!(e.native_type40_runtime.unwrap().split_terminal.is_none());
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
}

struct RejectChildPublication;
impl crate::intro2_type17::capture::CaptureTaskCustody for RejectChildPublication {
    fn capture_child_mutation_ready(&mut self, _: &EntityManager, _: u32) -> bool {
        false
    }
    fn mutate_capture_child(
        &mut self,
        _: &mut EntityManager,
        _: crate::intro2_type17::capture::CaptureChildFrame<'_>,
        _: &mut dyn FnMut(
            &mut EntityManager,
        ) -> Result<(), crate::intro2_type17::capture::CaptureBlock>,
    ) -> Result<(), crate::intro2_type17::capture::CaptureBlock> {
        Err(crate::intro2_type17::capture::CaptureBlock::new(
            "test rejects unrelated capture",
        ))
    }
}
impl shared::NativeGroundTaskCustody for RejectChildPublication {
    fn register_split_type56_child(
        &mut self,
        _: crate::native_type56::Type56Owner,
    ) -> Result<(), &'static str> {
        Err("test rejects child publication after its genuine constructor")
    }
    fn register_split_rolling_boulder_child(
        &mut self,
        _: crate::rolling_boulder::RollingBoulderOwner,
    ) -> Result<(), &'static str> {
        Err("test rejects child publication after its genuine constructor")
    }
}
#[v2k_test_support::retail_test]
fn blocked_child_publication_retains_completed_constructor_and_never_replays_prefix() {
    let (session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(19);
    let id = parent(&manager);
    let before = manager.iter_all().count();
    let mut expected = fx.fork_for_main_base_abort_transaction();
    for _ in 0..14 {
        expected.next_shared_retail_random_u16();
    }
    let mut tasks = RejectChildPublication;
    let result = death::begin_type40_standard_death(
        &mut manager,
        id,
        death::Type40Class18Frame {
            resources: &session.cache,
            world_fx: &mut fx,
            retail_tick: 4794,
            tasks: &mut tasks,
        },
    );
    assert!(matches!(
        result,
        Err(death::Type40DeathBlock::Split(
            crate::split_and_explode::SplitAndExplodeBlock::Host {
                phase: crate::split_and_explode::SplitAndExplodePhase::ChildConstruction,
                reason: death::Type40SplitHostBlock::Runtime(
                    "test rejects child publication after its genuine constructor"
                ),
            }
        ))
    ));
    assert_eq!(
        manager.iter_all().count(),
        before + 1,
        "constructor appended the actual child before returning its task owner"
    );
    let child = manager.iter_all().last().unwrap();
    assert!(crate::native_type56::Type56Owner::adopt(&manager, child.id).is_ok());
    assert!(!manager.pending_actor_deferred_destroy_ids().contains(&id));
    let split = manager
        .entity_mut(id)
        .unwrap()
        .native_type40_runtime
        .unwrap()
        .split_terminal
        .unwrap();
    assert_eq!(
        split.status,
        death::Type40SplitStatus::Blocked(
            crate::split_and_explode::SplitAndExplodePhase::ChildConstruction
        )
    );
    assert!(split.progress.tasks_cleared);
    assert_eq!(
        (
            split.progress.attempted_children,
            split.progress.created_children,
            split.progress.launch_rng_words
        ),
        (1, 0, 9)
    );
    assert!(!split.progress.source_deferred_destroyed);
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
    let mut expected = fx.fork_for_main_base_abort_transaction();
    let repeat = death::begin_type40_standard_death(
        &mut manager,
        id,
        death::Type40Class18Frame {
            resources: &session.cache,
            world_fx: &mut fx,
            retail_tick: 4795,
            tasks: &mut tasks,
        },
    );
    assert!(matches!(repeat, Err(death::Type40DeathBlock::Graph)));
    assert_eq!(manager.iter_all().count(), before + 1);
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn real_c690_hit_reselection_installs_own_class7_method1_then_fifo_survives_split() {
    use crate::{
        common_mover::{
            component_dispatch::CommonMoverDispatchMode, type9_attitude::Type9BodyBasis,
        },
        generic_projectile_emitter::GenericEmitterSpeedField,
        world_fx::ParticleEnvironment,
    };
    let (session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(19);
    let id = parent(&manager);
    let player = manager.player().unwrap().id;
    let target = manager.entity_mut(player).unwrap();
    target.set_position_raw([0, 1000, 1000]);
    target.set_velocity_raw([0; 3]);
    target.collision.state_flags_at_0x08.overwrite(u32::MAX, 4);
    let e = manager.entity_mut(id).unwrap();
    e.set_position_raw([0, 1000, 0]);
    e.set_velocity_raw([100, 0, -200]);
    e.set_rotation_heading_pitch_roll_raw([0x4000, 0, 0]);
    e.physical_body_basis_q31 =
        RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(0x4000, 0, 0));
    e.collision.state_flags_at_0x08.overwrite(u32::MAX, 4);
    e.collision.last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(4794);
    // At tick4794 the actual UnderAttack rule weights both branches1. Find an
    // even word in the live process stream; do not install a captured RNG state.
    for _ in 0..65536 {
        let mut peek = fx.fork_for_main_base_abort_transaction();
        if peek.next_shared_retail_random_u16() % 2 == 0 {
            break;
        }
        fx.next_shared_retail_random_u16();
    }
    shared::behavior::reselect::<profile::Type40Profile>(
        &mut manager,
        id,
        4794,
        &mut fx,
        shared::behavior::ReselectionEntry::Impact,
    )
    .unwrap();
    let e = manager.entity_mut(id).unwrap();
    let RetailRuntimeValue::Known(Some(context)) = e.current_behavior_context else {
        panic!("class7 context")
    };
    assert_eq!(context.active_style().style_address(), 0x004c7a50);
    shared::behavior::secondary::<profile::Type40Profile>(&mut manager, id, 20_000, 4794, &mut fx)
        .unwrap();
    assert!(shared::search::pursuing_graph_authenticates::<
        profile::Type40Profile,
    >(manager.entity_mut(id).unwrap()));
    assert!(Type40Owner::adopt(&manager, id).is_ok());
    let metadata = manager.type_runtime_metadata(40).unwrap().clone();
    for _ in 0..65536 {
        let mut peek = fx.fork_for_main_base_abort_transaction();
        if u32::from(peek.next_shared_retail_random_u16()) % (600_000u32 / 125_000u32) == 0 {
            break;
        }
        fx.next_shared_retail_random_u16();
    }
    let outcome = aim::tick_type40_aim(
        CommonMoverDispatchMode::Normal,
        &mut manager,
        &mut fx,
        id,
        125_000,
        Some(&metadata),
    )
    .unwrap();
    assert!(outcome.queued_shots_added > 0, "{outcome:?}");
    let queue = manager
        .entity_mut(id)
        .unwrap()
        .native_type40_aim_runtime
        .clone()
        .unwrap();
    assert!(queue
        .transient_shots()
        .iter()
        .all(|shot| shot.projectile_method == 1
            && shot.source_handle == id
            && shot.owner_handle == id
            && shot.speed_field == GenericEmitterSpeedField::Explicit(4000)));
    let mut tasks = SpecializedActorTaskScheduler::default();
    death::begin_type40_standard_death(
        &mut manager,
        id,
        death::Type40Class18Frame {
            resources: &session.cache,
            world_fx: &mut fx,
            retail_tick: 4794,
            tasks: &mut tasks,
        },
    )
    .unwrap();
    assert_eq!(
        manager
            .entity_mut(id)
            .unwrap()
            .native_type40_aim_runtime
            .as_ref(),
        Some(&queue)
    );
    let drained =
        aim::drain_type40_shots(&mut manager, &mut fx, id, ParticleEnvironment::Dry, 4794).unwrap();
    assert_eq!(
        drained.materialized_particle_classes,
        vec![1; queue.queued_shot_count()]
    );
    assert!(
        aim::drain_type40_shots(&mut manager, &mut fx, id, ParticleEnvironment::Dry, 4794)
            .unwrap()
            .materialized_particle_classes
            .is_empty()
    );
}
#[v2k_test_support::retail_test]
fn class9_acquires_real_target_but_never_borrows_j_or_carry_styles() {
    let (_session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(19);
    let id = parent(&manager);
    let person = manager
        .iter_all()
        .find(|entity| entity.entity_type == 79)
        .expect("world19's genuine native person")
        .id;
    assert_ne!(person, manager.player().unwrap().id);
    let ids = manager.retail_live_order_ids().collect::<Vec<_>>();
    for actor in ids {
        manager
            .entity_mut(actor)
            .unwrap()
            .set_motion_raw([16000, 16000, 16000], [0; 3]);
    }
    let position = [4096, 6000, 4096];
    manager.entity_mut(id).unwrap().set_position_raw(position);
    let target = manager.entity_mut(person).unwrap();
    // C910/185C0 uses best_distance==0 as its initial-candidate sentinel.
    // Keep this genuine person near, rather than coincident, so a later
    // distant person cannot replace it through that source zero-distance rule.
    target.set_position_raw([position[0] + 16, position[1], position[2]]);
    target.set_velocity_raw([0; 3]);
    assert!(target.capability_flags & 0xc00 != 0);
    shared::behavior::secondary::<profile::Type40Profile>(&mut manager, id, 20_000, 4794, &mut fx)
        .unwrap();
    let e = manager.entity_mut(id).unwrap();
    let RetailRuntimeValue::Known(Some(context)) = e.current_behavior_context else {
        panic!("native class9")
    };
    assert_eq!(context.active_style().style_address(), 0x004c8038);
    assert_eq!(
        context.target_handle_at_0x08(),
        RetailRuntimeValue::Known(Some(person))
    );
    assert!(
        matches!(e.actor_task_state(Slot::Primary),Some(Task::CapturePeoplePursuit(task)) if task.target_id()==Some(person))
    );
    assert!(
        e.actor_task_state(Slot::Secondary).is_none()
            && e.actor_task_state(Slot::Tertiary).is_none()
    );
    assert_eq!(e.sub_j_attachment_runtime, RetailRuntimeValue::Known(None));
    assert!(Type40Owner::adopt(&manager, id).is_ok());
    assert!(!crate::native_actor_capture::carry_tasks::carrying_variant(
        manager.entity_mut(id).unwrap()
    )
    .is_some());
}

#[v2k_test_support::retail_test]
fn fresh_104b0_constructor_four_word_order_does_not_evaluate_unused_nearby_rules() {
    use crate::{
        common_mover::{
            sub_d::{construct_native_sub_d, SubDAllocationCounter},
            SubAPropulsionRuntime,
        },
        entity::EntityConstructionResources,
    };
    let (mut session, metadata) = crate::native_type122::construction_tests::fixture();
    session.load_level_by_id(19, 1).unwrap();
    for tick in [0, 249, 250, 4793, u32::MAX] {
        let mut manager = crate::native_type122::construction_tests::generic(&session, &metadata);
        let id = parent(&manager);
        let index = manager
            .entity_mut(id)
            .unwrap()
            .authored_spawn_index
            .unwrap();
        let mut spawn = session.cache.level_desc().unwrap().entities[index].clone();
        spawn.rotation = [0x9000, 0x0800, 0xfc00];
        spawn.initial_damage_buffer_raw = 137;
        let allocation = manager.main_base_abort_actor_observation(id).unwrap().lease;
        let e = manager.entity_mut(id).unwrap();
        e.set_rotation_heading_pitch_roll_raw(spawn.rotation.map(|w| w as i16));
        e.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(137);
        let RetailRuntimeValue::Known(Some(d)) = metadata[40].sub_d_steering_descriptor else {
            panic!("source D")
        };
        let sub_d = construct_native_sub_d(&mut SubDAllocationCounter::from_next_seed(0xd3), d);
        let mut draws = [0x5300, 0xffff, 0x1234, 0x9876].into_iter();
        let result = publish_native_type40(
            NativeType40ConstructionRequest {
                entity: e,
                allocation,
                metadata: &metadata[40],
                spawn: &spawn,
                preceding: &[],
                resources: EntityConstructionResources::new(
                    session.cache.terrain(),
                    session.cache.terrain_objects(),
                ),
                constructor_surface_bits: 0,
                retail_tick: tick,
                sub_d,
            },
            &mut || {
                draws
                    .next()
                    .expect("20450,425680,Secondary406070,Primary406070")
            },
        )
        .unwrap();
        assert!(draws.next().is_none());
        assert_eq!(
            result.selection.program.class_id, 9,
            "fresh+34=0 is never under attack at tick{tick}"
        );
        assert_eq!(result.selector_word, 0xffff);
        assert!(!result.initializer_fallback);
        let e = manager.entity_mut(id).unwrap();
        let RetailRuntimeValue::Known(Some(a)) = e.sub_a_propulsion_runtime else {
            panic!("source A")
        };
        assert_eq!(
            a.target_speed_raw(),
            RetailRuntimeValue::Known(crate::common_mover::shared_initializer_target_speed_raw(
                200, 0x9876
            ))
        );
        assert_eq!(a.direction_multiplier(), 1);
        let r = e.native_type40_runtime.unwrap();
        assert_eq!(r.sub_d_runtime, sub_d.runtime);
        assert_eq!(r.sub_d_owner, sub_d.frame_owner);
        assert_eq!(
            e.collision.pre_health_damage_buffer_raw,
            RetailRuntimeValue::Known(137)
        );
        assert_eq!(
            e.rotation_heading_pitch_roll_raw(),
            [0x9000u16 as i16, 0x0800, 0xfc00u16 as i16]
        );
        assert_eq!(e.sub_j_attachment_runtime, RetailRuntimeValue::Known(None));
        let RetailRuntimeValue::Known(Some(descriptor)) = metadata[40].sub_a_propulsion_descriptor
        else {
            unreachable!()
        };
        assert_ne!(
            e.sub_a_propulsion_runtime,
            RetailRuntimeValue::Known(Some(SubAPropulsionRuntime::pending_constructor_rng(
                descriptor
            )))
        );
    }
}
#[v2k_test_support::retail_test]
fn class18_water_burst_uses_selected_extent_owner_and_displaced_sound_before_children() {
    let (session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(19);
    let id = parent(&manager);
    let sea = session.cache.level_terrain().unwrap().sea_level_raw();
    let position = [10240, sea.wrapping_sub(1), 10240];
    manager.entity_mut(id).unwrap().set_position_raw(position);
    for _ in 0..8 {
        fx.advance_frame_pacing(20_000);
    }
    fx.take_positional_sounds();
    let before = fx.particle_count();
    let mut tasks = SpecializedActorTaskScheduler::default();
    death::begin_type40_standard_death(
        &mut manager,
        id,
        death::Type40Class18Frame {
            resources: &session.cache,
            world_fx: &mut fx,
            retail_tick: 4794,
            tasks: &mut tasks,
        },
    )
    .unwrap();
    let particles = fx.test_particles_in_virgin_birth_order();
    let burst = &particles[before..];
    assert_eq!(
        burst.iter().map(|p| p.source_class).collect::<Vec<_>>(),
        vec![37, 37, 45, 46]
    );
    assert!(burst.iter().all(|p| p.owner_id == Some(id)
        && p.source_entity_type_at_birth == Some(40)
        && p.age_ticks == 0.0));
    let sounds = fx.take_positional_sounds();
    assert_eq!(sounds.len(), 1);
    assert_eq!(sounds[0].sound_id, 62);
    let expected = [position[0].wrapping_sub(64), position[1], position[2]];
    assert_eq!(sounds[0].position, expected.map(|w| f32::from(w) / 256.0));
    assert!(death::finished_terminal_authenticates(&manager, id));
}

#[v2k_test_support::retail_test]
fn actual_missing_child_model_blocks_after_launch_prefix_and_never_replays_or_fabricates_disposal()
{
    let (mut session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(19);
    let id = parent(&manager);
    assert_eq!(session.cache.global_model_system_level(1040), Some(9));
    assert_eq!(session.cache.global_model_system_level(1041), Some(9));
    // A test-only resident resource failure. Preserve the canonical1040 parent
    // and section12 data; remove the next model from the new in-memory layer.
    // No OVL bytes or generated assets are written or shifted.
    let mut missing =
        crate::loader::load_level(&session.data_dir.join("Overlay/1X9XX.OVL")).unwrap();
    missing.system_level = Some(9);
    missing
        .models
        .as_mut()
        .unwrap()
        .all_entries
        .truncate(1041 - 973);
    session.cache.add_auxiliary(missing);
    assert!(session.cache.global_model(1040).is_some());
    assert!(session.cache.global_model(1041).is_none());
    let before = manager.iter_all().count();
    let before_stamp = manager.next_common_body_ordinal();
    let mut expected = fx.fork_for_main_base_abort_transaction();
    for _ in 0..10 {
        expected.next_shared_retail_random_u16();
    }
    let mut tasks = SpecializedActorTaskScheduler::default();
    let result = death::begin_type40_standard_death(
        &mut manager,
        id,
        death::Type40Class18Frame {
            resources: &session.cache,
            world_fx: &mut fx,
            retail_tick: 4794,
            tasks: &mut tasks,
        },
    );
    assert!(matches!(
        result,
        Err(death::Type40DeathBlock::Split(
            crate::split_and_explode::SplitAndExplodeBlock::Host {
                phase: crate::split_and_explode::SplitAndExplodePhase::ChildConstruction,
                reason: death::Type40SplitHostBlock::Child(
                    crate::native_type56::Type56Block::Metadata
                ),
            }
        ))
    ));
    assert_eq!(manager.iter_all().count(), before);
    assert_eq!(
        manager.next_common_body_ordinal(),
        before_stamp,
        "missingmodel fails before the child body allocation"
    );
    let split = manager
        .entity_mut(id)
        .unwrap()
        .native_type40_runtime
        .unwrap()
        .split_terminal
        .unwrap();
    assert!(split.progress.tasks_cleared);
    assert_eq!(split.progress.launch_rng_words, 9);
    assert_eq!(split.progress.attempted_children, 1);
    assert_eq!(split.progress.created_children, 0);
    assert!(
        !split.progress.constructor_error_disposed,
        "an unowned host failure is not a native error-object receipt"
    );
    assert!(!split.progress.source_deferred_destroyed);
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
    let mut expected = fx.fork_for_main_base_abort_transaction();
    assert!(matches!(
        death::begin_type40_standard_death(
            &mut manager,
            id,
            death::Type40Class18Frame {
                resources: &session.cache,
                world_fx: &mut fx,
                retail_tick: 4795,
                tasks: &mut tasks,
            }
        ),
        Err(death::Type40DeathBlock::Graph)
    ));
    assert_eq!(manager.iter_all().count(), before);
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
}
