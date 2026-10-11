//! Actual Type122/8/9 allocations exercise the shared transport callbacks.
//! Ordinary unresolved people remain explicit prerequisites, not fake None tasks.

use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    entity_behavior::{behavior_program, BehaviorDescriptorIdentity},
    main_base_type9_abort::MainBaseType9ResultScreenState,
    native_ground_actor::behavior::{reselect, ReselectionEntry},
    native_type122::{construction_tests::native_fixture, profile::Type122Profile, Type122Owner},
    session::GameSession,
    shared_target_route::SharedTargetRouteLifetime,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
};

pub(crate) struct Fixture {
    pub(crate) _session: GameSession,
    pub(crate) manager: EntityManager,
    pub(crate) tasks: SpecializedActorTaskScheduler,
    pub(crate) fx: WorldFx,
    pub(crate) notifications: GameplayNotifications,
    pub(crate) parent: u32,
    pub(crate) child: u32,
}

impl Fixture {
    pub(crate) fn new(child_type: u32) -> Self {
        let (session, mut manager, mut fx) = native_fixture(50);
        let parent = manager
            .iter_all()
            .find(|entity| entity.entity_type == 122)
            .unwrap()
            .id;
        let child = manager
            .iter_all()
            .find(|entity| entity.entity_type == child_type)
            .unwrap()
            .id;
        let mut tasks = SpecializedActorTaskScheduler::default();
        tasks.adopt_intro2_type9(&mut manager);
        tasks.adopt_intro2_type8(&mut manager);
        tasks.adopt_type122(&manager);
        let position = actor(&manager, parent).unwrap().position_raw();
        manager.entity_mut(child).unwrap().set_position_raw([
            position[0].wrapping_add(32),
            position[1],
            position[2].wrapping_add(32),
        ]);
        for _ in 0..64 {
            reselect::<Type122Profile>(
                &mut manager,
                parent,
                4793,
                &mut fx,
                None,
                ReselectionEntry::Impact,
            )
            .unwrap();
            if matches!(actor(&manager,parent).unwrap().current_behavior_context,
                RetailRuntimeValue::Known(Some(context))
                if context.descriptor() == BehaviorDescriptorIdentity::Named(behavior_program(9).unwrap()))
            {
                break;
            }
        }
        assert!(
            matches!(actor(&manager,parent).unwrap().current_behavior_context,
            RetailRuntimeValue::Known(Some(context))
            if context.descriptor() == BehaviorDescriptorIdentity::Named(behavior_program(9).unwrap()))
        );
        tasks.register_type122(Type122Owner::adopt(&manager, parent).unwrap());
        Self {
            _session: session,
            manager,
            tasks,
            fx,
            notifications: GameplayNotifications::new(),
            parent,
            child,
        }
    }

    pub(crate) fn attach(&mut self) -> Result<(), CaptureBlock> {
        attach_capture_child(
            &mut self.manager,
            self.parent,
            self.child,
            &mut CaptureContext {
                resources: None,
                tasks: &mut self.tasks,
                world_fx: &mut self.fx,
                notifications: &mut self.notifications,
                retail_tick: 4793,
                result_screen: MainBaseType9ResultScreenState::NotShown,
                hive_dying: Default::default(),
            },
        )
    }

    pub(crate) fn carry(&mut self, variant: u8) {
        carry_tasks::publish_style(
            &mut self.manager,
            self.parent,
            variant,
            carry_tasks::CaptureTargetWrite::Set(Some(self.child)),
            &mut self.fx,
        )
        .unwrap();
        self.tasks
            .register_type122(Type122Owner::adopt(&self.manager, self.parent).unwrap());
    }

    fn root(
        &mut self,
        callback: CaptureRootCallback,
    ) -> Result<CaptureCallbackCompletion, CaptureBlock> {
        execute_capture_root(
            &mut self.manager,
            self.parent,
            callback,
            &mut CaptureContext {
                resources: None,
                tasks: &mut self.tasks,
                world_fx: &mut self.fx,
                notifications: &mut self.notifications,
                retail_tick: 4794,
                result_screen: MainBaseType9ResultScreenState::NotShown,
                hive_dying: Default::default(),
            },
        )
    }
}

#[v2k_test_support::retail_test]
fn type122_carries_and_releases_actual_worker_and_peasant_with_own_j_offset() {
    for child_type in [8, 9] {
        let mut f = Fixture::new(child_type);
        let initial = actor(&f.manager, f.child).unwrap().current_behavior_context;
        f.attach().unwrap();
        f.carry(2);
        let parent = actor(&f.manager, f.parent).unwrap();
        let RetailRuntimeValue::Known(basis) = parent.physical_body_basis_q31() else {
            panic!()
        };
        let mut expected = parent.position_raw();
        for axis in 0..3 {
            expected[axis] =
                expected[axis].wrapping_add(crate::hover::q31_mul(basis.forward[axis], 120) as i16);
        }
        assert_eq!(
            parent.native_capture_relation.unwrap().profile,
            NativeCaptorProfile::Type122
        );
        update_carried_pose(&mut f.manager, f.parent).unwrap();
        let child = actor(&f.manager, f.child).unwrap();
        assert_eq!(child.attached_to, Some(f.parent));
        assert_eq!(
            child.position_raw(),
            expected,
            "actual Type122 local[0,0,120] uses the current Q31 basis"
        );
        assert_eq!(child.velocity_raw(), [0; 3]);
        assert!(matches!(
            child.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::None)
        ));
        assert_ne!(child.current_behavior_context, initial);
        assert!(f
            .root(CaptureRootCallback::Cleanup)
            .unwrap()
            .common_dying_owner
            .is_none());
        assert_eq!(actor(&f.manager, f.child).unwrap().attached_to, None);
        assert!(actor(&f.manager, f.parent)
            .unwrap()
            .native_capture_relation
            .is_none());
        assert_eq!(
            actor(&f.manager, f.parent)
                .unwrap()
                .actor_common_axis_descriptor,
            RetailRuntimeValue::Known(crate::native_type122::AXIS)
        );
        assert!(Type122Owner::adopt(&f.manager, f.parent).is_ok());
    }
}

#[v2k_test_support::retail_test]
fn native_type122_c910_pair_keeps_actual_counterpart_and_new_a900_task_walk() {
    use crate::player_active_contact::{
        active_pair_body_from_entity, classify_oriented_active_pair_contact,
        OrientedActivePairContactRequest,
    };
    for child_type in [8, 9] {
        let mut f = Fixture::new(child_type);
        let center = [
            32720,
            f._session
                .cache
                .terrain()
                .unwrap()
                .sea_level_raw()
                .checked_add(6000)
                .unwrap(),
            16000,
        ];
        f.manager
            .entity_mut(f.parent)
            .unwrap()
            .set_position_raw(center);
        f.manager.entity_mut(f.child).unwrap().set_position_raw([
            center[0].wrapping_add(700),
            center[1],
            center[2].wrapping_add(500),
        ]);
        for id in [f.parent, f.child] {
            let entity = f.manager.entity_mut(id).unwrap();
            entity
                .collision
                .state_flags_at_0x08
                .overwrite(0x8000, 0x8000);
            entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
        }
        for _ in 0..128 {
            crate::native_ground_actor::behavior::secondary::<Type122Profile>(
                &mut f.manager,
                f.parent,
                20000,
                4793,
                &mut f.fx,
            )
            .unwrap();
            if matches!(actor(&f.manager,f.parent).unwrap().current_behavior_context,
                RetailRuntimeValue::Known(Some(context)) if context.active_style().style_address()==0x4c8038)
            {
                break;
            }
        }
        assert!(
            matches!(actor(&f.manager,f.parent).unwrap().current_behavior_context,
            RetailRuntimeValue::Known(Some(context)) if context.active_style().style_address()==0x4c8038
                && context.target_handle_at_0x08()==RetailRuntimeValue::Known(Some(f.child)))
        );
        f.tasks
            .register_type122(Type122Owner::adopt(&f.manager, f.parent).unwrap());
        let order = f.manager.retail_live_order_ids().collect::<Vec<_>>();
        let (subject, candidate) = if order.iter().position(|id| *id == f.parent)
            < order.iter().position(|id| *id == f.child)
        {
            (f.parent, f.child)
        } else {
            (f.child, f.parent)
        };
        let mut found = false;
        'contact: for dy in [-150_i16, -75, 0, 75, 150] {
            for dx in [80_i16, 160, 240, -80, -160, -240, 0] {
                for dz in [80_i16, 160, -80, -160, 0] {
                    f.manager.entity_mut(f.child).unwrap().set_position_raw([
                        center[0].wrapping_add(dx),
                        center[1].wrapping_add(dy),
                        center[2].wrapping_add(dz),
                    ]);
                    if classify_oriented_active_pair_contact(
                        OrientedActivePairContactRequest {
                            subject: actor(&f.manager, subject).unwrap(),
                            candidate: actor(&f.manager, candidate).unwrap(),
                            subject_entry: &active_pair_body_from_entity(
                                actor(&f.manager, subject).unwrap(),
                                &f._session.cache,
                            ),
                            retail_tick: 4793,
                        },
                        &f._session.cache,
                    )
                    .unwrap()
                    .is_some()
                    {
                        found = true;
                        break 'contact;
                    }
                }
            }
        }
        assert!(
            found,
            "canonical model274/person contact must pass the actual oriented interpreter"
        );
        let result = pair::resolve_native_captor_active_contacts(
            &mut crate::intro2_contacts::Intro2ContactFrame {
                entities: &mut f.manager,
                resources: &mut f._session.cache,
                world_fx: &mut f.fx,
                static_damage: &mut crate::static_damage::StaticDamageScheduler::new(),
                notifications: &mut f.notifications,
                retail_tick: 4793,
                actor_tasks: &mut f.tasks,
            },
            subject,
            pair::CaptureFeedbackPolicy::Cinematic,
        );
        let pair::NativeCaptorPairOutcome::Resolved { visits } = result else {
            panic!("{result:?}")
        };
        let visit = visits
            .iter()
            .find(|visit| visit.candidate_id == candidate)
            .unwrap();
        assert!(visit.physical_suppressed);
        assert!(visit
            .stages
            .iter()
            .any(|stage| matches!(stage, pair::NativeCaptorPairStage::DispatchCaptureAccepted)));
        assert_eq!(
            visit
                .stages
                .iter()
                .filter(|stage| matches!(stage, pair::NativeCaptorPairStage::Component { .. }))
                .count(),
            6
        );
        assert_eq!(
            actor(&f.manager, f.child).unwrap().attached_to,
            Some(f.parent)
        );
        assert!(matches!(
            carry_tasks::carrying_variant(actor(&f.manager, f.parent).unwrap()),
            Some(2 | 3)
        ));
    }
}

#[v2k_test_support::retail_test]
fn type122_carry_variants_own_slots_lifetimes_speed_and_constructor_rng() {
    for (variant, draws) in [(2, 1), (3, 2), (4, 1), (5, 1)] {
        let mut f = Fixture::new(8);
        f.attach().unwrap();
        let mut expected = f.fx.fork_for_main_base_abort_transaction();
        for _ in 0..draws {
            expected.next_shared_retail_random_u16();
        }
        f.carry(variant);
        assert_eq!(
            f.fx.next_shared_retail_random_u16(),
            expected.next_shared_retail_random_u16()
        );
        let parent = actor(&f.manager, f.parent).unwrap();
        assert_eq!(
            parent.collision.default_state_flags_at_0xc8,
            RetailRuntimeValue::Known(0x439)
        );
        assert_eq!(carry_tasks::carrying_variant(parent), Some(variant));
        assert!(parent.actor_task_state(ActorTaskSlot::Tertiary).is_none());
        match variant {
            2 => assert!(
                matches!(parent.actor_task_state(ActorTaskSlot::Primary), Some(ActorTaskRuntime::CapturePeoplePursuit(task))
                if task.lifetime() == SharedTargetRouteLifetime::Unlimited && task.target_id() == Some(f.child))
            ),
            4 => assert!(
                matches!(parent.actor_task_state(ActorTaskSlot::Primary), Some(ActorTaskRuntime::CapturePeopleFollowing(task))
                if task.target_id() == f.child && task.elapsed_ms() == 0)
            ),
            3 | 5 => assert!(
                matches!(parent.actor_task_state(ActorTaskSlot::Primary), Some(ActorTaskRuntime::SharedRetarget(task))
                if task.lifetime_ms() == if variant == 3 {500} else {5000})
            ),
            _ => unreachable!(),
        }
        if variant == 3 {
            assert!(matches!(
                parent.actor_task_state(ActorTaskSlot::Secondary),
                Some(ActorTaskRuntime::CaptureBeaconAcquisition)
            ));
        } else {
            assert!(parent.actor_task_state(ActorTaskSlot::Secondary).is_none());
        }
        if matches!(variant, 2 | 4) {
            let RetailRuntimeValue::Known(Some(a)) = parent.sub_a_propulsion_runtime else {
                panic!()
            };
            assert_eq!(a.target_speed_raw(), RetailRuntimeValue::Known(613));
        }
    }
}

#[v2k_test_support::retail_test]
fn ordinary_children_without_native_tasks_block_before_capture_row_or_rng_mutation() {
    let mut count = 0;
    let mut native_constructor_blocks = 0;
    let mut task_custody_blocks = 0;
    for level in [24, 42, 46, 49] {
        let (_session, mut manager, mut fx) = native_fixture(level);
        let parent = manager
            .iter_all()
            .find(|entity| entity.entity_type == 122)
            .unwrap()
            .id;
        let children = manager
            .iter_all()
            .filter(|entity| matches!(entity.entity_type, 86 | 90 | 116 | 123))
            .map(|entity| (entity.id, entity.entity_type))
            .collect::<Vec<_>>();
        let mut tasks = SpecializedActorTaskScheduler::default();
        tasks.adopt_type122(&manager);
        let mut notifications = GameplayNotifications::new();
        let mut expected_fx = fx.fork_for_main_base_abort_transaction();
        for (child, entity_type) in children {
            let before = actor(&manager, child)
                .unwrap()
                .collision
                .state_flags_at_0x08;
            assert!(!tasks.capture_child_mutation_ready(&manager, child));
            let error = attach_capture_child(
                &mut manager,
                parent,
                child,
                &mut CaptureContext {
                    resources: None,
                    tasks: &mut tasks,
                    world_fx: &mut fx,
                    notifications: &mut notifications,
                    retail_tick: 4793,
                    result_screen: MainBaseType9ResultScreenState::NotShown,
                    hive_dying: Default::default(),
                },
            )
            .unwrap_err();
            // All four families now own native constructions; without
            // scheduler adoption they block at task custody before any row
            // or RNG mutation.
            let expected = if matches!(entity_type, 86 | 90 | 116 | 123) {
                task_custody_blocks += 1;
                CaptureBlock::new("capture child task custody")
            } else {
                native_constructor_blocks += 1;
                CaptureBlock::new("capture child native constructor")
            };
            assert_eq!(error, expected);
            assert_eq!(
                actor(&manager, child)
                    .unwrap()
                    .collision
                    .state_flags_at_0x08,
                before
            );
            assert_eq!(actor(&manager, child).unwrap().attached_to, None);
            assert!(actor(&manager, parent)
                .unwrap()
                .native_capture_relation
                .is_none());
            let RetailRuntimeValue::Known(Some(rows)) =
                &actor(&manager, parent).unwrap().sub_j_attachment_runtime
            else {
                panic!()
            };
            assert!(rows.is_empty());
            count += 1;
        }
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            expected_fx.next_shared_retail_random_u16()
        );
    }
    assert_eq!(count, 27);
    assert_eq!(native_constructor_blocks, 0);
    assert_eq!(task_custody_blocks, 27);
}

#[v2k_test_support::retail_test]
fn changed_type122_body_receipt_cannot_borrow_type17_capture_identity() {
    let mut f = Fixture::new(8);
    f.manager.entity_mut(f.parent).unwrap().model_slots = [Some(256); 4];
    assert_eq!(
        f.attach().unwrap_err(),
        CaptureBlock::new("capture native allocation")
    );
    assert_eq!(actor(&f.manager, f.child).unwrap().attached_to, None);
    assert!(actor(&f.manager, f.parent)
        .unwrap()
        .native_capture_relation
        .is_none());
}

#[v2k_test_support::retail_test]
fn carrying_static_contact_rejects_stale_relation_but_accepts_18640_empty_row() {
    use crate::native_type122::contact::{
        resolve_type122_static_contact, Type122ContactBlock, Type122ContactOutcome,
    };
    for compacted in [false, true] {
        let mut f = Fixture::new(8);
        f.attach().unwrap();
        f.carry(2);
        if compacted {
            f.manager
                .entity_mut(f.child)
                .unwrap()
                .collision
                .state_flags_at_0x08
                .overwrite(DYING_STATE_BIT, DYING_STATE_BIT);
            update_carried_pose(&mut f.manager, f.parent).unwrap();
            assert!(
                !actor(&f.manager, f.parent)
                    .unwrap()
                    .native_capture_relation
                    .unwrap()
                    .row_present
            );
            validate_capture_relation(&f.manager, f.parent).unwrap();
        } else {
            f.manager.entity_mut(f.child).unwrap().attached_to = None;
            assert!(validate_capture_relation(&f.manager, f.parent).is_err());
        }
        let mut static_fixture = crate::native_type122::contact_tests::Fixture {
            session: f._session,
            entities: f.manager,
            fx: f.fx,
            tasks: f.tasks,
            damage: crate::static_damage::StaticDamageScheduler::new(),
            notifications: f.notifications,
            id: f.parent,
        };
        static_fixture.enable_contact();
        static_fixture.overlap();
        let position = static_fixture.entity().position_raw();
        let primary = static_fixture
            .entity()
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary);
        let mut expected_fx = static_fixture.fx.fork_for_main_base_abort_transaction();
        let result = resolve_type122_static_contact(&mut static_fixture.frame(), f.parent);
        if compacted {
            assert!(
                matches!(result, Type122ContactOutcome::Applied(_)),
                "{result:?}"
            );
        } else {
            assert!(
                matches!(
                    result,
                    Type122ContactOutcome::Blocked {
                        reason: Type122ContactBlock::Runtime("Capture transport relation"),
                        committed_prefix: false
                    }
                ),
                "{result:?}"
            );
            assert_eq!(static_fixture.entity().position_raw(), position);
            assert_eq!(
                static_fixture
                    .entity()
                    .actor_tasks
                    .task_in_slot(ActorTaskSlot::Primary),
                primary
            );
            assert_eq!(
                static_fixture.fx.next_shared_retail_random_u16(),
                expected_fx.next_shared_retail_random_u16()
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn type122_destination_delivery_runs_actual_child_release_and_retirement() {
    for child_type in [8, 9] {
        let mut f = Fixture::new(child_type);
        f.attach().unwrap();
        f.carry(2);
        let health = actor(&f.manager, f.child).unwrap().collision.health_raw;
        let destination = f
            .manager
            .iter_all()
            .find(|entity| entity.entity_type == 67)
            .unwrap()
            .id;
        execute_capture_delivery(
            &mut f.manager,
            f.parent,
            destination,
            &mut CaptureContext {
                resources: None,
                tasks: &mut f.tasks,
                world_fx: &mut f.fx,
                notifications: &mut f.notifications,
                retail_tick: 4794,
                result_screen: MainBaseType9ResultScreenState::NotShown,
                hive_dying: Default::default(),
            },
        )
        .unwrap();
        assert_eq!(actor(&f.manager, f.child).unwrap().attached_to, None);
        assert!(f
            .manager
            .pending_actor_deferred_destroy_ids()
            .contains(&f.child));
        assert_eq!(
            actor(&f.manager, f.child).unwrap().collision.health_raw,
            health
        );
        assert!(actor(&f.manager, f.parent)
            .unwrap()
            .native_capture_relation
            .is_none());
        assert!(Type122Owner::adopt(&f.manager, f.parent).is_ok());
    }
}

#[v2k_test_support::retail_test]
fn occupied_type122_direct_death_releases_then_runs_two_c620_constructors() {
    let mut f = Fixture::new(8);
    f.attach().unwrap();
    f.carry(2);
    let mut expected = f.manager.fork_for_main_base_abort_transaction();
    let mut expected_fx = f.fx.fork_for_main_base_abort_transaction();
    let mut expected_tasks = f.tasks.fork_for_main_base_abort_transaction();
    let mut expected_notifications = GameplayNotifications::new();
    expected
        .entity_mut(f.parent)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(DYING_STATE_BIT, DYING_STATE_BIT);
    expected.entity_mut(f.parent).unwrap().collision.health_raw = RetailRuntimeValue::Known(0);
    let first = execute_capture_root(
        &mut expected,
        f.parent,
        CaptureRootCallback::Cleanup,
        &mut CaptureContext {
            resources: None,
            tasks: &mut expected_tasks,
            world_fx: &mut expected_fx,
            notifications: &mut expected_notifications,
            retail_tick: 4794,
            result_screen: MainBaseType9ResultScreenState::NotShown,
            hive_dying: Default::default(),
        },
    )
    .unwrap();
    assert!(first.common_dying_owner.is_some());
    let first_task = actor(&expected, f.parent)
        .unwrap()
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary);
    crate::intro2_common_dying::publish_intro2_common_dying_alternate(
        &mut expected,
        f.parent,
        &mut expected_fx,
    )
    .unwrap()
    .unwrap();
    let second_task = actor(&expected, f.parent)
        .unwrap()
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary);
    assert_ne!(first_task, second_task);
    publish_native_captor_standard_death(
        &mut f.manager,
        f.parent,
        &mut CaptureContext {
            resources: None,
            tasks: &mut f.tasks,
            world_fx: &mut f.fx,
            notifications: &mut f.notifications,
            retail_tick: 4794,
            result_screen: MainBaseType9ResultScreenState::NotShown,
            hive_dying: Default::default(),
        },
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        actor(&f.manager, f.parent)
            .unwrap()
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary),
        second_task
    );
    assert_eq!(
        f.fx.next_shared_retail_random_u16(),
        expected_fx.next_shared_retail_random_u16()
    );
    assert_eq!(
        actor(&f.manager, f.parent)
            .unwrap()
            .collision
            .default_state_flags_at_0xc8,
        RetailRuntimeValue::Known(0x439)
    );
    assert_eq!(actor(&f.manager, f.child).unwrap().attached_to, None);
    assert!(actor(&f.manager, f.parent)
        .unwrap()
        .native_capture_relation
        .is_none());
}

#[v2k_test_support::retail_test]
fn ordinary_type116_capture_attach_and_release_uses_native_worker_receipt() {
    let (_session, mut manager, mut fx) = native_fixture(42);
    let parent = manager
        .iter_all()
        .find(|entity| entity.entity_type == 122)
        .unwrap()
        .id;
    let child = manager
        .iter_all()
        .find(|entity| entity.entity_type == 116)
        .unwrap()
        .id;
    assert!(crate::intro2_type8::intro2_type8_manager_allocation_authenticates(&manager, child));
    let mut tasks = SpecializedActorTaskScheduler::default();
    tasks.adopt_type122(&manager);
    assert_eq!(tasks.adopt_intro2_type8(&mut manager), 3);
    assert!(tasks.capture_child_mutation_ready(&manager, child));
    let mut notifications = GameplayNotifications::new();
    attach_capture_child(
        &mut manager,
        parent,
        child,
        &mut CaptureContext {
            resources: None,
            tasks: &mut tasks,
            world_fx: &mut fx,
            notifications: &mut notifications,
            retail_tick: 4793,
            result_screen: MainBaseType9ResultScreenState::NotShown,
            hive_dying: Default::default(),
        },
    )
    .unwrap();
    assert_eq!(actor(&manager, child).unwrap().attached_to, Some(parent));
    assert!(actor(&manager, parent)
        .unwrap()
        .native_capture_relation
        .is_some());
    // Carried Type116 retains its worker model and attention cue through the
    // shared C910 attachment; it does not borrow a Type8 receipt.
    let carried = actor(&manager, child).unwrap();
    assert_eq!(carried.model_slots, [Some(1136); 4]);
    let RetailRuntimeValue::Known(Some(animation)) = carried.actor_animation_runtime else {
        panic!();
    };
    assert_eq!(animation.descriptor().attention_stop_sound_id, 72);
}

#[v2k_test_support::retail_test]
fn ordinary_type90_capture_attach_uses_native_desert_worker_receipt() {
    let (_session, mut manager, mut fx) = native_fixture(24);
    let parent = manager
        .iter_all()
        .find(|entity| entity.entity_type == 122)
        .unwrap()
        .id;
    let child = manager
        .iter_all()
        .find(|entity| entity.entity_type == 90)
        .unwrap()
        .id;
    assert!(crate::intro2_type8::intro2_type8_manager_allocation_authenticates(&manager, child));
    let mut tasks = SpecializedActorTaskScheduler::default();
    tasks.adopt_type122(&manager);
    assert!(tasks.adopt_intro2_type8(&mut manager) >= 1);
    assert!(tasks.capture_child_mutation_ready(&manager, child));
    let mut notifications = GameplayNotifications::new();
    attach_capture_child(
        &mut manager,
        parent,
        child,
        &mut CaptureContext {
            resources: None,
            tasks: &mut tasks,
            world_fx: &mut fx,
            notifications: &mut notifications,
            retail_tick: 4793,
            result_screen: MainBaseType9ResultScreenState::NotShown,
            hive_dying: Default::default(),
        },
    )
    .unwrap();
    assert_eq!(actor(&manager, child).unwrap().attached_to, Some(parent));
    assert!(actor(&manager, parent)
        .unwrap()
        .native_capture_relation
        .is_some());
    let carried = actor(&manager, child).unwrap();
    assert_eq!(carried.model_slots, [Some(890); 4]);
    let RetailRuntimeValue::Known(Some(animation)) = carried.actor_animation_runtime else {
        panic!();
    };
    assert_eq!(animation.descriptor().capability_bit_3_sound_id, 85);
    assert_eq!(animation.descriptor().attention_stop_sound_id, 0);
}

#[v2k_test_support::retail_test]
fn ordinary_type123_capture_attach_uses_native_person_receipt() {
    let (_session, mut manager, mut fx) = native_fixture(49);
    let parent = manager
        .iter_all()
        .find(|entity| entity.entity_type == 122)
        .unwrap()
        .id;
    let child = manager
        .iter_all()
        .find(|entity| entity.entity_type == 123)
        .unwrap()
        .id;
    assert!(
        crate::native_type123::native_type123_manager_allocation_authenticates(&manager, child)
    );
    let mut tasks = SpecializedActorTaskScheduler::default();
    tasks.adopt_type122(&manager);
    assert!(tasks.adopt_native_type123(&mut manager) >= 1);
    assert!(tasks.capture_child_mutation_ready(&manager, child));
    let mut notifications = GameplayNotifications::new();
    attach_capture_child(
        &mut manager,
        parent,
        child,
        &mut CaptureContext {
            resources: None,
            tasks: &mut tasks,
            world_fx: &mut fx,
            notifications: &mut notifications,
            retail_tick: 4793,
            result_screen: MainBaseType9ResultScreenState::NotShown,
            hive_dying: Default::default(),
        },
    )
    .unwrap();
    assert_eq!(actor(&manager, child).unwrap().attached_to, Some(parent));
    assert!(actor(&manager, parent)
        .unwrap()
        .native_capture_relation
        .is_some());
    let carried = actor(&manager, child).unwrap();
    assert_eq!(carried.model_slots, [Some(889); 4]);
    let RetailRuntimeValue::Known(Some(animation)) = carried.actor_animation_runtime else {
        panic!();
    };
    assert_eq!(animation.descriptor().attention_stop_sound_id, 56);
}

#[v2k_test_support::retail_test]
fn ordinary_type86_capture_attach_uses_native_person_receipt() {
    let (_session, mut manager, mut fx) = native_fixture(24);
    let parent = manager
        .iter_all()
        .find(|entity| entity.entity_type == 122)
        .unwrap()
        .id;
    let child = manager
        .iter_all()
        .find(|entity| entity.entity_type == 86)
        .unwrap()
        .id;
    assert!(crate::native_type86::native_type86_manager_allocation_authenticates(&manager, child));
    let mut tasks = SpecializedActorTaskScheduler::default();
    tasks.adopt_type122(&manager);
    assert!(tasks.adopt_native_type86(&mut manager) >= 1);
    assert!(tasks.capture_child_mutation_ready(&manager, child));
    let mut notifications = GameplayNotifications::new();
    attach_capture_child(
        &mut manager,
        parent,
        child,
        &mut CaptureContext {
            resources: None,
            tasks: &mut tasks,
            world_fx: &mut fx,
            notifications: &mut notifications,
            retail_tick: 4793,
            result_screen: MainBaseType9ResultScreenState::NotShown,
            hive_dying: Default::default(),
        },
    )
    .unwrap();
    assert_eq!(actor(&manager, child).unwrap().attached_to, Some(parent));
    assert!(actor(&manager, parent)
        .unwrap()
        .native_capture_relation
        .is_some());
    let carried = actor(&manager, child).unwrap();
    assert_eq!(carried.model_slots, [Some(889); 4]);
    let RetailRuntimeValue::Known(Some(animation)) = carried.actor_animation_runtime else {
        panic!();
    };
    assert_eq!(animation.descriptor().capability_bit_3_sound_id, 85);
    assert_eq!(animation.descriptor().attention_stop_sound_id, 72);
}
