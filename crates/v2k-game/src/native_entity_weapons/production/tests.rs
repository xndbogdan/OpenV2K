use super::*;
use crate::{
    entity::{AuthoredPlayerArrival, AuthoredWorldConstruction, EntityConstructionResources},
    entity_collision_state::EntityTypeRuntimeMetadata,
    session::GameSession,
};

fn weapon(
    kind: EntityWeaponKind,
) -> (GameSession, EntityManager, WorldFx, NativeEntityWeaponOwner) {
    let data = v2k_test_support::retail_dir();
    assert!(
        data.join("PRELOAD.DAT").is_file(),
        "normal-tier corpus required"
    );
    let mut session = GameSession::init(&data).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(17, 1).unwrap();
    let metadata: Vec<_> = session
        .cache
        .global_entity_model_table()
        .iter()
        .enumerate()
        .map(|(id, _)| {
            EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(id).unwrap())
        })
        .collect();
    let mut fx = WorldFx::new();
    let mut manager = EntityManager::from_authored_world(
        AuthoredWorldConstruction {
            level: session.cache.level_desc().unwrap(),
            logical_world_index: 5,
            type_metadata: &metadata,
            resources: EntityConstructionResources {
                terrain: session.cache.level_terrain(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&|id| {
                    session.cache.global_model(id).map(|model| model.radius)
                }),
            },
            player_arrival: Some(AuthoredPlayerArrival {
                position_raw: [1000, 1024, 2000],
                heading_raw: 0,
            }),
            retail_tick: 0,
        },
        &mut fx,
    )
    .unwrap();
    let owner = manager
        .construct_entity_weapon(
            EntityWeaponConstructionRequest {
                kind,
                source_actor_id: manager.player().unwrap().id,
                position_raw: [0, 10_000, 0],
                velocity_raw: [100, 200, 300],
                rotation_raw: [0; 3],
            },
            &session.cache,
            &mut fx,
            0,
        )
        .unwrap();
    manager
        .entity_mut(owner.entity_id())
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(u32::MAX, 0x0206_0005);
    (session, manager, fx, owner)
}

#[v2k_test_support::retail_test]
fn grenade_stationary_tag_dispatches_once_after_wrapper_unwind() {
    let (mut session, mut manager, mut fx, owner) = weapon(EntityWeaponKind::Grenade);
    let primary = owner.task_ids[ActorTaskSlot::Primary as usize].unwrap();
    let mut calls = 0;
    for visit in 1..=11 {
        let result = tick_entity_weapon(
            &mut manager,
            owner,
            EntityWeaponTickFrame {
                resources: &mut session.cache,
                world_fx: &mut fx,
                elapsed_micros: 0,
                retail_tick: visit,
            },
            |manager, _, _, id| -> Result<(), ()> {
                let entity = manager.entity_mut(id).unwrap();
                assert!(
                    !entity
                        .actor_tasks
                        .wrapper_flags(primary)
                        .unwrap()
                        .in_callback
                );
                calls += 1;
                entity.actor_tasks.clear_slot(ActorTaskSlot::Primary);
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(
            matches!(result, EntityWeaponTickOutcome::Terminal),
            visit == 11
        );
    }
    assert_eq!(calls, 1);
    assert!(manager
        .entity_mut(owner.entity_id())
        .unwrap()
        .actor_tasks
        .wrapper_flags(primary)
        .is_none());
}

#[v2k_test_support::retail_test]
fn failed_after_unwind_timeout_preserves_elapsed_but_leaves_no_callback_loan() {
    let (mut session, mut manager, mut fx, owner) = weapon(EntityWeaponKind::Grenade);
    let primary = owner.task_ids[ActorTaskSlot::Primary as usize].unwrap();
    for visit in 1..=17 {
        // Keep the prior-position branch displaced so timeout owns this test.
        manager
            .entity_mut(owner.entity_id())
            .unwrap()
            .set_position_raw([visit as i16, 10_000, 0]);
        let result = tick_entity_weapon(
            &mut manager,
            owner,
            EntityWeaponTickFrame {
                resources: &mut session.cache,
                world_fx: &mut fx,
                elapsed_micros: 125_000,
                retail_tick: visit,
            },
            |manager, _, _, id| -> Result<(), &'static str> {
                assert!(
                    !manager
                        .entity_mut(id)
                        .unwrap()
                        .actor_tasks
                        .wrapper_flags(primary)
                        .unwrap()
                        .in_callback
                );
                Err("committed terminal boundary")
            },
        );
        if visit == 17 {
            assert_eq!(
                result,
                Err(EntityWeaponTickError::Terminal(
                    "committed terminal boundary"
                ))
            );
        } else {
            assert!(matches!(
                result,
                Ok(EntityWeaponTickOutcome::Advanced { .. })
            ));
        }
    }
    let entity = manager.entity_mut(owner.entity_id()).unwrap();
    assert!(
        !entity
            .actor_tasks
            .wrapper_flags(primary)
            .unwrap()
            .in_callback
    );
    let Some(ActorTaskRuntime::BoulderRolling(state)) =
        entity.actor_task_state(ActorTaskSlot::Primary)
    else {
        panic!("elapsed state remains owned")
    };
    assert_eq!(state.elapsed_ms, 2125);
}

#[v2k_test_support::retail_test]
fn callback_validation_failure_unwinds_the_exact_rocket_wrapper() {
    let (mut session, mut manager, mut fx, owner) = weapon(EntityWeaponKind::Rocket);
    let primary = owner.task_ids[ActorTaskSlot::Primary as usize].unwrap();
    manager
        .entity_mut(owner.entity_id())
        .unwrap()
        .sub_a_propulsion_runtime = RetailRuntimeValue::Unresolved;
    let result = tick_entity_weapon(
        &mut manager,
        owner,
        EntityWeaponTickFrame {
            resources: &mut session.cache,
            world_fx: &mut fx,
            elapsed_micros: 20_000,
            retail_tick: 1,
        },
        |_, _, _, _| -> Result<(), ()> { panic!("failed callback cannot dispatch death") },
    );
    assert_eq!(
        result,
        Err(EntityWeaponTickError::Owned {
            block: EntityWeaponBlock::Runtime("SubA runtime"),
            committed_prefix: true
        })
    );
    assert!(
        !manager
            .entity_mut(owner.entity_id())
            .unwrap()
            .actor_tasks
            .wrapper_flags(primary)
            .unwrap()
            .in_callback
    );
}
