use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    common_mover::shared_initializer_target_speed_raw,
    entity::{EntityConstructionResources, EntityManager, Intro2BirthSelection},
    entity_collision_state::RetailStateWord,
    session::GameSession,
};

fn fixture() -> Option<(GameSession, Vec<EntityTypeRuntimeMetadata>)> {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(50, 1).unwrap();
    let metadata = session
        .cache
        .global_entity_model_table()
        .iter()
        .copied()
        .enumerate()
        .map(|(id, model_slots)| {
            session
                .cache
                .global_entity_type(id)
                .map(EntityTypeRuntimeMetadata::from_section12)
                .unwrap_or(EntityTypeRuntimeMetadata {
                    model_slots,
                    ..Default::default()
                })
        })
        .collect();
    Some((session, metadata))
}

fn generic(session: &GameSession, metadata: &[EntityTypeRuntimeMetadata]) -> EntityManager {
    EntityManager::from_level_with_type_metadata(
        session.cache.level_desc().unwrap(),
        metadata,
        EntityConstructionResources {
            terrain: session.cache.terrain(),
            terrain_objects: session.cache.terrain_objects(),
            model_extent_raw: Some(&|id| session.cache.global_model(id).map(|model| model.radius)),
        },
    )
}

#[v2k_test_support::retail_test]
fn intro2_type9_native_wander_and_run_away_preserve_component_selector_suffix_order() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    assert!(exact_level_one_type9_metadata(&metadata[9]));
    for (capability, selector, class, expected_draws) in
        [(0, 0xffff, 6, 3), (8, 0, 10, 4), (8, 0xffff, 6, 3)]
    {
        let mut manager = generic(&session, &metadata);
        let mut candidates = generic(&session, &metadata);
        let owner = manager.entity_mut(3).unwrap();
        let candidate = candidates.entity_mut(1).unwrap();
        candidate.set_position_raw(owner.position_raw());
        candidate.capability_flags = capability;
        candidate.collision.state_flags_at_0x08 = RetailStateWord::exact(4);
        let mut words = [0x5300, selector, 0x1234, 0x9876].into_iter();
        let mut draws = 0;
        let result = publish_intro2_type9(
            owner,
            &metadata[9],
            std::slice::from_ref(candidate),
            session.cache.terrain().unwrap(),
            &mut || {
                draws += 1;
                words.next().unwrap()
            },
        )
        .unwrap();
        assert_eq!(result.selection.program.class_id, class);
        assert_eq!(result.selector_word, selector);
        assert_eq!(draws, expected_draws);
        assert!(intro2_type9_allocation_authenticates(owner));
        assert_eq!(
            owner.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0)
        );
        assert!(!owner.collision.fresh_level1_type9_first_scheduler_pending);
        let selected = owner.ordinary_type9_selected_component_runtime.unwrap();
        let components = selected.components();
        assert_eq!(
            components.immutable_anchor_raw_at_0x90(),
            RetailRuntimeValue::Known(owner.position_raw())
        );
        assert_eq!(
            components.sub_d_frame_owner.classifier_cache().origin(),
            RetailRuntimeValue::Unresolved
        );
        assert_eq!(
            components
                .sub_d_frame_owner
                .classifier_cache()
                .stagger_counter(),
            0x02
        );
        assert_eq!(
            components.sub_d_frame_owner.classifier_cache().rows(),
            [0; 8]
        );
        assert!(components
            .sub_d_frame_owner
            .classifier_cache()
            .can_classify());
        let RetailRuntimeValue::Known(Some(sub_a)) = owner.sub_a_propulsion_runtime else {
            panic!()
        };
        assert_eq!(
            sub_a.target_speed_raw(),
            RetailRuntimeValue::Known(shared_initializer_target_speed_raw(
                250,
                if class == 10 { 0x9876 } else { 0x1234 }
            ))
        );
        assert_eq!(owner.actor_task_state(ActorTaskSlot::Tertiary), None);
        if class == 10 {
            assert!(matches!(
                owner.actor_task_state(ActorTaskSlot::Primary),
                Some(ActorTaskRuntime::SharedRetarget(_))
            ));
            assert!(matches!(
                owner.actor_task_state(ActorTaskSlot::Secondary),
                Some(ActorTaskRuntime::TargetAcquisition(_))
            ));
        } else {
            assert!(matches!(
                owner.actor_task_state(ActorTaskSlot::Primary),
                Some(ActorTaskRuntime::OrdinaryType9Wander(_))
            ));
            assert_eq!(owner.actor_task_state(ActorTaskSlot::Secondary), None);
        }
        let before = draws;
        assert_eq!(
            publish_intro2_type9(
                owner,
                &metadata[9],
                std::slice::from_ref(candidate),
                session.cache.terrain().unwrap(),
                &mut || {
                    draws += 1;
                    0
                }
            ),
            Err(Intro2Type9Error::AlreadyPublished)
        );
        assert_eq!(draws, before);
        owner.current_behavior_context = RetailRuntimeValue::Known(None);
        owner.collision.health_raw = RetailRuntimeValue::Known(0);
        owner
            .collision
            .state_flags_at_0x08
            .overwrite(0x4000, 0x4000);
        assert!(
            intro2_type9_allocation_authenticates(owner),
            "allocation receipt survives behavior/death"
        );
        owner.id += 1;
        assert!(!intro2_type9_allocation_authenticates(owner));
    }
}

#[v2k_test_support::retail_test]
fn intro2_type9_complete_authored_prefix_keeps_each_own_seed_and_unknown_origin() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    for seed in [0u32, 1, 0x1234, 0xffff] {
        let mut state = seed;
        let manager = EntityManager::from_captured_intro2_frontend_with_type_metadata(
            session.cache.level_desc().unwrap(),
            &metadata,
            EntityConstructionResources {
                terrain: session.cache.terrain(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&|id| {
                    session.cache.global_model(id).map(|model| model.radius)
                }),
            },
            Intro2BirthSelection::default(),
            &mut || {
                state = state.wrapping_mul(214013).wrapping_add(2531011);
                (state >> 16) & 0xffff
            },
        )
        .unwrap();
        let peasants: Vec<_> = manager.iter_all().filter(|e| e.entity_type == 9).collect();
        assert_eq!(peasants.len(), 13);
        for entity in peasants {
            assert!(intro2_type9_allocation_authenticates(entity));
            let &(spawn, _, seed) = BIRTHS
                .iter()
                .find(|(spawn, _, _)| entity.authored_spawn_index == Some(*spawn))
                .unwrap();
            let components = entity
                .ordinary_type9_selected_component_runtime
                .unwrap()
                .components();
            assert_eq!(
                components.sub_d_frame_owner.classifier_cache().origin(),
                RetailRuntimeValue::Unresolved,
                "spawn{spawn}"
            );
            assert_eq!(
                components
                    .sub_d_frame_owner
                    .classifier_cache()
                    .stagger_counter(),
                seed,
                "spawn{spawn}"
            );
            assert_eq!(
                components.sub_d_frame_owner.classifier_cache().rows(),
                [0; 8]
            );
            assert!(components
                .sub_d_frame_owner
                .classifier_cache()
                .can_classify());
            assert_eq!(
                entity.collision.fresh_level1_type9_first_scheduler_pending,
                false
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn intro2_type9_rejects_wrong_birth_or_player_prefix_before_rng() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let mut candidates = generic(&session, &metadata);
    let owner = manager.entity_mut(3).unwrap();
    let candidate = candidates.entity_mut(1).unwrap();
    candidate.set_position_raw(owner.position_raw());
    candidate.capability_flags = 1;
    candidate.collision.state_flags_at_0x08 = RetailStateWord::exact(4);
    let mut draws = 0;
    assert_eq!(
        publish_intro2_type9(
            owner,
            &metadata[9],
            std::slice::from_ref(candidate),
            session.cache.terrain().unwrap(),
            &mut || {
                draws += 1;
                0
            }
        ),
        Err(Intro2Type9Error::PlayerCandidateOutsideIntro2Contract)
    );
    assert_eq!(draws, 0);
    assert_eq!(owner.intro2_type9_runtime, None);
    owner.authored_spawn_index = Some(0);
    assert_eq!(
        publish_intro2_type9(
            owner,
            &metadata[9],
            &[],
            session.cache.terrain().unwrap(),
            &mut || {
                draws += 1;
                0
            }
        ),
        Err(Intro2Type9Error::Identity)
    );
    assert_eq!(draws, 0);
}

#[v2k_test_support::retail_test]
fn intro2_type9_native_birth_transfers_once_to_shared_scheduler_in_both_modes() {
    use crate::{
        entity_scheduler::SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
        gameplay_notifications::GameplayNotifications,
        ordinary_type9_go_to_job_production::OrdinaryType9GoToJobProductionOutcome as Job,
        ordinary_type9_run_away_production::OrdinaryType9RunAwayProductionOutcome as Run,
        ordinary_type9_wander_production::OrdinaryType9WanderProductionOutcome as Wander,
        specialized_actor_task_production::{
            SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome as Outcome,
            SpecializedActorTaskScheduler,
        },
        static_damage::StaticDamageScheduler,
        world_fx::WorldFx,
    };
    for detailed in [false, true] {
        let Some((mut session, metadata)) = fixture() else {
            return;
        };
        let mut world_fx = WorldFx::new();
        let mut manager = EntityManager::from_captured_intro2_frontend_with_type_metadata(
            session.cache.level_desc().unwrap(),
            &metadata,
            EntityConstructionResources {
                terrain: session.cache.terrain(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&|id| {
                    session.cache.global_model(id).map(|model| model.radius)
                }),
            },
            Intro2BirthSelection::default(),
            &mut || u32::from(world_fx.next_shared_retail_random_u16()),
        )
        .unwrap();
        assert_eq!(manager.intro2_type13_environment(), (0, 3));
        let mut scheduler = SpecializedActorTaskScheduler::new();
        let ids: Vec<_> = manager
            .iter_all()
            .filter(|entity| entity.entity_type == 9)
            .map(|entity| entity.id)
            .collect();
        assert_eq!(ids.len(), 13);
        for &id in &ids {
            let entity = manager.entity_mut(id).unwrap();
            assert_eq!(
                entity
                    .collision
                    .state_flags_at_0x08
                    .masked(crate::entity_collision_state::SURFACE_STATE_MASK),
                RetailRuntimeValue::Unresolved,
                "the separate surface classifier has not run"
            );
            let authority = take_intro2_type9_task_authority(entity).unwrap();
            assert!(authority.authenticates_retained_entity(entity));
            assert!(take_intro2_type9_task_authority(entity).is_none());
            assert!(intro2_type9_allocation_authenticates(entity));
            entity.collision.state_flags_at_0x08.overwrite(
                SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
                if detailed {
                    SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT
                } else {
                    0
                },
            );
            // Controlled callback carry passes both scheduler gates without
            // a random wait; native task/component/heap receipts stay intact.
            entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(250_000);
            entity.collision.callback_scheduler_accumulator_us_at_0x6c =
                RetailRuntimeValue::Known(125_001);
            entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(9);
            scheduler
                .adopt_type9_current_task(&manager, authority)
                .unwrap();
        }
        let mut static_damage = StaticDamageScheduler::new();
        let mut notifications = GameplayNotifications::new();
        let pass = scheduler.tick(
            &mut manager,
            SpecializedActorTaskProductionFrame {
                world:
                    crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                hive_components: None,
                notification_phase:
                    crate::gameplay_notifications::GameplayNotificationPhase::NonGameplay,
                resources: &mut session.cache,
                world_fx: &mut world_fx,
                static_damage: &mut static_damage,
                elapsed_micros: 0,
                global_elapsed_micros: 20_000,
                retail_tick: 6,
                main_base_abort_active: false,
            },
            &mut notifications,
        );
        assert_eq!(pass.block, None);
        assert_eq!(pass.outcomes.len(), 13);
        for outcome in pass.outcomes {
            assert!(
                matches!(
                    outcome,
                    Outcome::OrdinaryType9Wander(Wander::PostBasisTailPending { .. })
                        | Outcome::OrdinaryType9RunAway(Run::PostBasisTailPending { .. })
                        | Outcome::OrdinaryType9GoToJob(Job::PostBasisTailPending { .. })
                ),
                "native first callback detailed={detailed}: {outcome:?}"
            );
        }
        for id in ids {
            let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
            assert_eq!(entity.mass_raw, 19, "callback consumes this frame's B2");
            assert_eq!(
                entity.collision.animation_offset_at_0xb2,
                RetailRuntimeValue::Known(0)
            );
            assert_eq!(
                entity
                    .collision
                    .state_flags_at_0x08
                    .masked(crate::entity_collision_state::SURFACE_STATE_MASK),
                RetailRuntimeValue::Unresolved,
                "task/world tail cannot manufacture later surface-classifier state"
            );
            assert_eq!(
                entity.collision.state_flags_at_0x08.masked(0x0100_0000),
                RetailRuntimeValue::Known(0x0100_0000),
                "no Level1 startup state overwrite"
            );
            let cache = entity
                .ordinary_type9_selected_component_runtime
                .unwrap()
                .components()
                .sub_d_frame_owner;
            assert!(
                matches!(
                    cache.classifier_cache().origin(),
                    RetailRuntimeValue::Known(_)
                ),
                "own first query consumed for entity{id}"
            );
            assert!(!entity.collision.fresh_level1_type9_first_scheduler_pending);
        }
    }
}

#[v2k_test_support::retail_test]
fn intro2_type9_changed_graph_cannot_claim_birth_authority() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let entity = manager.entity_mut(3).unwrap();
    assert!(take_intro2_type9_task_authority(entity).is_none());
    publish_intro2_type9(
        entity,
        &metadata[9],
        &[],
        session.cache.terrain().unwrap(),
        &mut || 0,
    )
    .unwrap();
    let original = entity.current_behavior_context;
    entity.current_behavior_context = RetailRuntimeValue::Known(None);
    assert!(take_intro2_type9_task_authority(entity).is_none());
    assert!(intro2_type9_allocation_authenticates(entity));
    entity.current_behavior_context = original;
    assert!(take_intro2_type9_task_authority(entity).is_some());
    assert!(take_intro2_type9_task_authority(entity).is_none());
}
