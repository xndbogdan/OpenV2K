use v2k_game::{
    actor_task_dispatcher::ActorTaskRuntimeFamily,
    actor_task_owner::ActorTaskSlot,
    common_mover::sub_d::{type17_seed_for_fresh_level1_spawn, FRESH_LEVEL1_TYPE17_SUB_D_SEEDS},
    entity::{EntityConstructionResources, EntityManager},
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    session::GameSession,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    type17_follow_beacons_live::Type17FollowBeaconsAcquisitionOutcome,
    type17_follow_beacons_mover::Type17FollowBeaconsMoverBlock,
    type17_follow_beacons_production::{
        tick_type17_follow_beacons_scheduler_owner, Type17FollowBeaconsFollowingVisitResult,
        Type17FollowBeaconsProductionFrame, Type17FollowBeaconsSchedulerOwner,
        Type17FollowBeaconsSchedulerProductionOutcome,
    },
    world_fx::WorldFx,
};

use v2k_game::static_damage::StaticDamageScheduler;

fn type_metadata(session: &GameSession) -> Vec<EntityTypeRuntimeMetadata> {
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

#[v2k_test_support::retail_test]
fn fresh_level1_adopts_spawn18_follow_beacons_and_ticks_03ce0_to_sub_d() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("init session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-tier first-world resources");
    session
        .load_level_by_id(13, 1)
        .expect("load normal-tier Level 1");

    let type_metadata = type_metadata(&session);
    let resources =
        EntityConstructionResources::new(session.cache.terrain(), session.cache.terrain_objects());
    let mut world_fx = WorldFx::new();
    let mut manager = EntityManager::from_fresh_new_game_level_with_type_metadata(
        session.cache.level_desc().expect("Level-1 Section 13"),
        &type_metadata,
        resources,
        0,
        &mut world_fx,
    )
    .expect("fresh type-17 birth publication");

    let follow = manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(18))
        .expect("spawn 18");
    assert_eq!(
        follow
            .actor_task_state(ActorTaskSlot::Secondary)
            .map(|task| task.family()),
        Some(ActorTaskRuntimeFamily::FollowBeaconAcquisition)
    );
    assert_eq!(FRESH_LEVEL1_TYPE17_SUB_D_SEEDS, [0x31, 0x32, 0x33, 0x34]);
    for spawn_index in [17, 18, 19, 20] {
        let entity = manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(spawn_index))
            .expect("fresh Level-1 type-17 spawn");
        let seed = type17_seed_for_fresh_level1_spawn(spawn_index).expect("spawn 17-20");
        let owner = entity
            .type17_sub_d_frame_owner
            .expect("V200002 first-query owner");
        assert_eq!(owner.classifier_cache().stagger_counter(), seed);
        assert!(owner.classifier_cache().can_classify());
        assert_eq!(
            owner.classifier_cache().origin(),
            RetailRuntimeValue::Unresolved
        );
        assert!(entity.type17_sub_d_runtime.is_some());
        assert!(entity.type8_sub_d_frame_owner.is_none());
    }
    for entity in manager.iter_all().filter(|entity| entity.entity_type == 47) {
        assert!(entity.type17_sub_d_frame_owner.is_none());
        assert!(entity.type8_sub_d_frame_owner.is_none());
    }

    let mut scheduler = SpecializedActorTaskScheduler::new();
    assert_eq!(
        scheduler.adopt_fresh_level1_type17_follow_beacons(&manager),
        1
    );

    let metadata = type_metadata[17].clone();
    let owner = Type17FollowBeaconsSchedulerOwner::adopt_published(follow, &metadata)
        .expect("published Follow Beacons graph");
    let mut static_damage = StaticDamageScheduler::new();
    let tick = tick_type17_follow_beacons_scheduler_owner(
        &mut manager,
        owner,
        Type17FollowBeaconsProductionFrame {
            resources: &mut session.cache,
            static_damage: &mut static_damage,
            world_fx: &mut world_fx,
            elapsed_micros: 19_500,
            retail_tick: 0,
        },
    );
    match tick.outcome {
        Type17FollowBeaconsSchedulerProductionOutcome::Acquiring { outcome, .. } => {
            assert!(matches!(
                outcome,
                Type17FollowBeaconsAcquisitionOutcome::NoPositiveScore { .. }
                    | Type17FollowBeaconsAcquisitionOutcome::FollowingPublished { .. }
            ));
        }
        other => panic!("expected acquiring visit, got {other:?}"),
    }
    assert!(tick.retained_owner.is_some());
    if let Some(Type17FollowBeaconsSchedulerOwner::Following(following)) = tick.retained_owner {
        let following_tick = tick_type17_follow_beacons_scheduler_owner(
            &mut manager,
            Type17FollowBeaconsSchedulerOwner::Following(following),
            Type17FollowBeaconsProductionFrame {
                resources: &mut session.cache,
                static_damage: &mut static_damage,
                world_fx: &mut world_fx,
                elapsed_micros: 19_500,
                retail_tick: 1,
            },
        );
        match following_tick.outcome {
            Type17FollowBeaconsSchedulerProductionOutcome::Following { visit, .. } => {
                assert!(
                    matches!(
                        visit.result,
                        Type17FollowBeaconsFollowingVisitResult::Continue
                            | Type17FollowBeaconsFollowingVisitResult::Tagged(_)
                            | Type17FollowBeaconsFollowingVisitResult::CommonMoverBlocked(
                                Type17FollowBeaconsMoverBlock::SubHUnavailable
                                    | Type17FollowBeaconsMoverBlock::SubH(_)
                                    | Type17FollowBeaconsMoverBlock::SubHCompletionCue
                                    | Type17FollowBeaconsMoverBlock::SubD(_)
                                    | Type17FollowBeaconsMoverBlock::Frame(_)
                            )
                    ),
                    "FUN_00403CE0 must start the mover after the live-target prelude, got {:?}",
                    visit.result
                );
                assert!(
                    !matches!(
                        visit.result,
                        Type17FollowBeaconsFollowingVisitResult::CommonMoverBlocked(
                            Type17FollowBeaconsMoverBlock::SubDFirstQueryUnavailable
                        )
                    ),
                    "spawn-18 must not fail closed as if the V200002 owner were missing"
                );
            }
            other => panic!("expected FUN_00403CE0 visit, got {other:?}"),
        }
        assert!(following_tick.retained_owner.is_some());
    }

    let direct = EntityManager::from_level_with_type_metadata(
        session.cache.level_desc().expect("Level-1 Section 13"),
        &type_metadata,
        EntityConstructionResources::new(session.cache.terrain(), session.cache.terrain_objects()),
    );
    let mut empty = SpecializedActorTaskScheduler::new();
    assert_eq!(empty.adopt_fresh_level1_type17_follow_beacons(&direct), 0);
}

#[v2k_test_support::retail_test]
fn spawn18_follow_beacons_starts_a_sub_h_stride() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("init session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-tier first-world resources");
    session
        .load_level_by_id(13, 1)
        .expect("load normal-tier Level 1");

    let type_metadata = type_metadata(&session);
    let resources =
        EntityConstructionResources::new(session.cache.terrain(), session.cache.terrain_objects());
    let mut world_fx = WorldFx::new();
    let mut manager = EntityManager::from_fresh_new_game_level_with_type_metadata(
        session.cache.level_desc().expect("Level-1 Section 13"),
        &type_metadata,
        resources,
        0,
        &mut world_fx,
    )
    .expect("fresh type-17 birth publication");

    let follow = manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(18))
        .expect("spawn 18");
    let spider_id = follow.id;
    let start = follow.position_raw();
    let metadata = type_metadata[17].clone();
    let mut owner = Type17FollowBeaconsSchedulerOwner::adopt_published(follow, &metadata)
        .expect("published Follow Beacons graph");
    let mut static_damage = StaticDamageScheduler::new();

    let mut following_frames = 0usize;
    let mut saw_stride = false;
    for frame in 0..500 {
        let tick = tick_type17_follow_beacons_scheduler_owner(
            &mut manager,
            owner,
            Type17FollowBeaconsProductionFrame {
                resources: &mut session.cache,
                static_damage: &mut static_damage,
                world_fx: &mut world_fx,
                elapsed_micros: 19_500,
                retail_tick: frame,
            },
        );
        match tick.outcome {
            Type17FollowBeaconsSchedulerProductionOutcome::Acquiring { outcome, .. } => {
                if let Type17FollowBeaconsAcquisitionOutcome::FollowingPublished { .. } = outcome {
                    following_frames = 0;
                }
            }
            Type17FollowBeaconsSchedulerProductionOutcome::Following { visit, .. } => {
                following_frames += 1;
                assert!(
                    !matches!(visit.static_contact, Some(Err(_))),
                    "static contact must complete: {:?}",
                    visit.static_contact
                );
                assert!(
                    !matches!(
                        visit.result,
                        Type17FollowBeaconsFollowingVisitResult::CommonMoverBlocked(
                            Type17FollowBeaconsMoverBlock::SubDFirstQueryUnavailable
                        )
                    ),
                    "spawn-18 must not fail closed as if the V200002 owner were missing"
                );
            }
            other => panic!("spawn 18 unexpected {other:?} on frame {frame}"),
        }
        owner = tick.retained_owner.expect("spawn 18 owner stays live");
        if manager
            .iter_all()
            .find(|entity| entity.id == spider_id)
            .and_then(|entity| match &entity.sub_h_external_frame_runtime {
                RetailRuntimeValue::Known(Some(sub_h)) => Some(sub_h),
                _ => None,
            })
            .is_some_and(|sub_h| sub_h.records().iter().any(|record| record.phase_raw != 0))
        {
            saw_stride = true;
        }
        if following_frames >= 400 && saw_stride {
            break;
        }
    }

    let entity = manager
        .iter_all()
        .find(|entity| entity.id == spider_id)
        .expect("spawn 18 remains");
    let RetailRuntimeValue::Known(Some(sub_h)) = &entity.sub_h_external_frame_runtime else {
        panic!("spawn 18 must own live Sub-H");
    };
    let end = entity.position_raw();
    let beacons: Vec<_> = manager
        .iter_all()
        .filter_map(|candidate| {
            candidate
                .authored_follow_beacon_priority_raw
                .map(|score| (candidate.id, candidate.entity_type, score))
        })
        .collect();
    assert!(
        following_frames > 0,
        "spawn 18 must hand off to Follow Beacons variant one, start={start:?} end={end:?} beacons={beacons:?}"
    );
    assert!(
        sub_h.records().iter().any(|record| record.flags_raw != 0),
        "Type-17 D360 writeback must leave constructor-zero Sub-H flags"
    );
    assert!(
        saw_stride,
        "FUN_0041D0A0 must start a spider stride once D360 publishes COPY_TARGET/WAIT_FOR_DEPS, flags={:08x?} phases={:?} following_frames={following_frames} start={start:?} end={end:?}",
        sub_h.records().iter().map(|record| record.flags_raw).collect::<Vec<_>>(),
        sub_h.records().iter().map(|record| record.phase_raw).collect::<Vec<_>>()
    );
}
