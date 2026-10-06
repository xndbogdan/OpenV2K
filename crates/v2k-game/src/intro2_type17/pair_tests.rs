use super::*;
use crate::{
    entity::{AuthoredWorldConstruction, EntityConstructionResources},
    entity_behavior::BehaviorDescriptorIdentity,
    entity_collision_state::EntityTypeRuntimeMetadata,
    gameplay_notifications::GameplayNotifications,
    session::GameSession,
    specialized_actor_task_production::{
        SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
        SpecializedActorTaskScheduler,
    },
    static_damage::StaticDamageScheduler,
    world_fx::WorldFx,
};

/// Actual native allocations, with controlled C690/acquisition and contact
/// positions. This is a phase regression, not a retail timing observation.
pub(crate) struct Fixture {
    pub session: GameSession,
    pub entities: EntityManager,
    pub scheduler: SpecializedActorTaskScheduler,
    pub fx: WorldFx,
    pub notifications: GameplayNotifications,
    pub static_damage: StaticDamageScheduler,
    pub spider: u32,
    pub child: u32,
    pub subject: u32,
    pub candidate: u32,
    pub tick: u32,
}

#[derive(Clone, Copy)]
pub(crate) enum ChildOrder {
    AuthoredSpawn(usize),
    First,
    BeforeSpider,
    AfterSpider,
}

impl Fixture {
    pub(crate) fn ready(world: u32, spawn: Option<usize>, child_order: ChildOrder) -> Self {
        let data = v2k_test_support::retail_dir();
        assert!(
            data.join("PRELOAD.DAT").is_file(),
            "normal-tier retail data required"
        );
        let mut session = GameSession::init(&data).unwrap();
        session.load_auxiliary_ovl(3, 1).unwrap();
        session.load_level_by_id(world, 1).unwrap();
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
        let RetailRuntimeValue::Known(Some(slots)) = &metadata[17].sub_j_attachment_descriptor
        else {
            panic!("native Type17 has its authored Sub-J descriptor")
        };
        assert_eq!(slots.slots.len(), 1);
        assert_eq!(slots.slots[0].policy_word_raw, 1);
        let mut fx = WorldFx::new();
        let model_extent = |id| session.cache.global_model(id).map(|model| model.radius);
        let resources = EntityConstructionResources {
            terrain: session.cache.terrain(),
            terrain_objects: session.cache.terrain_objects(),
            model_extent_raw: Some(&model_extent),
        };
        let mut entities = if world == 50 {
            EntityManager::from_native_intro2_frontend(
                session.cache.level_desc().unwrap(),
                &metadata,
                resources,
                321,
                &mut fx,
            )
            .unwrap()
        } else {
            EntityManager::from_authored_world(
                AuthoredWorldConstruction {
                    logical_world_index: (world - 12) as i32,
                    level: session.cache.level_desc().unwrap(),
                    type_metadata: &metadata,
                    resources,
                    player_arrival: None,
                    retail_tick: 321,
                },
                &mut fx,
            )
            .unwrap()
        };
        let spider = entities
            .iter_all()
            .find(|entity| {
                entity.entity_type == 17
                    && spawn.is_none_or(|spawn| entity.authored_spawn_index == Some(spawn))
            })
            .unwrap()
            .id;
        let order = entities.retail_live_order_ids().collect::<Vec<_>>();
        let spider_cursor = order.iter().position(|id| *id == spider).unwrap();
        let child = order
            .iter()
            .enumerate()
            .find_map(|(cursor, &id)| {
                let correct_side = match child_order {
                    ChildOrder::AuthoredSpawn(spawn) => {
                        actor(&entities, id).unwrap().authored_spawn_index == Some(spawn)
                    }
                    ChildOrder::First => true,
                    ChildOrder::BeforeSpider => cursor < spider_cursor,
                    ChildOrder::AfterSpider => cursor > spider_cursor,
                };
                (correct_side && actor(&entities, id).unwrap().entity_type == 9).then_some(id)
            })
            .expect("actual authored Type9 on the requested side of the spider");
        // A wrapped X contact, far above the authored world, isolates these two
        // actual bodies without deleting participants or forging contact data.
        // Retail raw Y is positive upward: E370 regards Y < sea-radius/4
        // as deep water. Keep this dry contact control above the actual sea.
        let center = [
            32_720,
            session
                .cache
                .terrain()
                .unwrap()
                .sea_level_raw()
                .checked_add(6_000)
                .expect("controlled dry altitude fits the signed position word"),
            16_000,
        ];
        entities
            .entity_mut(spider)
            .unwrap()
            .set_position_raw(center);
        entities.entity_mut(child).unwrap().set_position_raw([
            center[0].wrapping_add(700),
            center[1],
            center[2].wrapping_add(500),
        ]);
        for id in [spider, child] {
            let entity = entities.entity_mut(id).unwrap();
            // Explicit admission for the controlled contact phase, including
            // Intro2 actors whose scenario start command has not run here.
            entity
                .collision
                .state_flags_at_0x08
                .overwrite(0x8000, 0x8000);
            entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
        }
        entities
            .entity_mut(spider)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .overwrite(0x0400_0000, 0x0400_0000);
        // Use AC60's genuine weighted table and the shared process RNG. The
        // only nearby person is the actual native child placed above.
        for _ in 0..64 {
            super::super::behavior::reselect(
                &mut entities,
                spider,
                321,
                &mut fx,
                super::super::behavior::ReselectionEntry::TaskResult,
            )
            .unwrap();
            let RetailRuntimeValue::Known(Some(context)) = entities
                .entity_mut(spider)
                .unwrap()
                .current_behavior_context
            else {
                panic!()
            };
            if matches!(context.descriptor(), BehaviorDescriptorIdentity::Named(program) if program.class_id == 9)
            {
                break;
            }
        }
        assert_eq!(
            style_address(entities.entity_mut(spider).unwrap()).unwrap(),
            0x4c7ff0
        );
        for _ in 0..128 {
            super::super::behavior::secondary(&mut entities, spider, 20_000, 321, &mut fx).unwrap();
            if style_address(entities.entity_mut(spider).unwrap()).unwrap() == 0x4c8038 {
                break;
            }
        }
        let RetailRuntimeValue::Known(Some(context)) = entities
            .entity_mut(spider)
            .unwrap()
            .current_behavior_context
        else {
            panic!()
        };
        assert_eq!(context.active_style().style_address(), 0x4c8038);
        assert_eq!(
            context.target_handle_at_0x08(),
            RetailRuntimeValue::Known(Some(child))
        );
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler.adopt_intro2_type9(&mut entities);
        scheduler
            .adopt_fresh_level1_type9_selected(&mut entities)
            .unwrap();
        scheduler.adopt_intro2_type8(&mut entities);
        scheduler.adopt_intro2_type17(&entities);
        let order = entities.retail_live_order_ids().collect::<Vec<_>>();
        let (subject, candidate) = if order.iter().position(|id| *id == spider)
            < order.iter().position(|id| *id == child)
        {
            (spider, child)
        } else {
            (child, spider)
        };
        let mut fixture = Self {
            session,
            entities,
            scheduler,
            fx,
            notifications: GameplayNotifications::new(),
            static_damage: StaticDamageScheduler::new(),
            spider,
            child,
            subject,
            candidate,
            tick: 321,
        };
        fixture.place_oriented_contact();
        fixture
    }

    pub(crate) fn place_oriented_contact(&mut self) {
        let center = self
            .entities
            .entity_mut(self.spider)
            .unwrap()
            .position_raw();
        // Use the real subject-first sphere/plane interpreter; neither the
        // wrapped broad phase nor a model-radius overlap counts as contact.
        for dy in [-150_i16, -75, 0, 75, 150] {
            for dx in [80_i16, 160, 240, -80, -160, -240, 0] {
                for dz in [80_i16, 160, -80, -160, 0] {
                    self.entities
                        .entity_mut(self.child)
                        .unwrap()
                        .set_position_raw([
                            center[0].wrapping_add(dx),
                            center[1].wrapping_add(dy),
                            center[2].wrapping_add(dz),
                        ]);
                    if classify_oriented_active_pair_contact(
                        crate::player_active_contact::OrientedActivePairContactRequest {
                            subject: actor(&self.entities, self.subject).unwrap(),
                            candidate: actor(&self.entities, self.candidate).unwrap(),
                            subject_entry: &active_pair_body_from_entity(
                                actor(&self.entities, self.subject).unwrap(),
                                &self.session.cache,
                            ),
                            retail_tick: self.tick,
                        },
                        &self.session.cache,
                    )
                    .unwrap()
                    .is_some()
                    {
                        return;
                    }
                }
            }
        }
        panic!("real model256/person geometry did not contact in controlled scan");
    }

    pub(crate) fn pair(&mut self, feedback: CaptureFeedbackPolicy) -> Type17PairOutcome {
        resolve_type17_active_contacts(
            &mut Intro2ContactFrame {
                entities: &mut self.entities,
                resources: &mut self.session.cache,
                world_fx: &mut self.fx,
                static_damage: &mut self.static_damage,
                notifications: &mut self.notifications,
                retail_tick: self.tick,
                actor_tasks: &mut self.scheduler,
            },
            self.subject,
            feedback,
        )
    }
}

#[v2k_test_support::retail_test]
fn native_intro_and_later_world_capture_follow_both_real_intrusive_orders() {
    let mut orders = Vec::new();
    for (world, spawn, child_order) in [
        (50, Some(4), ChildOrder::BeforeSpider),
        (50, Some(4), ChildOrder::AfterSpider),
        (50, Some(30), ChildOrder::AfterSpider),
        (14, None, ChildOrder::First),
    ] {
        let mut fixture = Fixture::ready(world, spawn, child_order);
        let spider_first = fixture.subject == fixture.spider;
        orders.push(spider_first);
        let old_primary = fixture
            .entities
            .entity_mut(fixture.spider)
            .unwrap()
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        let before_spider = fixture
            .entities
            .entity_mut(fixture.spider)
            .unwrap()
            .position_raw();
        let before_child = fixture
            .entities
            .entity_mut(fixture.child)
            .unwrap()
            .position_raw();
        let basis = fixture
            .entities
            .entity_mut(fixture.spider)
            .unwrap()
            .physical_body_basis_q31();
        let result = fixture.pair(CaptureFeedbackPolicy::Cinematic);
        let Type17PairOutcome::Resolved { visits } = result else {
            panic!("world{world}: {result:?}")
        };
        assert_eq!(visits.len(), 1, "isolated actual pair");
        let visit = &visits[0];
        assert!(visit.physical_suppressed);
        assert!(!visit
            .stages
            .iter()
            .any(|stage| matches!(stage, Type17PairStage::Physical { .. })));
        let behavior_ids: Vec<_> = visit
            .stages
            .iter()
            .filter_map(|stage| match stage {
                Type17PairStage::Behavior { owner, .. } => Some(*owner),
                _ => None,
            })
            .collect();
        assert_eq!(behavior_ids, [fixture.subject, fixture.candidate]);
        let tag = visit
            .stages
            .iter()
            .position(|stage| *stage == Type17PairStage::DispatchCaptureAccepted)
            .unwrap();
        if spider_first {
            assert!(
                matches!(visit.stages[tag+1],Type17PairStage::Behavior{owner,..} if owner==fixture.child)
            );
        }
        let entity = fixture.entities.entity_mut(fixture.spider).unwrap();
        assert_eq!(
            entity.position_raw(),
            before_spider,
            "A300 suppresses displacement"
        );
        assert_eq!(
            entity.physical_body_basis_q31(),
            basis,
            "2DA0 retains incoming body matrix"
        );
        assert_ne!(
            entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
            Some(old_primary)
        );
        assert_eq!(
            style_address(entity).unwrap(),
            0x4c80c8,
            "no cap10 destination in isolated search cube"
        );
        let RetailRuntimeValue::Known(Some(rows)) = &entity.sub_j_attachment_runtime else {
            panic!()
        };
        assert_eq!(rows.ordered_entity_ids(), [fixture.child]);
        let child = fixture.entities.entity_mut(fixture.child).unwrap();
        assert_eq!(child.position_raw(), before_child);
        assert_eq!(child.attached_to, Some(fixture.spider));
        assert_eq!(
            child.collision.state_flags_at_0x08.masked(0x800),
            RetailRuntimeValue::Known(0x800),
            "18440 policy1 retains the captured child's ordinary model visibility",
        );
        assert!(matches!(
            child.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::None)
        ));
        assert!(visit.stages.iter().any(|stage|matches!(stage,Type17PairStage::Component{owner,slot:ActorTaskSlot::Primary,result:Type17PairComponentResult::Null} if *owner==fixture.child)));
        assert!(fixture
            .scheduler
            .prepare_native_actor_mutation(&fixture.entities, fixture.spider));
        assert_eq!(
            fixture.notifications,
            GameplayNotifications::new(),
            "cinematic callback cannot consume event8 dedup"
        );
        fixture
            .notifications
            .queue_spider_capture(fixture.tick as i32 + 1);
        assert_ne!(
            fixture.notifications,
            GameplayNotifications::new(),
            "subsequent gameplay event8 remains available"
        );
        let next = fixture.pair(CaptureFeedbackPolicy::Cinematic);
        assert!(
            matches!(next,Type17PairOutcome::Resolved{ref visits} if visits.is_empty()),
            "{next:?}"
        );
    }
    assert!(
        orders.contains(&true) && orders.contains(&false),
        "actual Intro2 authored participants cover both callback directions"
    );
}

#[v2k_test_support::retail_test]
fn capture_requires_real_overlap_and_completed_native_scheduler_custody() {
    let mut fixture = Fixture::ready(14, None, ChildOrder::First);
    let child_position = fixture
        .entities
        .entity_mut(fixture.child)
        .unwrap()
        .position_raw();
    fixture.scheduler = SpecializedActorTaskScheduler::new();
    fixture
        .entities
        .entity_mut(fixture.child)
        .unwrap()
        .set_position_raw([0, child_position[1], 0]);
    assert!(
        matches!(fixture.pair(CaptureFeedbackPolicy::Gameplay),Type17PairOutcome::Resolved{visits} if visits.is_empty())
    );
    fixture
        .entities
        .entity_mut(fixture.child)
        .unwrap()
        .set_position_raw(child_position);
    let before = fixture
        .entities
        .entity_mut(fixture.spider)
        .unwrap()
        .current_behavior_context;
    let result = fixture.pair(CaptureFeedbackPolicy::Gameplay);
    assert!(
        matches!(
            result,
            Type17PairOutcome::Blocked {
                committed_prefix: false,
                ..
            }
        ),
        "{result:?}"
    );
    assert_eq!(
        fixture
            .entities
            .entity_mut(fixture.spider)
            .unwrap()
            .current_behavior_context,
        before
    );
    assert_eq!(
        fixture
            .entities
            .entity_mut(fixture.child)
            .unwrap()
            .attached_to,
        None
    );
    assert_eq!(fixture.notifications, GameplayNotifications::new());
}

#[v2k_test_support::retail_test]
fn captured_pair_retains_its_new_tasks_through_the_next_full_scheduler_pass() {
    for (world, spawn, child_order) in [
        (50, Some(4), ChildOrder::AfterSpider),
        (14, None, ChildOrder::First),
    ] {
        let mut fixture = Fixture::ready(world, spawn, child_order);
        let result = fixture.pair(CaptureFeedbackPolicy::Cinematic);
        assert!(
            matches!(result, Type17PairOutcome::Resolved { ref visits }
                if visits.len() == 1 && visits[0].physical_suppressed),
            "{result:?}"
        );
        let parent = fixture.entities.entity_mut(fixture.spider).unwrap();
        let primary = parent
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        let Some(ActorTaskRuntime::SharedRetarget(task)) =
            parent.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("C6B0 published the real 500ms Capture3 task")
        };
        assert_eq!((task.elapsed_ms(), task.lifetime_ms()), (0, 500));
        // The controlled presentation admits a detailed callback. It does not
        // replace either participant's task, constructor receipt or attachment.
        parent
            .collision
            .state_flags_at_0x08
            .overwrite(0x60000, 0x60000);
        let child_position = fixture
            .entities
            .entity_mut(fixture.child)
            .unwrap()
            .position_raw();
        let sea = fixture.session.cache.terrain().unwrap().sea_level_raw();
        let child_extent = fixture
            .session
            .cache
            .global_model(crate::main_base_type9_abort::LEVEL_ONE_TYPE9_MODEL_ID)
            .unwrap()
            .radius;
        assert!(
            i32::from(child_position[1]) >= i32::from(sea) - i32::from(child_extent >> 2),
            "the scheduler control must begin outside E370's deep-water branch"
        );
        fixture.tick += 1;
        let pass = fixture.scheduler.tick(
            &mut fixture.entities,
            SpecializedActorTaskProductionFrame {
                world:
                    crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                hive_components: None,
                notification_phase:
                    crate::gameplay_notifications::GameplayNotificationPhase::NonGameplay,
                resources: &mut fixture.session.cache,
                world_fx: &mut fixture.fx,
                static_damage: &mut fixture.static_damage,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: fixture.tick,
                main_base_abort_active: false,
            },
            &mut fixture.notifications,
        );
        assert!(pass.block.is_none(), "{:?}", pass.block);
        let parent_pass = pass
            .outcomes
            .iter()
            .find(|outcome| outcome.entity_id() == fixture.spider)
            .unwrap();
        assert!(
            matches!(
                parent_pass,
                SpecializedActorTaskProductionOutcome::Intro2Type17(
                    super::super::Intro2Type17Outcome::Advanced {
                        callback_enabled: true,
                        callback_elapsed_micros: 20_000,
                        ..
                    }
                )
            ),
            "{parent_pass:?}"
        );
        let child_pass = pass
            .outcomes
            .iter()
            .find(|outcome| outcome.entity_id() == fixture.child)
            .unwrap();
        assert!(matches!(child_pass,
            SpecializedActorTaskProductionOutcome::OrdinaryType9Carried(
                crate::ordinary_type9_carried_production::Type9CarriedProductionOutcome::Continuing { .. }
                | crate::ordinary_type9_carried_production::Type9CarriedProductionOutcome::SchedulerWaiting { .. }
            )), "{child_pass:?}");
        let parent = fixture.entities.entity_mut(fixture.spider).unwrap();
        assert_eq!(
            parent.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
            Some(primary)
        );
        let Some(ActorTaskRuntime::SharedRetarget(task)) =
            parent.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("the same carried Primary remains installed")
        };
        assert_eq!((task.elapsed_ms(), task.lifetime_ms()), (20, 500));
        let child = fixture.entities.entity_mut(fixture.child).unwrap();
        assert_eq!(child.attached_to, Some(fixture.spider));
        assert!(matches!(
            child.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::None)
        ));
        assert_ne!(
            child.position_raw(),
            child_position,
            "the actual Sub-J tail places the captured child"
        );
        assert!(fixture
            .scheduler
            .prepare_native_actor_mutation(&fixture.entities, fixture.spider));
        assert!(capture::CaptureTaskCustody::capture_child_mutation_ready(
            &mut fixture.scheduler,
            &fixture.entities,
            fixture.child
        ));
    }
}
