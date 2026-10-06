use super::*;
use crate::{
    intro2_gun_turret::impact::ordinary_tests::World,
    main_base_abort_world_effects::{
        snapshot_main_base_abort_terrain_geometry, MainBaseAbortClass49Frame,
    },
};

#[v2k_test_support::retail_test]
fn ordinary_type97_abort_runs_class49_with_real_world_custody_and_no_replay() {
    for level in [31, 42, 46, 47] {
        let mut f = World::new(level);
        let geometry = snapshot_main_base_abort_terrain_geometry(&f.session.cache).unwrap();
        let mut effects =
            MainBaseAbortWorldEffects::new(&geometry, &mut f.session.cache, &mut f.static_damage)
                .unwrap();
        let mut publications = MainBaseAbortPublicationCounts::default();
        for repeat in [false, true] {
            let mut expected = f.fx.fork_for_main_base_abort_transaction();
            let actor = f.manager.main_base_abort_actor_observation(f.id).unwrap();
            let result = dispatch_native_class49(
                actor,
                &mut effects,
                MainBaseAbortClass49Frame {
                    extra_lives: crate::entity_collision_state::RetailRuntimeValue::Unresolved,
                    entities: &mut f.manager,
                    world_fx: &mut f.fx,
                    notifications: &mut f.notifications,
                    retail_tick: 600,
                    scheduler: &mut f.scheduler,
                    player_hull: &mut f.player_hull,
                },
                &mut publications,
            )
            .unwrap();
            let result = match result {
                Ok(result) => result,
                Err(error) => panic!("{:?}", error.callback_error),
            };
            assert_eq!(
                result.disposition,
                MainBaseAbortActorDisposition::Class49Death
            );
            assert!(crate::class49_death::finished_terminal_hit_authenticates(
                &f.manager, f.id
            ));
            assert_eq!(publications.appended_type60_actors, 1);
            if repeat {
                assert_eq!(
                    f.fx.next_shared_retail_random_u16(),
                    expected.next_shared_retail_random_u16()
                );
            }
        }
    }
}

#[v2k_test_support::retail_test]
fn ordinary_type97_abort_rejects_foreign_receipt_before_any_callback_write() {
    let mut f = World::new(31);
    let mut other = World::new(31);
    std::mem::swap(
        f.manager.entity_mut(f.id).unwrap(),
        other.manager.entity_mut(other.id).unwrap(),
    );
    let before = f.manager.entity_mut(f.id).unwrap().collision.clone();
    let mut expected = f.fx.fork_for_main_base_abort_transaction();
    let geometry = snapshot_main_base_abort_terrain_geometry(&f.session.cache).unwrap();
    let mut effects =
        MainBaseAbortWorldEffects::new(&geometry, &mut f.session.cache, &mut f.static_damage)
            .unwrap();
    let actor = f.manager.main_base_abort_actor_observation(f.id).unwrap();
    let mut publications = MainBaseAbortPublicationCounts::default();
    let result = dispatch_native_class49(
        actor,
        &mut effects,
        MainBaseAbortClass49Frame {
            extra_lives: crate::entity_collision_state::RetailRuntimeValue::Unresolved,
            entities: &mut f.manager,
            world_fx: &mut f.fx,
            notifications: &mut f.notifications,
            retail_tick: 600,
            scheduler: &mut f.scheduler,
            player_hull: &mut f.player_hull,
        },
        &mut publications,
    )
    .unwrap();
    let Err(error) = result else {
        panic!("foreign receipt unexpectedly admitted")
    };
    assert_eq!(
        error.callback_error,
        Some(MainBaseAbortActorCallbackBlock::Native(
            NativeMainBaseAbortDeathBlock::ActorLeaseMismatch
        ))
    );
    assert_eq!(publications, MainBaseAbortPublicationCounts::default());
    assert_eq!(f.manager.entity_mut(f.id).unwrap().collision, before);
    assert_eq!(
        f.fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
}
