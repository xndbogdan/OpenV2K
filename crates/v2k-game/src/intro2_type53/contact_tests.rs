//! Native Type53 late-11AD0 static contact.
//!
//! Authored positions miss static geometry, so Miss custody is covered across
//! ordinary worlds 14/15 and native Intro2. Deterministic neighborhood
//! overlaps drive the Applied path with the exact 02CA0 prefix for both the
//! acquiring retarget and a real driven Chase graph, and a lethal particle
//! hit drives the Class12 null-hook path. No synthetic behavior context,
//! task or allocation receipt is used.

use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    damage::DamagePacket,
    entity::{AuthoredPlayerArrival, AuthoredWorldConstruction, EntityConstructionResources},
    gameplay_notifications::GameplayNotifications,
    intro2_common_dying::Intro2CommonDyingOwner,
    session::GameSession,
    shared_actor_impact::SharedActorImpactFrame,
    specialized_actor_task_production::{
        SpecializedActorTaskFamily, SpecializedActorTaskProductionFrame,
        SpecializedActorTaskScheduler,
    },
    static_contact::{scan_deepest_static_contact, StaticContactQuery, StaticModelContact},
    static_damage::StaticDamageScheduler,
    world_fx::{BallisticDamageRequest, ParticleEntityImpact, WorldFx},
};

const ORDINARY_PLAYER_ARRIVAL: [i16; 3] = [19_712, -500, 14_848];

fn ordinary_manager(
    level: u32,
    player: Option<[i16; 3]>,
) -> Option<(GameSession, EntityManager, WorldFx)> {
    let (mut session, metadata) = super::super::tests::fixture()?;
    session.load_level_by_id(level, 1).unwrap();
    let mut fx = WorldFx::new();
    let model_extent = |id| session.cache.global_model(id).map(|model| model.radius);
    let manager = EntityManager::from_authored_world(
        AuthoredWorldConstruction {
            level: session.cache.level_desc().unwrap(),
            logical_world_index: level as i32 - 12,
            type_metadata: &metadata,
            resources: EntityConstructionResources {
                terrain: session.cache.terrain(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&model_extent),
            },
            player_arrival: player.map(|position_raw| AuthoredPlayerArrival {
                position_raw,
                heading_raw: 0x4000,
            }),
            retail_tick: 4793,
        },
        &mut fx,
    )
    .unwrap();
    Some((session, manager, fx))
}

fn resolve(
    session: &mut GameSession,
    manager: &mut EntityManager,
    fx: &mut WorldFx,
    damage: &mut StaticDamageScheduler,
    notifications: &mut GameplayNotifications,
    scheduler: &mut SpecializedActorTaskScheduler,
    tick: u32,
    id: u32,
) -> Type53ContactOutcome {
    super::resolve_type53_static_contact(
        &mut crate::intro2_contacts::Intro2ContactFrame {
            entities: manager,
            resources: &mut session.cache,
            world_fx: fx,
            static_damage: damage,
            notifications,
            retail_tick: tick,
            actor_tasks: scheduler,
        },
        id,
    )
}

/// First static overlap in a deterministic neighborhood sweep around the
/// actor's current pose, with the resolving contact at that pose.
fn find_overlap(
    session: &GameSession,
    manager: &EntityManager,
    id: u32,
    tick: u32,
) -> Option<([i16; 3], StaticModelContact)> {
    let entity = manager.iter_all().find(|entity| entity.id == id)?;
    let center = entity.position_raw();
    let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
        return None;
    };
    let model = session.cache.global_model(entity.model_index?)?;
    for dx in (-1024..=1024).step_by(64) {
        for dz in (-1024..=1024).step_by(64) {
            let position = [
                center[0].wrapping_add(dx as i16),
                center[1],
                center[2].wrapping_add(dz as i16),
            ];
            let contact: Option<StaticModelContact> =
                scan_deepest_static_contact(StaticContactQuery {
                    terrain: session.cache.terrain()?,
                    terrain_objects: session.cache.terrain_objects()?,
                    model_pool: &session.cache,
                    tick,
                    active_model: model,
                    active_model_to_world_basis: basis
                        .orientation_world_from_model()
                        .map(|row| row.map(f64::from)),
                    active_anim_vars: &entity.presentation_anim_vars(tick),
                    position_raw: position,
                })
                .ok()?;
            if let Some(contact) = contact {
                return Some((position, contact));
            }
        }
    }
    None
}

#[v2k_test_support::retail_test]
fn authored_positions_miss_without_mutation_or_rng() {
    let mut counts = Vec::new();
    for level in [14u32, 15] {
        let Some((mut session, mut manager, mut fx)) =
            ordinary_manager(level, Some(ORDINARY_PLAYER_ARRIVAL))
        else {
            return;
        };
        let ids: Vec<u32> = manager
            .iter_all()
            .filter(|entity| entity.entity_type == 53)
            .map(|entity| entity.id)
            .collect();
        let mut scheduler = SpecializedActorTaskScheduler::default();
        assert_eq!(scheduler.adopt_intro2_type53(&manager), ids.len());
        counts.push((level, ids.len()));
        let mut damage = StaticDamageScheduler::new();
        let mut notifications = GameplayNotifications::new();
        for id in ids {
            let before_slots = ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| {
                manager
                    .iter_all()
                    .find(|e| e.id == id)
                    .unwrap()
                    .actor_tasks
                    .task_in_slot(slot)
            });
            let before_context = manager
                .iter_all()
                .find(|e| e.id == id)
                .unwrap()
                .current_behavior_context;
            let mut oracle = fx.fork_for_main_base_abort_transaction();
            let outcome = resolve(
                &mut session,
                &mut manager,
                &mut fx,
                &mut damage,
                &mut notifications,
                &mut scheduler,
                4793,
                id,
            );
            assert_eq!(outcome, Type53ContactOutcome::Miss);
            assert_eq!(
                fx.next_shared_retail_random_u16(),
                oracle.next_shared_retail_random_u16(),
                "a geometric miss consumes no task work"
            );
            let entity = manager.iter_all().find(|e| e.id == id).unwrap();
            assert_eq!(
                ActorTaskSlot::IN_RETAIL_TICK_ORDER
                    .map(|slot| entity.actor_tasks.task_in_slot(slot)),
                before_slots
            );
            assert_eq!(entity.current_behavior_context, before_context);
            assert!(Intro2Type53Owner::adopt(&manager, id).is_ok());
        }
    }
    assert_eq!(counts, [(14, 3), (15, 3)]);
    // Native Intro2 births miss their authored poses as well.
    let (mut session, metadata) = super::super::tests::fixture().unwrap();
    session.load_level_by_id(50, 1).unwrap();
    let mut fx = WorldFx::new();
    let model_extent = |id| session.cache.global_model(id).map(|model| model.radius);
    let mut manager = EntityManager::from_native_intro2_frontend(
        session.cache.level_desc().unwrap(),
        &metadata,
        EntityConstructionResources {
            terrain: session.cache.terrain(),
            terrain_objects: session.cache.terrain_objects(),
            model_extent_raw: Some(&model_extent),
        },
        4793,
        &mut fx,
    )
    .unwrap();
    let ids: Vec<u32> = manager
        .iter_all()
        .filter(|entity| entity.entity_type == 53)
        .map(|entity| entity.id)
        .collect();
    assert_eq!(ids.len(), 4);
    let mut scheduler = SpecializedActorTaskScheduler::default();
    assert_eq!(scheduler.adopt_intro2_type53(&manager), 4);
    let mut damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    for id in ids {
        let mut oracle = fx.fork_for_main_base_abort_transaction();
        let outcome = resolve(
            &mut session,
            &mut manager,
            &mut fx,
            &mut damage,
            &mut notifications,
            &mut scheduler,
            4793,
            id,
        );
        assert_eq!(outcome, Type53ContactOutcome::Miss);
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            oracle.next_shared_retail_random_u16()
        );
    }
}

#[v2k_test_support::retail_test]
fn non_type53_owners_are_ineligible() {
    let Some((mut session, mut manager, mut fx)) =
        ordinary_manager(14, Some(ORDINARY_PLAYER_ARRIVAL))
    else {
        return;
    };
    let peasant = manager
        .iter_all()
        .find(|entity| entity.entity_type == 9)
        .unwrap()
        .id;
    let mut scheduler = SpecializedActorTaskScheduler::default();
    scheduler.adopt_intro2_type53(&manager);
    let mut damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    let mut oracle = fx.fork_for_main_base_abort_transaction();
    let outcome = resolve(
        &mut session,
        &mut manager,
        &mut fx,
        &mut damage,
        &mut notifications,
        &mut scheduler,
        4793,
        peasant,
    );
    assert_eq!(outcome, Type53ContactOutcome::Ineligible);
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        oracle.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn overlapping_static_applies_02ca0_hook_with_exact_prefix() {
    let Some((mut session, mut manager, mut fx)) =
        ordinary_manager(14, Some(ORDINARY_PLAYER_ARRIVAL))
    else {
        return;
    };
    let id = manager
        .iter_all()
        .find(|entity| entity.entity_type == 53 && entity.authored_spawn_index == Some(35))
        .unwrap()
        .id;
    let mut scheduler = SpecializedActorTaskScheduler::default();
    assert_eq!(scheduler.adopt_intro2_type53(&manager), 3);
    let (position, expected) = find_overlap(&session, &manager, id, 4793)
        .expect("world14 spawn35 must overlap static geometry nearby");
    let entity = manager.iter_all().find(|e| e.id == id).unwrap();
    let Some(ActorTaskRuntime::SharedRetarget(task)) =
        entity.actor_task_state(ActorTaskSlot::Primary)
    else {
        panic!("fresh spawn35 retains its acquiring retarget");
    };
    let before_private = task.private_state();
    let before_heading = entity.heading_raw();
    let RetailRuntimeValue::Known(Some(before_sub_a)) = entity.sub_a_propulsion_runtime else {
        panic!("fresh Sub-A runtime");
    };
    let before_sub_a_direction = before_sub_a.direction_multiplier();
    manager
        .entity_mut(id)
        .unwrap()
        .set_motion_raw(position, [0; 3]);
    let mut oracle = fx.fork_for_main_base_abort_transaction();
    let mut damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    let outcome = resolve(
        &mut session,
        &mut manager,
        &mut fx,
        &mut damage,
        &mut notifications,
        &mut scheduler,
        4793,
        id,
    );
    let Type53ContactOutcome::Applied(applied) = outcome else {
        panic!("overlap must apply: {outcome:?}");
    };
    assert_eq!(applied.contact.cell, expected.cell);
    assert_eq!(applied.contact.model_id, expected.model_id);
    assert_eq!(applied.position_before_raw, position);
    // The 02CA0 hook consumes exactly two words before physical response:
    // target X then Z from each low16 shifted by six.
    let mut word = || u32::from(oracle.next_shared_retail_random_u16());
    let offset = |random_word: u32| (((random_word as u16) >> 6) as i16).wrapping_sub(0x200);
    let expected_target = [
        position[0].wrapping_add(offset(word())),
        before_private.target_position_raw[1],
        position[2].wrapping_add(offset(word())),
    ];
    let entity = manager.iter_all().find(|e| e.id == id).unwrap();
    assert_eq!(entity.position_raw(), applied.position_after_raw);
    assert_eq!(entity.velocity_raw(), applied.velocity_after_raw);
    let Some(ActorTaskRuntime::SharedRetarget(task)) =
        entity.actor_task_state(ActorTaskSlot::Primary)
    else {
        panic!("contact preserves the acquiring retarget graph");
    };
    let after_private = task.private_state();
    assert_eq!(
        after_private.direction,
        before_private.direction.wrapping_neg()
    );
    assert_eq!(after_private.reversal_timer_ms, 2_500);
    assert_eq!(after_private.target_position_raw, expected_target);
    assert_eq!(
        after_private.tracked_entity_handle,
        before_private.tracked_entity_handle
    );
    assert_eq!(entity.heading_raw(), before_heading);
    let RetailRuntimeValue::Known(Some(after_sub_a)) = entity.sub_a_propulsion_runtime else {
        panic!("contact retains Sub-A");
    };
    assert_eq!(after_sub_a.direction_multiplier(), after_private.direction);
    assert_ne!(before_sub_a_direction, after_private.direction);
    // A fence brush is nonlethal: no class12 publication, health survives.
    assert!(entity.collision.health_raw != RetailRuntimeValue::Known(0));
    assert!(applied
        .actor_damage
        .as_ref()
        .is_none_or(|damage| damage.death_publication.is_none()));
    assert!(Intro2Type53Owner::adopt(&manager, id).is_ok());
    assert_eq!(
        scheduler.family_for(id),
        Some(SpecializedActorTaskFamily::Intro2Type53)
    );
}

#[v2k_test_support::retail_test]
fn driven_chase_graph_applies_02ca0_hook_with_exact_prefix() {
    // Parking the player beside spawn34 turns real PlayerNearby selection
    // into class7 Search births; the live scheduler then drives genuine
    // ADE0 Chase/Aim handoffs. No synthetic task is used. The retail Chase
    // installer 0x403360 stores 02CA0 at task +0x20, so the scan commits the
    // Primary's shared private state while the coexisting Tertiary Aim keeps
    // its own records.
    let (mut session, metadata) = super::super::tests::fixture().unwrap();
    session.load_level_by_id(14, 1).unwrap();
    let mut fx = WorldFx::new();
    let model_extent = |id| session.cache.global_model(id).map(|model| model.radius);
    let mut manager = EntityManager::from_authored_world(
        AuthoredWorldConstruction {
            level: session.cache.level_desc().unwrap(),
            logical_world_index: 2,
            type_metadata: &metadata,
            resources: EntityConstructionResources {
                terrain: session.cache.terrain(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&model_extent),
            },
            player_arrival: Some(AuthoredPlayerArrival {
                position_raw: [-16_640, -832, 29_184],
                heading_raw: 0x4000,
            }),
            retail_tick: 4793,
        },
        &mut fx,
    )
    .unwrap();
    // Spawn35's authored neighborhood supplies real static geometry for the
    // contact phase. Keep it independent of which actor acquires Chase first
    // and of that actor's subsequent path and vertical position.
    let contact_origin = manager
        .iter_all()
        .find(|entity| entity.entity_type == 53 && entity.authored_spawn_index == Some(35))
        .unwrap()
        .position_raw();
    let mut scheduler = SpecializedActorTaskScheduler::default();
    scheduler.adopt_intro2_type53(&manager);
    let mut damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    let mut chaser = None;
    for tick in 4794..5594 {
        let pass = scheduler.tick(
            &mut manager,
            SpecializedActorTaskProductionFrame {
                world:
                    crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                hive_components: None,
                notification_phase:
                    crate::gameplay_notifications::GameplayNotificationPhase::Playing,
                resources: &mut session.cache,
                world_fx: &mut fx,
                static_damage: &mut damage,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: tick,
                main_base_abort_active: false,
            },
            &mut notifications,
        );
        assert!(pass.block.is_none(), "{pass:?}");
        chaser = manager
            .iter_all()
            .filter(|entity| entity.entity_type == 53)
            .find(|entity| {
                matches!(
                    entity.actor_task_state(ActorTaskSlot::Primary),
                    Some(ActorTaskRuntime::ChaseTarget(_))
                )
            })
            .map(|entity| (entity.id, tick));
        if chaser.is_some() {
            break;
        }
    }
    let Some((id, tick)) = chaser else {
        panic!("a real Chase handoff must occur in this bounded drive");
    };
    let entity = manager.iter_all().find(|e| e.id == id).unwrap();
    let before_aim = entity.actor_task_state(ActorTaskSlot::Tertiary).copied();
    assert!(matches!(before_aim, Some(ActorTaskRuntime::AimAndFire(_))));
    let Some(ActorTaskRuntime::ChaseTarget(task)) = entity.actor_task_state(ActorTaskSlot::Primary)
    else {
        panic!("chase primary");
    };
    let before_private = task.private_state();
    let before_heading = entity.heading_raw();
    let RetailRuntimeValue::Known(Some(before_sub_a)) = entity.sub_a_propulsion_runtime else {
        panic!("chase Sub-A runtime");
    };
    // Preserve the driven Chase/Aim graph, heading, and body basis while
    // placing its contact probe beside known authored static geometry.
    manager
        .entity_mut(id)
        .unwrap()
        .set_motion_raw(contact_origin, [0; 3]);
    let (position, expected) =
        find_overlap(&session, &manager, id, tick).expect("spawn35 neighborhood must overlap");
    manager
        .entity_mut(id)
        .unwrap()
        .set_motion_raw(position, [0; 3]);
    let mut oracle = fx.fork_for_main_base_abort_transaction();
    let outcome = resolve(
        &mut session,
        &mut manager,
        &mut fx,
        &mut damage,
        &mut notifications,
        &mut scheduler,
        tick,
        id,
    );
    let Type53ContactOutcome::Applied(applied) = outcome else {
        panic!("chase overlap must apply: {outcome:?}");
    };
    assert_eq!(applied.contact.cell, expected.cell);
    assert_eq!(applied.contact.model_id, expected.model_id);
    assert_eq!(applied.position_before_raw, position);
    let mut word = || u32::from(oracle.next_shared_retail_random_u16());
    let offset = |random_word: u32| (((random_word as u16) >> 6) as i16).wrapping_sub(0x200);
    let expected_target = [
        position[0].wrapping_add(offset(word())),
        before_private.target_position_raw[1],
        position[2].wrapping_add(offset(word())),
    ];
    let entity = manager.iter_all().find(|e| e.id == id).unwrap();
    assert_eq!(entity.position_raw(), applied.position_after_raw);
    assert_eq!(entity.velocity_raw(), applied.velocity_after_raw);
    let Some(ActorTaskRuntime::ChaseTarget(task)) = entity.actor_task_state(ActorTaskSlot::Primary)
    else {
        panic!("contact preserves the chase graph");
    };
    let after_private = task.private_state();
    assert_eq!(
        after_private.direction,
        before_private.direction.wrapping_neg()
    );
    assert_eq!(after_private.reversal_timer_ms, 2_500);
    assert_eq!(after_private.target_position_raw, expected_target);
    assert_eq!(
        after_private.tracked_entity_handle,
        before_private.tracked_entity_handle
    );
    assert_eq!(entity.heading_raw(), before_heading);
    // The Tertiary Aim keeps its own records through the Primary hook.
    assert_eq!(
        entity.actor_task_state(ActorTaskSlot::Tertiary).copied(),
        before_aim
    );
    let RetailRuntimeValue::Known(Some(after_sub_a)) = entity.sub_a_propulsion_runtime else {
        panic!("contact retains Sub-A");
    };
    assert_eq!(after_sub_a.direction_multiplier(), after_private.direction);
    assert_eq!(
        after_sub_a.target_speed_raw(),
        before_sub_a.target_speed_raw()
    );
    assert_eq!(
        after_sub_a.drive_scale_percent(),
        before_sub_a.drive_scale_percent()
    );
    assert!(entity.collision.health_raw != RetailRuntimeValue::Known(0));
    assert!(applied
        .actor_damage
        .as_ref()
        .is_none_or(|damage| damage.death_publication.is_none()));
    assert!(Intro2Type53Owner::adopt(&manager, id).is_ok());
    assert_eq!(
        scheduler.family_for(id),
        Some(SpecializedActorTaskFamily::Intro2Type53)
    );
}

#[v2k_test_support::retail_test]
fn dying_type53_uses_null_hook_without_task_writes() {
    let Some((mut session, mut manager, mut fx)) =
        ordinary_manager(14, Some(ORDINARY_PLAYER_ARRIVAL))
    else {
        return;
    };
    let id = manager
        .iter_all()
        .find(|entity| entity.entity_type == 53 && entity.authored_spawn_index == Some(35))
        .unwrap()
        .id;
    let mut scheduler = SpecializedActorTaskScheduler::default();
    assert_eq!(scheduler.adopt_intro2_type53(&manager), 3);
    let mut notifications = GameplayNotifications::new();
    manager.entity_mut(id).unwrap().collision.health_raw = RetailRuntimeValue::Known(1);
    let result = super::super::impact::apply_intro2_type53_particle_hit(
        SharedActorImpactFrame {
            resources: &session.cache,
            entities: &mut manager,
            world_fx: &mut fx,
            scheduler: &mut scheduler,
            notifications: &mut notifications,
            retail_tick: 4793,
        },
        ParticleEntityImpact {
            source_particle_class: 16,
            impact_position_argument_va: 0,
            target_entity_id: id,
            position_world: [0.0; 3],
            velocity_raw: [0, 0, 8192],
            damage: Some(BallisticDamageRequest {
                packet: DamagePacket {
                    channels: [1, 0],
                    amounts_raw: [2_500, 0],
                },
                source_entity_type_at_birth: Some(34),
                source_owner_id: Some(35),
            }),
        },
    );
    let super::super::impact::Intro2Type53ImpactOutcome::Applied(hit) = result else {
        panic!("lethal fragment must apply: {result:?}");
    };
    assert!(hit.death_publication.is_some());
    assert_eq!(
        manager
            .iter_all()
            .find(|e| e.id == id)
            .unwrap()
            .collision
            .health_raw,
        RetailRuntimeValue::Known(0)
    );
    assert!(Intro2CommonDyingOwner::adopt(&manager, id).is_ok());
    assert_eq!(
        scheduler.family_for(id),
        Some(SpecializedActorTaskFamily::Intro2CommonDying)
    );
    // The corpse still scans static geometry; its null hook writes no task,
    // Sub-A or RNG state before physical response.
    let (position, _) =
        find_overlap(&session, &manager, id, 4793).expect("corpse pose must overlap nearby");
    let before_tasks = ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| {
        manager
            .iter_all()
            .find(|e| e.id == id)
            .unwrap()
            .actor_tasks
            .task_in_slot(slot)
    });
    manager
        .entity_mut(id)
        .unwrap()
        .set_motion_raw(position, [0; 3]);
    let mut damage = StaticDamageScheduler::new();
    let outcome = resolve(
        &mut session,
        &mut manager,
        &mut fx,
        &mut damage,
        &mut notifications,
        &mut scheduler,
        4793,
        id,
    );
    let Type53ContactOutcome::Applied(applied) = outcome else {
        panic!("corpse overlap must apply: {outcome:?}");
    };
    let entity = manager.iter_all().find(|e| e.id == id).unwrap();
    assert_eq!(
        ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_tasks.task_in_slot(slot)),
        before_tasks
    );
    assert!(matches!(
        entity.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::CommonDying(_))
    ));
    assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
    assert!(
        applied
            .actor_damage
            .as_ref()
            .is_none_or(|damage| damage.death_publication.is_none()),
        "already-dying contact must not republish class12"
    );
    assert!(Intro2CommonDyingOwner::adopt(&manager, id).is_ok());
}
