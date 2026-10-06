//! Real-data admission and prefix composition for Main Base abort.

use std::path::Path;

use v2k_game::{
    entity::{
        EntityManager, MainBaseAbortAlternateCleanupAdvance, MainBaseAbortAlternateCleanupEffects,
        MainBaseAbortAlternateCleanupOutcome,
    },
    entity_collision_state::EntityTypeRuntimeMetadata,
    main_base_abort::{
        MainBaseAbortAction, MainBaseAbortActorRoute, MainBaseAbortMachine, MainBaseAbortPhase,
        MainBaseAbortPoll, MainBaseAbortQuietDeathAdvance, MainBaseAbortQuietDeathOutcome,
        MainBaseAbortResume, MainBaseAbortTransactionId, MainBaseAbortWorldControlLease,
    },
    session::GameSession,
    world_fx::WorldFx,
};

fn fresh_level_one_manager(dir: &Path) -> EntityManager {
    let mut session = GameSession::init(dir).expect("init session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-tier first-world resources");
    let type_metadata = session
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
        .collect::<Vec<_>>();
    session
        .load_level_by_id(13, 1)
        .expect("load normal-tier Level 1");

    EntityManager::from_fresh_new_game_level_with_type_metadata(
        session.cache.level_desc().expect("Level-1 Section 13"),
        &type_metadata,
        session.cache.terrain(),
        0,
        &mut WorldFx::new(),
    )
    .expect("fresh first-world behavior publication")
}

struct RejectTerrainPublication;

impl MainBaseAbortAlternateCleanupEffects for RejectTerrainPublication {
    fn clear_and_publish_old_terrain_attribute(&mut self, cell: [u8; 2]) {
        panic!("the player's exact no-Sub-N route cannot publish cell {cell:?}")
    }
}

#[v2k_test_support::retail_test]
fn fresh_retail_constructor_admits_all_fifteen_quiet_death_actors() {
    let dir = v2k_test_support::retail_dir();

    let mut manager = fresh_level_one_manager(&dir);
    let cohort = manager
        .iter_all()
        .filter(|entity| [52, 62, 68].contains(&entity.entity_type))
        .map(|entity| {
            (
                entity.id,
                entity.authored_spawn_index.expect("authored Level-1 spawn"),
                entity.entity_type,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        cohort
            .iter()
            .map(|(_, spawn_index, entity_type)| (*spawn_index, *entity_type))
            .collect::<Vec<_>>(),
        [
            (0, 52),
            (1, 52),
            (2, 52),
            (3, 52),
            (4, 62),
            (5, 68),
            (7, 52),
            (21, 52),
            (25, 62),
            (26, 62),
            (27, 52),
            (28, 52),
            (29, 52),
            (30, 52),
            (31, 52),
        ]
    );

    let mut abort_fx = WorldFx::new();
    for &(entity_id, _, entity_type) in &cohort {
        let observation = manager
            .main_base_abort_actor_observation(entity_id)
            .expect("live quiet-death actor");
        let result = manager
            .apply_main_base_abort_quiet_death(observation.lease, &mut abort_fx)
            .expect("real constructor published an admissible class-2 predecessor");
        assert!(matches!(
            result,
            MainBaseAbortQuietDeathAdvance::Advanced {
                outcome: MainBaseAbortQuietDeathOutcome::DeferredDestroyStaged {
                    entity_id: actual_id,
                    entity_type: actual_type,
                    ..
                },
                ..
            } if actual_id == entity_id && actual_type == entity_type
        ));
    }

    let expected_ids = cohort
        .iter()
        .map(|(entity_id, _, _)| *entity_id)
        .collect::<Vec<_>>();
    assert_eq!(manager.pending_actor_deferred_destroy_ids(), expected_ids);
    assert_eq!(abort_fx.pending_event_count(), 1, "only type 68 has a cue");
    assert_eq!(
        manager.cleanup_pending_actor_deferred_destroys(),
        expected_ids
    );
}

/// Prove the prefix represented by the port's live manager. Retail's earlier
/// type-1/type-0 and interleaved type-111 system allocations are not currently
/// materialized here, so this is deliberately not the captured 39-node census
/// or a unified transaction with the separate spawn-6-starting suffix proof.
#[v2k_test_support::retail_test]
fn port_live_player_and_spawns_0_through_5_compose_to_spawn_6() {
    let dir = v2k_test_support::retail_dir();

    let mut manager = fresh_level_one_manager(&dir);
    assert_eq!(
        manager
            .iter_all()
            .take(8)
            .map(|entity| (entity.id, entity.authored_spawn_index, entity.entity_type))
            .collect::<Vec<_>>(),
        [
            (1, None, 46),
            (2, Some(0), 52),
            (3, Some(1), 52),
            (4, Some(2), 52),
            (5, Some(3), 52),
            (6, Some(4), 62),
            (7, Some(5), 68),
            (8, Some(6), 6),
        ]
    );

    let first_actor = manager
        .first_main_base_abort_actor_observation()
        .expect("fresh Level 1 begins with the persistent player");
    let mut machine = MainBaseAbortMachine::start(
        MainBaseAbortTransactionId::new(1).unwrap(),
        v2k_game::entity_collision_state::RetailRuntimeValue::Known(Some(
            MainBaseAbortWorldControlLease {
                allocation_identity: 1,
            },
        )),
        v2k_game::entity_collision_state::RetailRuntimeValue::Known(Some(first_actor)),
    );

    let issued = match machine.poll() {
        MainBaseAbortPoll::Action(issued) => issued,
        other => panic!("expected player action, got {other:?}"),
    };
    let MainBaseAbortAction::ProcessActor { actor, route, .. } = issued.action else {
        panic!("expected player actor action")
    };
    assert_eq!((actor.lease.entity_id, actor.entity_type), (1, 46));
    assert_eq!(
        route,
        MainBaseAbortActorRoute::AlternateCleanup { entity_id: 1 }
    );
    let MainBaseAbortAlternateCleanupAdvance::Advanced {
        outcome,
        next_actor,
    } = manager
        .apply_main_base_abort_alternate_cleanup(actor.lease, &mut RejectTerrainPublication)
        .expect("retail player metadata proves the no-Sub-N alternate route")
    else {
        panic!("the persistent player remains linked")
    };
    assert_eq!(
        outcome,
        MainBaseAbortAlternateCleanupOutcome::ExactNoSubN {
            entity_id: 1,
            entity_type: 46,
        }
    );
    assert_eq!(next_actor.map(|next| next.lease.entity_id), Some(2));
    machine
        .resume(
            issued.receipt,
            MainBaseAbortResume::ActorProcessed {
                phase: MainBaseAbortPhase::ActorSweep,
                actor: actor.lease,
                successor: v2k_game::entity_collision_state::RetailRuntimeValue::Known(next_actor),
            },
        )
        .unwrap();

    let mut abort_fx = WorldFx::new();
    for (entity_id, spawn_index, entity_type, death_sound_id) in [
        (2_u32, 0_usize, 52_u32, None),
        (3, 1, 52, None),
        (4, 2, 52, None),
        (5, 3, 52, None),
        (6, 4, 62, None),
        (7, 5, 68, Some(62_u16)),
    ] {
        let issued = match machine.poll() {
            MainBaseAbortPoll::Action(issued) => issued,
            other => panic!("expected spawn-{spawn_index} actor action, got {other:?}"),
        };
        let MainBaseAbortAction::ProcessActor { actor, route, .. } = issued.action else {
            panic!("expected spawn-{spawn_index} actor action")
        };
        assert_eq!(
            (actor.lease.entity_id, actor.entity_type),
            (entity_id, entity_type)
        );
        assert_eq!(route, MainBaseAbortActorRoute::OrdinaryDeath { entity_id });
        assert_eq!(
            manager
                .iter_all()
                .find(|entity| entity.id == entity_id)
                .and_then(|entity| entity.authored_spawn_index),
            Some(spawn_index)
        );

        let MainBaseAbortQuietDeathAdvance::Advanced {
            outcome,
            next_actor,
        } = manager
            .apply_main_base_abort_quiet_death(actor.lease, &mut abort_fx)
            .expect("real constructor published an admissible class-2 predecessor")
        else {
            panic!("spawn-{spawn_index} remains linked")
        };
        assert_eq!(
            outcome,
            MainBaseAbortQuietDeathOutcome::DeferredDestroyStaged {
                entity_id,
                entity_type,
                death_sound_id,
            }
        );
        assert_eq!(
            next_actor.map(|next| next.lease.entity_id),
            Some(entity_id + 1)
        );
        machine
            .resume(
                issued.receipt,
                MainBaseAbortResume::ActorProcessed {
                    phase: MainBaseAbortPhase::ActorSweep,
                    actor: actor.lease,
                    successor: v2k_game::entity_collision_state::RetailRuntimeValue::Known(
                        next_actor,
                    ),
                },
            )
            .unwrap();
    }

    let issued = match machine.poll() {
        MainBaseAbortPoll::Action(issued) => issued,
        other => panic!("expected spawn-6 Main Base action, got {other:?}"),
    };
    let MainBaseAbortAction::ProcessActor { actor, route, .. } = issued.action else {
        panic!("expected spawn-6 actor action")
    };
    assert_eq!((actor.lease.entity_id, actor.entity_type), (8, 6));
    assert_eq!(
        route,
        MainBaseAbortActorRoute::OrdinaryDeath { entity_id: 8 }
    );
    assert_eq!(
        manager
            .iter_all()
            .find(|entity| entity.id == 8)
            .and_then(|entity| entity.authored_spawn_index),
        Some(6)
    );
    assert_eq!(
        manager.pending_actor_deferred_destroy_ids(),
        &[2, 3, 4, 5, 6, 7]
    );
    assert_eq!(abort_fx.pending_event_count(), 1);
    abort_fx.process_pending();
    assert_eq!(
        abort_fx
            .take_positional_sounds()
            .into_iter()
            .map(|sound| sound.sound_id)
            .collect::<Vec<_>>(),
        [62]
    );
}
