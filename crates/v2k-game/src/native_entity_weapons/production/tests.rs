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

#[v2k_test_support::retail_test]
fn mine_falls_arms_its_search_in_the_same_pass_floats_and_dies_on_a_target() {
    let (mut session, mut manager, mut fx, mut owner) = weapon(EntityWeaponKind::ProximityMine);
    let id = owner.entity_id();
    let terrain = session.cache.level_terrain().unwrap().clone();
    let floor = |position: [i16; 3]| {
        let cell = terrain
            .cell(
                usize::from((position[0] as u16) >> 8),
                usize::from((position[2] as u16) >> 8),
            )
            .unwrap();
        (i16::from(cell.height as i8) * 32).max(terrain.sea_level_raw())
    };
    let mut tick = 0;
    let armed_at = loop {
        tick += 1;
        assert!(tick < 3_000, "the mine never armed");
        let entry = manager.entity_mut(id).unwrap().position_raw();
        let result = tick_entity_weapon(
            &mut manager,
            owner,
            EntityWeaponTickFrame {
                resources: &mut session.cache,
                world_fx: &mut fx,
                elapsed_micros: 20_000,
                retail_tick: tick,
            },
            |_, _, _, _| -> Result<(), ()> { panic!("a falling mine has no death") },
        )
        .unwrap();
        let EntityWeaponTickOutcome::Advanced { restyled, .. } = result else {
            panic!("{result:?}")
        };
        if let Some(armed) = restyled {
            // 406BA0 tags once within 300 of the higher of ground and sea.
            assert!(i32::from(entry[1]) - i32::from(floor(entry)) <= 299);
            owner = armed;
            break tick;
        }
        assert!(i32::from(entry[1]) - i32::from(floor(entry)) > 299);
    };
    assert!(owner.authenticates(&manager));
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(
        entity.actor_task_state(ActorTaskSlot::Primary),
        Some(&ActorTaskRuntime::ProximityMine(MinePrimary::Armed))
    );
    assert!(matches!(
        entity.actor_task_state(ActorTaskSlot::Secondary),
        Some(ActorTaskRuntime::GuardLocationAcquisition(_))
    ));
    assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        panic!("context")
    };
    assert_eq!(
        context.active_style().style_address(),
        mine::MINE_ARMED_STYLE_ADDRESS
    );
    // Falling keeps variable 2 at zero; variable 1 ran at dt/4 a visit.
    let runtime = entity.native_entity_weapon_runtime.unwrap();
    assert_eq!(runtime.sub_k_raw[1], 0);
    assert_eq!(runtime.sub_k_raw[0], (armed_at * 5_000) as u16);
    // Nothing with capability 8 is near: it floats on the lift.
    for _ in 0..500 {
        tick += 1;
        let result = tick_entity_weapon(
            &mut manager,
            owner,
            EntityWeaponTickFrame {
                resources: &mut session.cache,
                world_fx: &mut fx,
                elapsed_micros: 20_000,
                retail_tick: tick,
            },
            |_, _, _, _| -> Result<(), ()> { panic!("no target in reach") },
        )
        .unwrap();
        assert!(matches!(
            result,
            EntityWeaponTickOutcome::Advanced { restyled: None, .. }
        ));
    }
    let entity = manager.entity_mut(id).unwrap();
    let rest = entity.position_raw();
    let ground = terrain.bilinear_height_raw(rest[0], rest[2]);
    assert!(
        rest[1] > ground.max(terrain.sea_level_raw() - 200) && rest[1] < floor(rest) + 300,
        "floats near the surface: {rest:?}"
    );
    assert_eq!(entity.rotation_heading_pitch_roll_raw()[1].abs() < 4, true);
    // Variable 2 has eased up to 0x4000.
    assert!(entity.native_entity_weapon_runtime.unwrap().sub_k_raw[1] > 0x3f00);
    // A live capability-8 body beside it: the one-in-four search finds it
    // and the armed style's +04 (40CEF0) dies inside the Secondary visit.
    let target = manager
        .iter_all()
        .find(|entity| {
            entity.id != id
                && entity.capability_flags & 8 != 0
                && entity.collision.state_flags_at_0x08.masked(0xd000)
                    == RetailRuntimeValue::Known(0x8000)
        })
        .map(|entity| entity.id)
        .expect("a capability-8 body in world 17");
    manager.entity_mut(target).unwrap().set_position_raw([
        rest[0].wrapping_add(100),
        rest[1],
        rest[2],
    ]);
    let mut deaths = 0;
    for _ in 0..64 {
        tick += 1;
        let result = tick_entity_weapon(
            &mut manager,
            owner,
            EntityWeaponTickFrame {
                resources: &mut session.cache,
                world_fx: &mut fx,
                elapsed_micros: 20_000,
                retail_tick: tick,
            },
            |manager, _, _, dead| -> Result<(), ()> {
                assert_eq!(dead, id);
                deaths += 1;
                let entity = manager.entity_mut(dead).unwrap();
                for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
                    entity.actor_tasks.clear_slot(slot);
                }
                Ok(())
            },
        )
        .unwrap();
        if result == EntityWeaponTickOutcome::Terminal {
            break;
        }
    }
    assert_eq!(deaths, 1);
}

#[v2k_test_support::retail_test]
fn guided_missile_rolls_climbs_takes_a_target_ahead_and_dies_after_15_s() {
    let (mut session, mut manager, mut fx, owner) = weapon(EntityWeaponKind::GuidedMissile);
    let id = owner.entity_id();
    let primary = owner.task_ids[ActorTaskSlot::Primary as usize].unwrap();
    let tick = |manager: &mut EntityManager,
                session: &mut GameSession,
                fx: &mut WorldFx,
                retail_tick: u32| {
        tick_entity_weapon(
            manager,
            owner,
            EntityWeaponTickFrame {
                resources: &mut session.cache,
                world_fx: fx,
                elapsed_micros: 20_000,
                retail_tick,
            },
            |manager, _, _, dead| -> Result<(), ()> {
                let entity = manager.entity_mut(dead).unwrap();
                for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
                    entity.actor_tasks.clear_slot(slot);
                }
                Ok(())
            },
        )
        .unwrap()
    };
    let state = |manager: &mut EntityManager| {
        let Some(ActorTaskRuntime::GuidedMissile(state)) = manager
            .entity_mut(id)
            .unwrap()
            .actor_task_state(ActorTaskSlot::Primary)
            .copied()
        else {
            panic!("class28 Primary")
        };
        state
    };
    // 408610: the birth speed is the length of [100, 200, 300].
    assert_eq!(state(&mut manager).speed_raw, 374);
    let before = manager.entity_mut(id).unwrap().physical_body_basis_q31();
    assert!(matches!(
        tick(&mut manager, &mut session, &mut fx, 1),
        EntityWeaponTickOutcome::Advanced { .. }
    ));
    let entity = manager.entity_mut(id).unwrap();
    // The matrix rolled (457DD0) and the Euler words stayed: class28's +34
    // keeps 13F70 from rebuilding it.
    assert_ne!(entity.physical_body_basis_q31(), before);
    assert_eq!(entity.rotation_heading_pitch_roll_raw(), [0; 3]);
    let after = state(&mut manager);
    assert_eq!(after.target, None);
    assert_eq!(after.speed_raw, 374 + 76);
    // A live capability-8 body ahead along the missile's forward axis.
    let RetailRuntimeValue::Known(basis) =
        manager.entity_mut(id).unwrap().physical_body_basis_q31()
    else {
        panic!("matrix")
    };
    let position = manager.entity_mut(id).unwrap().position_raw();
    let ahead: [i16; 3] = std::array::from_fn(|axis| {
        position[axis].wrapping_add(((i64::from(basis.forward[axis]) * 2000) >> 31) as i16)
    });
    let target = manager
        .iter_all()
        .find(|entity| {
            entity.id != id
                && entity.capability_flags & 8 != 0
                && entity.collision.state_flags_at_0x08.masked(0xc000)
                    == RetailRuntimeValue::Known(0x8000)
        })
        .map(|entity| entity.id)
        .expect("a capability-8 body in world 17");
    manager.entity_mut(target).unwrap().set_position_raw(ahead);
    tick(&mut manager, &mut session, &mut fx, 2);
    assert_eq!(state(&mut manager).target, Some(target));
    tick(&mut manager, &mut session, &mut fx, 3);
    assert_eq!(state(&mut manager).flight_us, 20_000);
    // The shooter relation's age passes 15 s inside this visit's prefix.
    manager
        .entity_mut(id)
        .unwrap()
        .collision
        .recent_relation_elapsed_us_at_0x68 = RetailRuntimeValue::Known(15_000_000);
    assert_eq!(
        tick(&mut manager, &mut session, &mut fx, 4),
        EntityWeaponTickOutcome::Terminal
    );
    assert!(manager
        .entity_mut(id)
        .unwrap()
        .actor_tasks
        .wrapper_flags(primary)
        .is_none());
}
