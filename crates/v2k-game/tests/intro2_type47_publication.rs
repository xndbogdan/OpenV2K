#[path = "intro2_type47_publication/birth.rs"]
mod birth;

use v2k_game::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    aim_and_fire::{AimAndFireFrameOutcome, AimAndFireTransitionReason},
    chase_target::ChaseTargetLifetimeStatus,
    entity::{EntityConstructionResources, EntityManager},
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue, RetailStateWord},
    guard_location_owner::acquisition::{
        GuardLocationAcquisitionCallbackPrefix, GuardLocationAcquisitionCallbackResult,
    },
    intro2_type47_live::{
        intro2_type47_seed_for_spawn, tick_intro2_type47_scheduler_owner,
        Intro2Type47CallbackFrame, Intro2Type47ChaseVisitResult, Intro2Type47PrimaryVisitResult,
        Intro2Type47SchedulerOwner, Intro2Type47SchedulerOwnerTick,
        Intro2Type47SchedulerProductionOutcome, INTRO2_TYPE47_ENTITY_TYPE, INTRO2_TYPE47_MODEL_ID,
        INTRO2_TYPE47_SPAWN_INDICES, INTRO2_TYPE47_SUB_D_SEEDS,
    },
    opening::intro2_uses_live_actor_pose,
    ordinary_type47_live::OrdinaryType47LiveError,
    session::GameSession,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    type47_c690::{Type47C690Slot, TYPE47_C690_SUPPRESS_STATE_BIT, TYPE47_IMPACT_DYING_STATE_BIT},
    type47_initial_behavior_live::TYPE47_GUARD_BEHAVIOR_CLASS_ID,
    type47_scheduler_production::Type47SchedulerC690Transition,
    world_fx::WorldFx,
};

#[test]
fn intro2_type47_seeds_bind_only_spawns_6_7_8() {
    for (spawn_index, seed) in INTRO2_TYPE47_SPAWN_INDICES
        .into_iter()
        .zip(INTRO2_TYPE47_SUB_D_SEEDS)
    {
        assert_eq!(intro2_type47_seed_for_spawn(spawn_index), Some(seed));
    }
    assert_eq!(intro2_type47_seed_for_spawn(11), None);
    assert_eq!(intro2_type47_seed_for_spawn(12), None);
    assert_eq!(intro2_type47_seed_for_spawn(13), None);
    assert_eq!(intro2_type47_seed_for_spawn(25), None);
}

#[v2k_test_support::retail_test]
fn captured_intro2_guard_acquires_type9_then_visits_new_aim_same_pass() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("initialize retail-data session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-tier cumulative Section-12 records");
    session
        .load_level_by_id(50, 1)
        .expect("load normal-tier Intro2");
    let type_metadata = intro2_type_metadata(&session);
    let level = session.cache.level_desc().expect("Intro2 Section 13");
    let model_extent = |id| session.cache.global_model(id).map(|model| model.radius);
    let resources = EntityConstructionResources {
        terrain: session.cache.terrain(),
        terrain_objects: session.cache.terrain_objects(),
        model_extent_raw: Some(&model_extent),
    };
    let mut manager = EntityManager::from_captured_intro2_frontend_with_type_metadata(
        level,
        &type_metadata,
        resources,
        v2k_game::entity::Intro2BirthSelection {
            type47: v2k_game::intro2_type47_live::Intro2Type47BirthSelection::CapturedGuard,
            ..Default::default()
        },
        &mut || 1,
    )
    .expect("accepted frontend-to-Intro2 constructor receipt");
    let source_id = manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(6))
        .expect("captured spawn 6")
        .id;
    let owner = Intro2Type47SchedulerOwner::adopt_published(
        manager
            .iter_all()
            .find(|entity| entity.id == source_id)
            .unwrap(),
    )
    .expect("published Intro2 Guard owner");
    let mut world_fx = v2k_game::world_fx::WorldFx::new();
    let mut words = [1_u32, 0].into_iter();
    let mut draws = Vec::new();
    let tick = tick_intro2_type47_scheduler_owner(
        &mut manager,
        owner,
        &mut world_fx,
        session.cache.terrain(),
        Intro2Type47CallbackFrame {
            elapsed_micros: 20_000,
            global_elapsed_micros: 20_000,
        },
        &mut |_| {
            let word = words
                .next()
                .expect("Primary retarget then Secondary acquisition consume two words");
            draws.push(word);
            word
        },
    );
    assert_eq!(draws, [1, 0]);
    assert_eq!(words.next(), None);
    let (candidate_id, same_pass_aim) = match tick.outcome {
        Intro2Type47SchedulerProductionOutcome::Primary {
            acquisition_prefix:
                GuardLocationAcquisitionCallbackPrefix::Acquire {
                    random_low16: 0, ..
                },
            acquisition:
                GuardLocationAcquisitionCallbackResult::TaggedCandidateAccepted { candidate, .. },
            same_pass_aim,
            ..
        } => (candidate.id, same_pass_aim),
        other => panic!("expected accepted Guard handoff, got {other:?}"),
    };
    assert!(same_pass_aim.is_some_and(|result| result.is_ok()));
    assert_eq!(
        manager
            .iter_all()
            .find(|entity| entity.id == candidate_id)
            .map(|entity| entity.entity_type),
        Some(9),
        "accepted passive Intro2 targets are Type-9 allocations"
    );
    let source = manager
        .iter_all()
        .find(|entity| entity.id == source_id)
        .expect("source survives the handoff");
    let Some(ActorTaskRuntime::ChaseTarget(chase)) =
        source.actor_task_state(ActorTaskSlot::Primary)
    else {
        panic!("ADE0 must replace Primary with Chase")
    };
    assert_eq!(chase.target_id(), candidate_id);
    assert_eq!(
        chase.elapsed_ms(),
        0,
        "new Primary is not revisited same pass"
    );
    assert_eq!(source.actor_task_state(ActorTaskSlot::Secondary), None);
    let Some(ActorTaskRuntime::AimAndFire(aim)) = source.actor_task_state(ActorTaskSlot::Tertiary)
    else {
        panic!("ADE0 must publish Tertiary Aim")
    };
    assert_eq!(aim.private_state().target_entity_id(), candidate_id);
    assert_eq!(
        aim.elapsed_ms(),
        20,
        "new Tertiary is visited in the acquisition pass"
    );

    let retained = tick.retained_owner.expect("pursuing owner stays live");
    let next = tick_intro2_type47_scheduler_owner(
        &mut manager,
        retained,
        &mut world_fx,
        session.cache.terrain(),
        Intro2Type47CallbackFrame {
            elapsed_micros: 20_000,
            global_elapsed_micros: 20_000,
        },
        &mut |_| 1,
    );
    match next.outcome {
        Intro2Type47SchedulerProductionOutcome::Pursuing {
            chase,
            aim: Some(Ok(_)),
            ..
        } => assert!(matches!(
            chase.result,
            Intro2Type47ChaseVisitResult::Continue | Intro2Type47ChaseVisitResult::Tagged(_)
        )),
        other => panic!("expected subsequent Chase-then-Aim pass, got {other:?}"),
    }
    let source = manager
        .iter_all()
        .find(|entity| entity.id == source_id)
        .unwrap();
    assert!(source.intro2_type47_sub_d_frame_owner.is_some());
    assert!(source.intro2_type47_sub_d_runtime.is_some());
    assert!(source
        .ordinary_type47_aim_and_fire_runtime
        .as_ref()
        .is_some_and(|runtime| runtime.sub_d_frame_owner().is_none()));
    let Some(ActorTaskRuntime::ChaseTarget(chase)) =
        source.actor_task_state(ActorTaskSlot::Primary)
    else {
        panic!("Pursuing Primary remains Chase")
    };
    assert_eq!(chase.elapsed_ms(), 20);
    let Some(ActorTaskRuntime::AimAndFire(aim)) = source.actor_task_state(ActorTaskSlot::Tertiary)
    else {
        panic!("Pursuing Tertiary remains Aim")
    };
    assert_eq!(aim.elapsed_ms(), 40);
}

#[v2k_test_support::retail_test]
fn intro2_aim_rejects_a_level1_type46_target_after_handoff() {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).expect("initialize retail-data session");
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(50, 1).unwrap();
    let type_metadata = intro2_type_metadata(&session);
    let model_extent = |id| session.cache.global_model(id).map(|model| model.radius);
    let resources = EntityConstructionResources {
        terrain: session.cache.terrain(),
        terrain_objects: session.cache.terrain_objects(),
        model_extent_raw: Some(&model_extent),
    };
    let mut manager = EntityManager::from_captured_intro2_frontend_with_type_metadata(
        session.cache.level_desc().unwrap(),
        &type_metadata,
        resources,
        v2k_game::entity::Intro2BirthSelection {
            type47: v2k_game::intro2_type47_live::Intro2Type47BirthSelection::CapturedGuard,
            ..Default::default()
        },
        &mut || 1,
    )
    .unwrap();
    let source = manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(6))
        .unwrap();
    let owner = Intro2Type47SchedulerOwner::adopt_published(source).unwrap();
    let mut world_fx = v2k_game::world_fx::WorldFx::new();
    let mut words = [1_u32, 0].into_iter();
    let first = tick_intro2_type47_scheduler_owner(
        &mut manager,
        owner,
        &mut world_fx,
        session.cache.terrain(),
        Intro2Type47CallbackFrame {
            elapsed_micros: 20_000,
            global_elapsed_micros: 20_000,
        },
        &mut |_| words.next().unwrap(),
    );
    let candidate_id = match first.outcome {
        Intro2Type47SchedulerProductionOutcome::Primary {
            acquisition:
                GuardLocationAcquisitionCallbackResult::TaggedCandidateAccepted { candidate, .. },
            ..
        } => candidate.id,
        other => panic!("expected handoff, got {other:?}"),
    };
    manager.entity_mut(candidate_id).unwrap().entity_type = 46;
    let second = tick_intro2_type47_scheduler_owner(
        &mut manager,
        first.retained_owner.unwrap(),
        &mut world_fx,
        session.cache.terrain(),
        Intro2Type47CallbackFrame {
            elapsed_micros: 20_000,
            global_elapsed_micros: 20_000,
        },
        &mut |_| 1,
    );
    assert!(matches!(
        second.outcome,
        Intro2Type47SchedulerProductionOutcome::Pursuing {
            aim: Some(Err(OrdinaryType47LiveError::TargetAllocationTypeMismatch {
                actual: 46
            })),
            ..
        }
    ));
}

fn intro2_type_metadata(session: &GameSession) -> Vec<EntityTypeRuntimeMetadata> {
    session
        .cache
        .global_entity_model_table()
        .iter()
        .copied()
        .enumerate()
        .map(|(entity_type, model_slots)| {
            session
                .cache
                .global_entity_type(entity_type)
                .map(EntityTypeRuntimeMetadata::from_section12)
                .unwrap_or(EntityTypeRuntimeMetadata {
                    model_slots,
                    ..EntityTypeRuntimeMetadata::default()
                })
        })
        .collect()
}

struct Intro2PursuingFixture {
    session: GameSession,
    manager: EntityManager,
    owner: Intro2Type47SchedulerOwner,
    source_id: u32,
    target_id: u32,
    world_fx: WorldFx,
}

fn intro2_pursuing_fixture() -> Intro2PursuingFixture {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).expect("initialize retail-data session");
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(50, 1).unwrap();
    let type_metadata = intro2_type_metadata(&session);
    let model_extent = |id| session.cache.global_model(id).map(|model| model.radius);
    let resources = EntityConstructionResources {
        terrain: session.cache.terrain(),
        terrain_objects: session.cache.terrain_objects(),
        model_extent_raw: Some(&model_extent),
    };
    let mut manager = EntityManager::from_captured_intro2_frontend_with_type_metadata(
        session.cache.level_desc().unwrap(),
        &type_metadata,
        resources,
        v2k_game::entity::Intro2BirthSelection {
            type47: v2k_game::intro2_type47_live::Intro2Type47BirthSelection::CapturedGuard,
            ..Default::default()
        },
        &mut || 1,
    )
    .unwrap();
    let source = manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(6))
        .unwrap();
    let source_id = source.id;
    let owner = Intro2Type47SchedulerOwner::adopt_published(source).unwrap();
    let mut world_fx = WorldFx::new();
    let mut words = [1_u32, 0].into_iter();
    let first = tick_intro2_type47_scheduler_owner(
        &mut manager,
        owner,
        &mut world_fx,
        session.cache.terrain(),
        Intro2Type47CallbackFrame {
            elapsed_micros: 20_000,
            global_elapsed_micros: 20_000,
        },
        &mut |_| words.next().unwrap(),
    );
    let target_id = match first.outcome {
        Intro2Type47SchedulerProductionOutcome::Primary {
            acquisition:
                GuardLocationAcquisitionCallbackResult::TaggedCandidateAccepted { candidate, .. },
            same_pass_aim: Some(Ok(_)),
            ..
        } => candidate.id,
        other => panic!("expected Intro2 pursuing fixture, got {other:?}"),
    };
    Intro2PursuingFixture {
        session,
        manager,
        owner: first.retained_owner.expect("pursuing owner retained"),
        source_id,
        target_id,
        world_fx,
    }
}

fn tick_intro2_fixture(
    fixture: &mut Intro2PursuingFixture,
    elapsed_micros: u32,
    next_random: &mut impl FnMut(&mut WorldFx) -> u32,
) -> Intro2Type47SchedulerOwnerTick {
    tick_intro2_type47_scheduler_owner(
        &mut fixture.manager,
        fixture.owner,
        &mut fixture.world_fx,
        fixture.session.cache.terrain(),
        Intro2Type47CallbackFrame {
            elapsed_micros: elapsed_micros,
            global_elapsed_micros: elapsed_micros,
        },
        next_random,
    )
}

#[v2k_test_support::retail_test]
fn intro2_chase_tag_applies_plus00_guard_then_fresh_reads_secondary() {
    let mut fixture = intro2_pursuing_fixture();
    let preserved_context = fixture
        .manager
        .iter_all()
        .find(|entity| entity.id == fixture.source_id)
        .and_then(|entity| match entity.current_behavior_context {
            RetailRuntimeValue::Known(Some(context)) => Some(context),
            _ => None,
        })
        .unwrap();
    fixture
        .manager
        .entity_mut(fixture.target_id)
        .unwrap()
        .active = false;
    let mut words = [0_u32, 1].into_iter();
    let mut draws = Vec::new();
    let tick = tick_intro2_fixture(&mut fixture, 20_000, &mut |_| {
        let word = words
            .next()
            .expect("C690 selector then fresh Secondary acquisition");
        draws.push(word);
        word
    });
    assert_eq!(draws, [0, 1], "outcome: {:?}", tick.outcome);
    assert_eq!(words.next(), None);
    assert!(matches!(
        tick.outcome,
        Intro2Type47SchedulerProductionOutcome::Pursuing {
            chase: v2k_game::intro2_type47_live::Intro2Type47ChaseVisit {
                result: Intro2Type47ChaseVisitResult::Tagged(_),
                ..
            },
            post_chase_plus00_c690: Some(Type47SchedulerC690Transition::GuardPublished {
                slot: Type47C690Slot::Plus00,
                ..
            }),
            post_chase_guard: Some(
                v2k_game::intro2_type47_live::Intro2Type47PostChaseGuardPass {
                    same_pass_aim: None,
                    ..
                }
            ),
            aim: None,
            post_aim_plus04_c690: None,
            post_aim_plus00_c690: None,
            ..
        }
    ));
    assert!(tick.retained_owner.is_some());
    let source = fixture
        .manager
        .iter_all()
        .find(|entity| entity.id == fixture.source_id)
        .unwrap();
    assert!(matches!(
        source.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::OrdinaryType9Wander(_))
    ));
    assert!(matches!(
        source.actor_task_state(ActorTaskSlot::Secondary),
        Some(ActorTaskRuntime::GuardLocationAcquisition(_))
    ));
    assert_eq!(source.actor_task_state(ActorTaskSlot::Tertiary), None);
    let RetailRuntimeValue::Known(Some(reselected)) = source.current_behavior_context else {
        panic!("C690 reuses the allocated behavior context")
    };
    assert_eq!(
        reselected.target_handle_at_0x08(),
        preserved_context.target_handle_at_0x08()
    );
    assert_eq!(
        reselected.auxiliary_word_at_0x0c(),
        preserved_context.auxiliary_word_at_0x0c()
    );
    fixture.owner = tick.retained_owner.unwrap();
    let next = tick_intro2_fixture(&mut fixture, 20_000, &mut |_| 1);
    assert!(matches!(
        next.outcome,
        Intro2Type47SchedulerProductionOutcome::Primary { .. }
    ));
}

#[v2k_test_support::retail_test]
fn intro2_chase_tag_can_reselect_lone_wander_and_retain_owner() {
    let mut fixture = intro2_pursuing_fixture();
    fixture
        .manager
        .entity_mut(fixture.target_id)
        .unwrap()
        .active = false;
    let mut draws = 0;
    let tick = tick_intro2_fixture(&mut fixture, 20_000, &mut |_| {
        draws += 1;
        0xFFFF
    });
    assert_eq!(
        draws, 1,
        "Wander publishes no later Secondary slot; outcome: {:?}",
        tick.outcome
    );
    assert!(
        matches!(
            tick.outcome,
            Intro2Type47SchedulerProductionOutcome::Pursuing {
                post_chase_plus00_c690: Some(Type47SchedulerC690Transition::WanderPublished {
                    slot: Type47C690Slot::Plus00,
                    ..
                }),
                post_chase_guard: None,
                aim: None,
                ..
            }
        ),
        "outcome: {:?}",
        tick.outcome
    );
    let owner = tick
        .retained_owner
        .expect("Intro2 owns the lone Wander graph");
    let source = fixture
        .manager
        .iter_all()
        .find(|entity| entity.id == fixture.source_id)
        .unwrap();
    assert!(matches!(
        source.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::OrdinaryType9Wander(_))
    ));
    assert_eq!(source.actor_task_state(ActorTaskSlot::Secondary), None);
    assert_eq!(source.actor_task_state(ActorTaskSlot::Tertiary), None);

    fixture.owner = owner;
    let next = tick_intro2_fixture(&mut fixture, 20_000, &mut |_| 1);
    assert!(matches!(
        next.outcome,
        Intro2Type47SchedulerProductionOutcome::WanderNear { .. }
    ));
    assert!(next.retained_owner.is_some());
}

#[v2k_test_support::retail_test]
fn intro2_aim_lifetime_applies_plus00_after_chase_continue() {
    let mut fixture = intro2_pursuing_fixture();
    let tick = tick_intro2_fixture(&mut fixture, 4_981_000, &mut |_| 0);
    assert!(matches!(
        tick.outcome,
        Intro2Type47SchedulerProductionOutcome::Pursuing {
            chase: v2k_game::intro2_type47_live::Intro2Type47ChaseVisit {
                result: Intro2Type47ChaseVisitResult::Continue,
                ..
            },
            post_chase_plus00_c690: None,
            aim: Some(Ok(ref aim)),
            post_aim_plus04_c690: None,
            post_aim_plus00_c690: Some(
                Type47SchedulerC690Transition::GuardPublished {
                    slot: Type47C690Slot::Plus00,
                    ..
                }
            ),
            ..
        } if matches!(
            aim.resolution.outcome,
            AimAndFireFrameOutcome::RequestOwnerTransition {
                reason: AimAndFireTransitionReason::LifetimeExpired
            }
        )
    ));
    assert!(tick.retained_owner.is_some());
    let source = fixture
        .manager
        .iter_all()
        .find(|entity| entity.id == fixture.source_id)
        .unwrap();
    assert!(matches!(
        source.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::OrdinaryType9Wander(_))
    ));
    assert!(matches!(
        source.actor_task_state(ActorTaskSlot::Secondary),
        Some(ActorTaskRuntime::GuardLocationAcquisition(_))
    ));
    assert_eq!(source.actor_task_state(ActorTaskSlot::Tertiary), None);
}

#[v2k_test_support::retail_test]
fn intro2_chase_lifetime_applies_plus00_before_older_aim() {
    let mut fixture = intro2_pursuing_fixture();
    let tick = tick_intro2_fixture(&mut fixture, 5_001_000, &mut |_| 0);
    assert!(matches!(
        tick.outcome,
        Intro2Type47SchedulerProductionOutcome::Pursuing {
            chase: v2k_game::intro2_type47_live::Intro2Type47ChaseVisit {
                prefix: v2k_game::chase_target::ChaseTargetCallbackPrefix {
                    lifetime_status: ChaseTargetLifetimeStatus::OwnerTransitionDue,
                    ..
                },
                result: Intro2Type47ChaseVisitResult::Continue,
            },
            post_chase_plus00_c690: Some(Type47SchedulerC690Transition::GuardPublished {
                slot: Type47C690Slot::Plus00,
                ..
            }),
            post_chase_guard: Some(_),
            aim: None,
            post_aim_plus04_c690: None,
            post_aim_plus00_c690: None,
            ..
        }
    ));
    assert!(tick.retained_owner.is_some());
}

#[v2k_test_support::retail_test]
fn intro2_state_gate_orders_chase_plus00_then_aim_plus04_and_timeout_plus00() {
    let mut fixture = intro2_pursuing_fixture();
    fixture
        .manager
        .entity_mut(fixture.target_id)
        .unwrap()
        .active = false;
    let source = fixture.manager.entity_mut(fixture.source_id).unwrap();
    source.collision.state_flags_at_0x08 = RetailStateWord::exact(
        source.collision.state_flags_at_0x08.known_value_bits() | TYPE47_C690_SUPPRESS_STATE_BIT,
    );
    let mut draws = 0;
    let tick = tick_intro2_fixture(&mut fixture, 4_981_000, &mut |_| {
        draws += 1;
        0
    });
    assert_eq!(draws, 0, "all three outer state gates precede AC60 RNG");
    assert!(matches!(
        tick.outcome,
        Intro2Type47SchedulerProductionOutcome::Pursuing {
            post_chase_plus00_c690: Some(
                Type47SchedulerC690Transition::SuppressedByEntityState {
                    slot: Type47C690Slot::Plus00
                }
            ),
            aim: Some(Ok(ref aim)),
            post_aim_plus04_c690: Some(
                Type47SchedulerC690Transition::SuppressedByEntityState {
                    slot: Type47C690Slot::Plus04
                }
            ),
            post_aim_plus00_c690: Some(
                Type47SchedulerC690Transition::SuppressedByEntityState {
                    slot: Type47C690Slot::Plus00
                }
            ),
            ..
        } if matches!(
            aim.resolution.outcome,
            AimAndFireFrameOutcome::RequestOwnerTransition {
                reason: AimAndFireTransitionReason::TaggedInvalidTarget { .. }
            }
        )
    ));
    assert!(tick.retained_owner.is_some());
    let source = fixture
        .manager
        .iter_all()
        .find(|entity| entity.id == fixture.source_id)
        .unwrap();
    assert!(matches!(
        source.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::ChaseTarget(_))
    ));
    assert!(matches!(
        source.actor_task_state(ActorTaskSlot::Tertiary),
        Some(ActorTaskRuntime::AimAndFire(_))
    ));
}

#[v2k_test_support::retail_test]
fn intro2_source_dying_c690_publishes_class12_without_selector_rng_or_stale_aim() {
    let mut fixture = intro2_pursuing_fixture();
    fixture
        .manager
        .entity_mut(fixture.target_id)
        .unwrap()
        .active = false;
    let source = fixture.manager.entity_mut(fixture.source_id).unwrap();
    source.collision.state_flags_at_0x08 = RetailStateWord::exact(
        source.collision.state_flags_at_0x08.known_value_bits() | TYPE47_IMPACT_DYING_STATE_BIT,
    );
    let health_before = source.collision.health_raw;
    let mut draws = 0;
    let tick = tick_intro2_fixture(&mut fixture, 20_000, &mut |_| {
        draws += 1;
        0
    });
    assert_eq!(draws, 0, "alternate +0x124 branch consumes no selector RNG");
    assert!(matches!(
        tick.outcome,
        Intro2Type47SchedulerProductionOutcome::Pursuing {
            post_chase_plus00_c690: Some(Type47SchedulerC690Transition::Class12Published { .. }),
            aim: None,
            ..
        }
    ));
    assert!(tick.retained_owner.is_none());
    let source = fixture
        .manager
        .iter_all()
        .find(|entity| entity.id == fixture.source_id)
        .unwrap();
    assert!(matches!(
        source.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::CommonDying(_))
    ));
    assert!(source.actor_task_state(ActorTaskSlot::Tertiary).is_none());
    assert_eq!(
        source.collision.health_raw, health_before,
        "C690 is not the generic death prefix"
    );
}

#[v2k_test_support::retail_test]
fn captured_intro2_route_publishes_three_guard_anchor_graphs() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("initialize retail-data session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-tier cumulative Section-12 records");
    session
        .load_level_by_id(50, 1)
        .expect("load normal-tier Intro2");

    let type_metadata = intro2_type_metadata(&session);
    let level = session.cache.level_desc().expect("Intro2 Section 13");
    let model_extent = |id| session.cache.global_model(id).map(|model| model.radius);
    let resources = EntityConstructionResources {
        terrain: session.cache.terrain(),
        terrain_objects: session.cache.terrain_objects(),
        model_extent_raw: Some(&model_extent),
    };
    let direct = EntityManager::from_level_with_type_metadata(level, &type_metadata, resources);
    for spawn_index in INTRO2_TYPE47_SPAWN_INDICES {
        let entity = direct
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(spawn_index))
            .expect("Intro2 authored Type-47 spawn");
        assert_eq!(entity.entity_type, INTRO2_TYPE47_ENTITY_TYPE);
        assert_eq!(entity.initial_behavior, RetailRuntimeValue::Unresolved);
        assert_eq!(
            entity.current_behavior_context,
            RetailRuntimeValue::Unresolved
        );
        assert!(entity.intro2_type47_sub_d_frame_owner.is_none());
        assert!(entity.intro2_type47_sub_d_runtime.is_none());
        for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
            assert_eq!(entity.actor_task_state(slot), None);
        }
    }

    let mut captured = EntityManager::from_captured_intro2_frontend_with_type_metadata(
        level,
        &type_metadata,
        resources,
        v2k_game::entity::Intro2BirthSelection {
            type47: v2k_game::intro2_type47_live::Intro2Type47BirthSelection::CapturedGuard,
            ..Default::default()
        },
        &mut || 0,
    )
    .expect("accepted frontend-to-Intro2 constructor receipt");

    let mut published = captured
        .iter_all()
        .filter(|entity| {
            entity.active
                && entity.entity_type == INTRO2_TYPE47_ENTITY_TYPE
                && entity
                    .authored_spawn_index
                    .is_some_and(|index| intro2_type47_seed_for_spawn(index).is_some())
        })
        .collect::<Vec<_>>();
    published.sort_by_key(|entity| entity.authored_spawn_index);
    assert_eq!(published.len(), INTRO2_TYPE47_SPAWN_INDICES.len());
    for (entity, (spawn_index, seed)) in published.iter().zip(
        INTRO2_TYPE47_SPAWN_INDICES
            .into_iter()
            .zip(INTRO2_TYPE47_SUB_D_SEEDS),
    ) {
        assert_eq!(entity.authored_spawn_index, Some(spawn_index));
        let RetailRuntimeValue::Known(Some(selection)) = entity.initial_behavior else {
            panic!("captured route must publish the Guard selection")
        };
        assert_eq!(selection.choice_index, 0);
        assert_eq!(selection.program.class_id, TYPE47_GUARD_BEHAVIOR_CLASS_ID);
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::OrdinaryType9Wander(_))
        ));
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Secondary),
            Some(ActorTaskRuntime::GuardLocationAcquisition(_))
        ));
        assert_eq!(entity.actor_task_state(ActorTaskSlot::Tertiary), None);
        let owner = entity
            .intro2_type47_sub_d_frame_owner
            .expect("Intro2 Type-47 must retain the TTD first-query reset");
        assert!(owner.classifier_cache().can_classify());
        assert_eq!(owner.classifier_cache().stagger_counter(), seed);
        assert!(entity.intro2_type47_sub_d_runtime.is_some());
        assert!(intro2_uses_live_actor_pose(entity));
    }

    let mut scheduler = SpecializedActorTaskScheduler::new();
    assert_eq!(scheduler.adopt_intro2_type47_guards(&captured), 3);
    let mut empty = SpecializedActorTaskScheduler::new();
    assert_eq!(empty.adopt_intro2_type47_guards(&direct), 0);

    let first_guard_id = published[0].id;
    captured.entity_mut(first_guard_id).unwrap().model_index = Some(301);
    assert!(Intro2Type47SchedulerOwner::adopt_published(
        captured
            .iter_all()
            .find(|entity| entity.id == first_guard_id)
            .unwrap()
    )
    .is_err());
    captured.entity_mut(first_guard_id).unwrap().model_index = Some(INTRO2_TYPE47_MODEL_ID);

    let owners = captured
        .iter_all()
        .filter_map(|entity| Intro2Type47SchedulerOwner::adopt_published(entity).ok())
        .collect::<Vec<_>>();
    assert_eq!(owners.len(), 3);
    let mut world_fx = v2k_game::world_fx::WorldFx::new();
    for owner in owners {
        let entity_id = owner.entity_id();
        let tick = tick_intro2_type47_scheduler_owner(
            &mut captured,
            owner,
            &mut world_fx,
            session.cache.terrain(),
            Intro2Type47CallbackFrame {
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
            },
            &mut |_| 1,
        );
        match tick.outcome {
            Intro2Type47SchedulerProductionOutcome::Primary { visit, .. } => {
                assert!(matches!(
                    visit.result,
                    Intro2Type47PrimaryVisitResult::Continue
                        | Intro2Type47PrimaryVisitResult::TaggedZeroMover
                ));
            }
            other => panic!("expected Primary visit, got {other:?}"),
        }
        assert!(tick.retained_owner.is_some());
        let entity = captured
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .expect("Intro2 Type-47 remains after Primary");
        let owner = entity
            .intro2_type47_sub_d_frame_owner
            .expect("first-query owner survives the visit");
        assert!(matches!(
            owner.classifier_cache().origin(),
            RetailRuntimeValue::Known(_)
        ));
    }
}

#[v2k_test_support::retail_test]
fn captured_intro2_type47_guards_translate_after_master_motion() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("initialize retail-data session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-tier cumulative Section-12 records");
    session
        .load_level_by_id(50, 1)
        .expect("load normal-tier Intro2");

    let type_metadata = intro2_type_metadata(&session);
    let level = session.cache.level_desc().expect("Intro2 Section 13");
    let model_extent = |id| session.cache.global_model(id).map(|model| model.radius);
    let resources = EntityConstructionResources {
        terrain: session.cache.terrain(),
        terrain_objects: session.cache.terrain_objects(),
        model_extent_raw: Some(&model_extent),
    };
    let mut captured = EntityManager::from_captured_intro2_frontend_with_type_metadata(
        level,
        &type_metadata,
        resources,
        v2k_game::entity::Intro2BirthSelection {
            type47: v2k_game::intro2_type47_live::Intro2Type47BirthSelection::CapturedGuard,
            ..Default::default()
        },
        &mut || 0,
    )
    .expect("accepted frontend-to-Intro2 constructor receipt");

    let starts = captured
        .iter_all()
        .filter(|entity| {
            entity.active
                && entity.entity_type == INTRO2_TYPE47_ENTITY_TYPE
                && intro2_uses_live_actor_pose(entity)
        })
        .map(|entity| (entity.id, entity.position_raw()))
        .collect::<Vec<_>>();
    assert_eq!(starts.len(), INTRO2_TYPE47_SPAWN_INDICES.len());

    let mut owners = captured
        .iter_all()
        .filter_map(|entity| {
            v2k_game::intro2_type47_live::world::Intro2Type47WorldOwner::adopt(&captured, entity.id)
                .ok()
        })
        .collect::<Vec<_>>();
    assert_eq!(owners.len(), starts.len());
    let mut world_fx = v2k_game::world_fx::WorldFx::new();
    for frame in 0..90 {
        let mut next = Vec::with_capacity(owners.len());
        for owner in owners {
            let model = session.cache.global_model(INTRO2_TYPE47_MODEL_ID).unwrap();
            let tick = v2k_game::intro2_type47_live::world::tick_intro2_type47_world_owner(
                &mut captured,
                owner,
                &mut world_fx,
                Some(
                    v2k_game::intro2_type47_live::world::Intro2Type47WorldFrame {
                        type_record: session.cache.global_entity_type(47).unwrap(),
                        terrain_collision:
                            v2k_game::world_fx::TerrainCollisionContext::from_current_level_cache(
                                &session.cache,
                            )
                            .unwrap(),
                        active_model_extent_raw: model.radius,
                        waves_enabled: session.cache.level_desc().unwrap().raw_u32(0x84).unwrap()
                            != 0,
                        elapsed_micros: 19_500,
                        global_elapsed_micros: 19_500,
                        retail_tick: frame,
                    },
                ),
                &mut |_| 1,
            );
            match tick.outcome {
                v2k_game::intro2_type47_live::world::Intro2Type47WorldOutcome::Task {
                    outcome: Intro2Type47SchedulerProductionOutcome::Primary { visit, .. },
                    completed: true,
                    ..
                } => {
                    assert!(
                        matches!(
                            visit.result,
                            Intro2Type47PrimaryVisitResult::Continue
                                | Intro2Type47PrimaryVisitResult::TaggedZeroMover
                        ),
                        "Intro2 Type-47 blocked on frame {frame}: {:?}",
                        visit.result
                    );
                }
                other => panic!("expected Primary visit, got {other:?}"),
            }
            next.push(
                tick.retained_owner
                    .expect("Intro2 Type-47 owner stays live"),
            );
        }
        owners = next;
    }

    for (entity_id, start) in &starts {
        let entity = captured
            .iter_all()
            .find(|entity| entity.id == *entity_id)
            .expect("Intro2 Type-47 remains");
        let end = entity.position_raw();
        let vel = entity.velocity_raw();
        assert_ne!(
            [start[0], start[2]],
            [end[0], end[2]],
            "Intro2 Type-47 {entity_id} must walk after FUN_00412DA0, start={start:?} end={end:?} vel={vel:?} flags={:?}",
            entity.collision.state_flags_at_0x08
        );
        let RetailRuntimeValue::Known(Some(sub_h)) = &entity.sub_h_external_frame_runtime else {
            panic!("Intro2 Type-47 {entity_id} must own live Sub-H");
        };
        assert!(
            sub_h.records().iter().all(|record| record.flags_raw & 7 == 0),
            "Mover-only visits cannot publish D360 flags without selected model presentation, entity={entity_id}"
        );
    }
}
