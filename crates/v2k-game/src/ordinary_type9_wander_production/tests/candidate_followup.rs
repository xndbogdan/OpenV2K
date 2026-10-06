use super::*;

#[test]
fn later_candidate_completion_retains_the_new_route_or_exact_fallback() {
    for target_allocation in [
        OrdinaryType9AttractAttentionTargetAllocationDecision::Prepared,
        OrdinaryType9AttractAttentionTargetAllocationDecision::Failed,
    ] {
        let (mut manager, owner, entity_id) = production_owner_fixture();
        prepare_root_go_to_job_target(&mut manager, entity_id);
        append_root_player(&mut manager, entity_id);
        append_run_away_candidate(&mut manager, entity_id, RUN_AWAY_TARGET_ID, false);
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        // The odd graph includes Candidate, whose first gate deliberately
        // waits. Acceptance must happen through the later-frame handler.
        let mut words = VecDeque::from([0x1111_1111_u32, 3_063, 1, 0x3333, 0x4444, 0x5555, 1]);
        let first = tick_ordinary_type9_wander_owner_with_random_and_all_allocators(
            &mut manager,
            owner,
            root_frame(&resources),
            &mut world_fx,
            |_| {
                words
                    .pop_front()
                    .expect("exact odd graph and rejected gate")
            },
            |_| panic!("class 45"),
            |_| panic!("class 45"),
            |_| panic!("class 45"),
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            |_| panic!("the first Candidate gate must wait"),
        );
        assert!(words.is_empty());
        assert!(matches!(
            first.outcome,
            OrdinaryType9WanderProductionOutcome::RootAttractAttentionPublished { .. }
        ));
        let owner = first.retained_owner.unwrap();
        assert!(matches!(
            owner.outer_tail,
            Some(OrdinaryType9OuterTailCustody::Complete { .. })
        ));
        let old_primary = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .unwrap()
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        let expected_words = if target_allocation
            == OrdinaryType9AttractAttentionTargetAllocationDecision::Prepared
        {
            vec![0x1111, 0x2222, 0, 0x6666]
        } else {
            vec![0x1111, 0x2222, 0]
        };
        let mut words = VecDeque::from(expected_words);
        let mut allocations = 0;
        let next = tick_ordinary_type9_wander_owner_with_random_and_all_allocators(
            &mut manager,
            owner,
            OrdinaryType9WanderProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: 1,
            },
            &mut world_fx,
            |_| {
                words
                    .pop_front()
                    .expect("old Primary, Candidate gate, target suffix")
            },
            |_| panic!("Candidate does not select a root"),
            |_| panic!("Candidate does not select a root"),
            |_| panic!("Candidate does not select a root"),
            |_| panic!("Candidate does not select a root"),
            |_| {
                allocations += 1;
                target_allocation
            },
        );
        assert_eq!(allocations, 1);
        assert!(words.is_empty());
        let retained = next
            .retained_owner
            .expect("typed completion retains custody");
        assert!(matches!(
            retained.outer_tail,
            Some(OrdinaryType9OuterTailCustody::Complete { .. })
        ));
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .unwrap();
        assert!(entity.actor_tasks.task_state(old_primary).is_none());
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
        match target_allocation {
            OrdinaryType9AttractAttentionTargetAllocationDecision::Prepared => {
                assert!(matches!(
                    next.outcome,
                    OrdinaryType9WanderProductionOutcome::RootAttractAttentionTargetRoutePublished {
                        target_id: ROOT_PLAYER_ID, ..
                    }
                ));
                assert!(
                    matches!(
                        entity.actor_task_state(ActorTaskSlot::Primary),
                        Some(ActorTaskRuntime::AttractAttentionTargetRoute(route))
                            if route.elapsed_ms() == 0
                    ),
                    "replacement Primary waits until the next actor visit"
                );
                assert!(retained
                    .root_attract_attention_target_route
                    .as_ref()
                    .is_some_and(|route| route.validate(entity).is_ok()));
                assert!(retained.root_publication.is_none());
            }
            OrdinaryType9AttractAttentionTargetAllocationDecision::Failed => {
                assert!(matches!(next.outcome,
                    OrdinaryType9WanderProductionOutcome::RootAttractAttentionTargetInitializerFallbackPublished { .. }));
                assert!(entity.actor_task_state(ActorTaskSlot::Primary).is_none());
                assert!(retained
                    .root_publication
                    .as_ref()
                    .is_some_and(|publication| publication.authenticates(entity)));
            }
        }
    }
}
