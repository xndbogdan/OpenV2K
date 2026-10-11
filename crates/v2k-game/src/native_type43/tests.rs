//! Ordinary emitter-only Type43 shooters on their native owner.

use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    class49_death::{source_profile, NativeExplosionSourceProfile},
    damage::DamagePacket,
    entity_collision_state::RetailRuntimeValue,
    gameplay_notifications::{GameplayNotificationPhase, GameplayNotifications},
    player_hull::PlayerHull,
    shared_actor_impact::{
        apply_playing_actor_particle_hit, PlayingActorImpactFrame, SharedActorImpactOutcome,
    },
    specialized_actor_task_production::{
        SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
        SpecializedActorTaskScheduler, SpecializedActorTaskWorld,
    },
    static_damage::StaticDamageScheduler,
    world_fx::{BallisticDamageRequest, ParticleEntityImpact},
};
use crate::{
    intro2_contacts::Intro2ContactFrame,
    native_ground_actor::contact::{resolve_insect_static_contact, NativeGroundContactOutcome},
};

/// Every authored Type43: worlds 42, 46 and 47.
const WORLDS: [u32; 3] = [42, 46, 47];

fn type43_ids(manager: &EntityManager) -> Vec<u32> {
    manager
        .iter_all()
        .filter(|entity| entity.entity_type == ENTITY_TYPE)
        .map(|entity| entity.id)
        .collect()
}

fn entity(manager: &EntityManager, id: u32) -> &Entity {
    manager.iter_all().find(|entity| entity.id == id).unwrap()
}

fn activate(manager: &mut EntityManager, ids: &[u32]) {
    for &id in ids {
        let entity = manager.entity_mut(id).unwrap();
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(0x0202_8000, 0x0202_8000);
        entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
    }
}

fn style(manager: &EntityManager, id: u32) -> u32 {
    let RetailRuntimeValue::Known(Some(context)) = entity(manager, id).current_behavior_context
    else {
        panic!("context");
    };
    context.active_style().style_address()
}

#[allow(clippy::too_many_arguments)]
fn tick(
    tasks: &mut SpecializedActorTaskScheduler,
    manager: &mut EntityManager,
    session: &mut crate::session::GameSession,
    fx: &mut crate::world_fx::WorldFx,
    static_damage: &mut StaticDamageScheduler,
    notifications: &mut GameplayNotifications,
    tick: u32,
) -> crate::specialized_actor_task_production::SpecializedActorTaskProductionPass {
    tasks.tick(
        manager,
        SpecializedActorTaskProductionFrame {
            world: SpecializedActorTaskWorld::Cinematic,
            hive_components: None,
            notification_phase: GameplayNotificationPhase::Playing,
            resources: &mut session.cache,
            world_fx: fx,
            static_damage,
            elapsed_micros: 20_000,
            global_elapsed_micros: tick * 20_000,
            retail_tick: tick,
            main_base_abort_active: false,
        },
        notifications,
    )
}

fn assert_no_block(
    pass: &crate::specialized_actor_task_production::SpecializedActorTaskProductionPass,
    context: &str,
) {
    assert!(pass.block.is_none(), "{context}: {:?}", pass.block);
    for outcome in &pass.outcomes {
        if let SpecializedActorTaskProductionOutcome::NativeType43(outcome) = outcome {
            assert!(
                !matches!(
                    outcome,
                    Type43Outcome::Blocked { .. } | Type43Outcome::Pending { .. }
                ),
                "{context}: {outcome:?}"
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn type43_births_publish_the_emitter_only_search_graph() {
    let mut births = 0;
    for level in WORLDS {
        let (session, manager, _) = crate::intro2_gun_turret::authored_tests::fixture(level);
        let terrain = session.cache.terrain().unwrap();
        let ids = type43_ids(&manager);
        assert!(!ids.is_empty(), "world{level}");
        for &id in &ids {
            births += 1;
            let entity = entity(&manager, id);
            assert!(manager_allocation_authenticates(&manager, id));
            let runtime = entity.native_type43_runtime.unwrap();
            // 24E30 copies the row's method and sound into its only component.
            assert_eq!(runtime.sub_e_runtime.projectile_method, 10);
            assert_eq!(runtime.sub_e_runtime.sound_id, 68);
            // D4A0 grounds the authored X/Z; +C0 drops the terrain/water
            // lane, and Search's +38 frees the fixed body (motion, pairs).
            let [x, y, z] = entity.position_raw();
            assert_eq!(y, terrain.bilinear_height_raw(x, z), "world{level} id{id}");
            assert_eq!(
                entity.collision.state_flags_at_0x08.masked(0x0805_8000),
                RetailRuntimeValue::Known(0x0004_8000)
            );
            // B6C0: slot-1 acquisition, the 500-ms 02BA0 retarget, no Aim.
            assert_eq!(style(&manager, id), 0x004c_7a50);
            assert!(matches!(
                entity.actor_task_state(ActorTaskSlot::Secondary),
                Some(ActorTaskRuntime::TargetAcquisition(_))
            ));
            assert!(matches!(
                entity.actor_task_state(ActorTaskSlot::Primary),
                Some(ActorTaskRuntime::SharedRetarget(task)) if task.lifetime_ms() == 500
            ));
            assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
            assert_eq!(
                entity.actor_common_axis_descriptor,
                RetailRuntimeValue::Known(AXIS)
            );
            assert!(Type43Owner::adopt(&manager, id).is_ok());
            assert_eq!(
                source_profile(entity),
                Some(NativeExplosionSourceProfile::Type43)
            );
        }
        let mut tasks = SpecializedActorTaskScheduler::new();
        assert_eq!(tasks.adopt_type43(&manager), ids.len(), "world{level}");
    }
    assert_eq!(births, 12);
}

/// Ten seconds of the real scheduler: E-only 01430, E100 and DF70 grounding.
#[v2k_test_support::retail_test]
fn type43_cohorts_stay_grounded_for_ten_seconds() {
    for level in WORLDS {
        let (mut session, mut manager, mut fx) =
            crate::intro2_gun_turret::authored_tests::fixture(level);
        let ids = type43_ids(&manager);
        activate(&mut manager, &ids);
        let start: Vec<_> = ids
            .iter()
            .map(|&id| entity(&manager, id).position_raw())
            .collect();
        let mut tasks = SpecializedActorTaskScheduler::new();
        assert_eq!(tasks.adopt_type43(&manager), ids.len());
        let mut static_damage = StaticDamageScheduler::new();
        let mut notifications = GameplayNotifications::new();
        for t in 1..=500u32 {
            let pass = tick(
                &mut tasks,
                &mut manager,
                &mut session,
                &mut fx,
                &mut static_damage,
                &mut notifications,
                t,
            );
            assert_no_block(&pass, &format!("world{level} tick{t}"));
            // Search frees 0x08000000, so 11AD0 scans every living shooter.
            for &id in &ids {
                let outcome = resolve_insect_static_contact(
                    &mut Intro2ContactFrame {
                        entities: &mut manager,
                        resources: &mut session.cache,
                        world_fx: &mut fx,
                        static_damage: &mut static_damage,
                        notifications: &mut notifications,
                        retail_tick: t,
                        actor_tasks: &mut tasks,
                    },
                    id,
                );
                assert!(
                    matches!(
                        outcome,
                        NativeGroundContactOutcome::Miss | NativeGroundContactOutcome::Applied(_)
                    ),
                    "world{level} tick{t} id{id}: {outcome:?}"
                );
            }
        }
        for (&id, start) in ids.iter().zip(start) {
            let entity = entity(&manager, id);
            // DF70 zeroes vertical velocity and grounds at the pre-motion X/Z;
            // only E100's wind drag can move a body nobody has hit.
            assert_eq!(entity.velocity_raw()[1], 0, "world{level} id{id}");
            assert_eq!(
                entity.collision.state_flags_at_0x08.masked(0x0080_0000),
                RetailRuntimeValue::Known(0x0080_0000)
            );
            let [x, _, z] = entity.position_raw();
            println!(
                "world{level} id{id} drift [{}, {}]",
                i32::from(x) - i32::from(start[0]),
                i32::from(z) - i32::from(start[2])
            );
            assert!(Type43Owner::adopt(&manager, id).is_ok());
        }
    }
}

/// A player inside the 2560 axis is acquired: ADE0 publishes Chase and Aim,
/// and method10 shots leave through the native FIFO as class38 particles.
#[v2k_test_support::retail_test]
fn type43_acquires_the_player_and_fires_method10() {
    let (mut session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(46);
    let ids = type43_ids(&manager);
    activate(&mut manager, &ids);
    let id = ids[0];
    let [x, y, z] = entity(&manager, id).position_raw();
    manager.player_mut().unwrap().set_position_raw([
        x.wrapping_add(0x300),
        y.wrapping_add(0x200),
        z,
    ]);
    let mut tasks = SpecializedActorTaskScheduler::new();
    assert_eq!(tasks.adopt_type43(&manager), ids.len());
    let mut static_damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    let player = manager.player().unwrap().id;
    let mut pursued = false;
    let mut fired = 0;
    for t in 1..=250u32 {
        let pass = tick(
            &mut tasks,
            &mut manager,
            &mut session,
            &mut fx,
            &mut static_damage,
            &mut notifications,
            t,
        );
        assert_no_block(&pass, &format!("tick{t}"));
        if style(&manager, id) == 0x004c_7a98 {
            pursued = true;
            let RetailRuntimeValue::Known(Some(context)) =
                entity(&manager, id).current_behavior_context
            else {
                unreachable!()
            };
            assert_eq!(
                context.target_handle_at_0x08(),
                RetailRuntimeValue::Known(Some(player))
            );
        }
        let queued = entity(&manager, id)
            .native_type43_aim_runtime
            .as_ref()
            .map_or(0, |runtime| runtime.queued_shot_count());
        if queued > 0 {
            let outcome = aim::drain_type43_shots(
                &mut manager,
                &mut fx,
                id,
                crate::world_fx::ParticleEnvironment::Terrain(
                    crate::world_fx::TerrainCollisionContext::from_current_level_cache(
                        &session.cache,
                    )
                    .unwrap(),
                ),
                t,
            )
            .unwrap();
            assert_eq!(outcome.consumed_requests, queued);
            assert!(outcome
                .materialized_particle_classes
                .iter()
                .all(|&class| matches!(class, 38 | 46)));
            fired += queued;
        }
    }
    assert!(pursued, "the player was never acquired");
    assert!(fired > 0, "no method10 shot left the FIFO");
}

fn lethal_hit(id: u32) -> ParticleEntityImpact {
    ParticleEntityImpact {
        source_particle_class: 16,
        impact_position_argument_va: 0,
        target_entity_id: id,
        position_world: [0.0; 3],
        velocity_raw: [0, 0, 8192],
        damage: Some(BallisticDamageRequest {
            packet: DamagePacket {
                channels: [2, 0],
                amounts_raw: [100_000, 0],
            },
            source_entity_type_at_birth: Some(34),
            source_owner_id: Some(35),
        }),
    }
}

/// A lethal Playing hit publishes alternate class1: BAC0's blast and radial
/// run synchronously, every task is cleared and removal is staged.
#[v2k_test_support::retail_test]
fn a_lethal_playing_hit_explodes_the_type43() {
    for level in WORLDS {
        let (mut session, mut manager, mut fx) =
            crate::native_type122::construction_tests::native_fixture_with_player(level);
        manager.cleanup_pending_actor_deferred_destroys();
        let ids = type43_ids(&manager);
        activate(&mut manager, &ids);
        let mut tasks = SpecializedActorTaskScheduler::new();
        assert_eq!(tasks.adopt_type43(&manager), ids.len());
        let id = ids[0];
        let mut static_damage = StaticDamageScheduler::new();
        let mut notifications = GameplayNotifications::new();
        let mut hull = PlayerHull::default();
        let outcome = apply_playing_actor_particle_hit(
            PlayingActorImpactFrame {
                extra_lives: RetailRuntimeValue::Known(3),
                resources: &mut session.cache,
                entities: &mut manager,
                world_fx: &mut fx,
                scheduler: &mut tasks,
                notifications: &mut notifications,
                static_damage: &mut static_damage,
                player_hull: &mut hull,
                retail_tick: 600,
            },
            lethal_hit(id),
        );
        assert!(
            matches!(
                &outcome,
                Some(SharedActorImpactOutcome::Type43(impact::Type43ImpactOutcome::Applied(
                    applied
                ))) if applied.death_publication.is_none()
            ),
            "world{level}: {outcome:?}"
        );
        assert!(crate::class49_death::finished_terminal_hit_authenticates(
            &manager, id
        ));
        assert!(manager.pending_actor_deferred_destroy_ids().contains(&id));
        assert_eq!(style(&manager, id), 0x004c_7150);
        assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .all(|slot| entity(&manager, id)
                .actor_tasks
                .task_in_slot(slot)
                .is_none()));
        // The scheduler retires the living owner; nothing replays the prefix.
        let pass = tick(
            &mut tasks,
            &mut manager,
            &mut session,
            &mut fx,
            &mut static_damage,
            &mut notifications,
            601,
        );
        assert!(pass.block.is_none(), "world{level}: {:?}", pass.block);
        assert!(pass.outcomes.iter().all(|outcome| !matches!(
            outcome,
            SpecializedActorTaskProductionOutcome::NativeType43(Type43Outcome::Advanced {
                entity_id,
                ..
            }) if *entity_id == id
        )));
    }
}

/// A nonlethal hit runs C690: AC60 reselects class7 and B6C0 republishes
/// Search; the scheduler keeps the new graph's owner.
#[v2k_test_support::retail_test]
fn a_nonlethal_hit_reselects_search() {
    let (mut session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(42);
    let ids = type43_ids(&manager);
    activate(&mut manager, &ids);
    let mut tasks = SpecializedActorTaskScheduler::new();
    assert_eq!(tasks.adopt_type43(&manager), ids.len());
    let id = ids[0];
    let mut hit = lethal_hit(id);
    hit.damage.as_mut().unwrap().packet.amounts_raw = [2_100, 0];
    let mut static_damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    let mut hull = PlayerHull::default();
    let outcome = apply_playing_actor_particle_hit(
        PlayingActorImpactFrame {
            extra_lives: RetailRuntimeValue::Known(3),
            resources: &mut session.cache,
            entities: &mut manager,
            world_fx: &mut fx,
            scheduler: &mut tasks,
            notifications: &mut notifications,
            static_damage: &mut static_damage,
            player_hull: &mut hull,
            retail_tick: 600,
        },
        hit,
    );
    assert!(
        matches!(
            &outcome,
            Some(SharedActorImpactOutcome::Type43(impact::Type43ImpactOutcome::Applied(
                applied
            ))) if applied.death_publication.is_none() && applied.filtered_damage_raw != 0
        ),
        "{outcome:?}"
    );
    let entity = entity(&manager, id);
    assert!(
        matches!(entity.collision.health_raw, RetailRuntimeValue::Known(health) if health > 0 && health < HEALTH)
    );
    assert_eq!(style(&manager, id), 0x004c_7a50);
    assert!(tasks.type43_completed_owner(&manager, id));
    for t in 601..=650 {
        let pass = tick(
            &mut tasks,
            &mut manager,
            &mut session,
            &mut fx,
            &mut static_damage,
            &mut notifications,
            t,
        );
        assert_no_block(&pass, &format!("tick{t}"));
    }
}

/// A shooter moved into a Section-10 model takes A8B0's 02CA0 on its
/// retarget Primary: with no A or G it only reverses direction, installs the
/// 2500-ms timer and retargets. Its default flags have no D920 crush bit.
#[v2k_test_support::retail_test]
fn a_type43_inside_a_static_model_takes_the_insect_static_hook() {
    let (mut session, mut manager, mut fx) = crate::intro2_gun_turret::authored_tests::fixture(46);
    let ids = type43_ids(&manager);
    activate(&mut manager, &ids);
    let mut tasks = SpecializedActorTaskScheduler::new();
    assert_eq!(tasks.adopt_type43(&manager), ids.len());
    let id = ids[0];
    let terrain = session.cache.terrain().unwrap();
    let centers: Vec<[i16; 3]> = terrain
        .cells
        .iter()
        .enumerate()
        .filter(|(_, cell)| cell.attribute != 0)
        .map(|(index, _)| {
            let x = ((index / 256) as u16 * 256 + 127) as i16;
            let z = ((index % 256) as u16 * 256 + 127) as i16;
            [x, terrain.bilinear_height_raw(x, z), z]
        })
        .collect();
    let mut static_damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    let mut applied = None;
    for center in centers {
        manager.entity_mut(id).unwrap().set_position_raw(center);
        let outcome = resolve_insect_static_contact(
            &mut Intro2ContactFrame {
                entities: &mut manager,
                resources: &mut session.cache,
                world_fx: &mut fx,
                static_damage: &mut static_damage,
                notifications: &mut notifications,
                retail_tick: 700,
                actor_tasks: &mut tasks,
            },
            id,
        );
        match outcome {
            NativeGroundContactOutcome::Miss => {}
            NativeGroundContactOutcome::Applied(contact) => {
                applied = Some(contact);
                break;
            }
            other => panic!("{center:?}: {other:?}"),
        }
    }
    let contact = applied.expect("world46 authors solid Section-10 models");
    assert!(contact.crushing_damage.is_none());
    let Some(ActorTaskRuntime::SharedRetarget(task)) =
        entity(&manager, id).actor_task_state(ActorTaskSlot::Primary)
    else {
        panic!("retained retarget Primary");
    };
    assert_eq!(task.private_state().direction, -1);
    assert_eq!(task.private_state().reversal_timer_ms, 2500);
    assert!(tasks.type43_completed_owner(&manager, id));
}

/// 02DA0 against a body ahead: 01A20 with no A or D still times, reverses
/// and retargets the retarget Primary's private record (two RNG words).
#[v2k_test_support::retail_test]
fn the_retarget_descriptor_contact_runs_without_a_or_d() {
    let (mut session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(46);
    let ids = type43_ids(&manager);
    activate(&mut manager, &ids);
    let mut tasks = SpecializedActorTaskScheduler::new();
    assert_eq!(tasks.adopt_type43(&manager), ids.len());
    let id = ids[0];
    let RetailRuntimeValue::Known(basis) = entity(&manager, id).physical_body_basis_q31() else {
        panic!("basis");
    };
    // Put the opposite body on the shooter's forward axis.
    let [x, y, z] = entity(&manager, id).position_raw();
    let ahead = |axis: i32| ((i64::from(axis) * 0x200) >> 31) as i16;
    let player = manager.player().unwrap().id;
    manager.player_mut().unwrap().set_position_raw([
        x.wrapping_add(ahead(basis.forward[0])),
        y.wrapping_add(ahead(basis.forward[1])),
        z.wrapping_add(ahead(basis.forward[2])),
    ]);
    let outcome = crate::native_actor_descriptor_contact::resolve_native_actor_descriptor_contact(
        &mut Intro2ContactFrame {
            entities: &mut manager,
            resources: &mut session.cache,
            world_fx: &mut fx,
            static_damage: &mut StaticDamageScheduler::new(),
            notifications: &mut GameplayNotifications::new(),
            retail_tick: 700,
            actor_tasks: &mut tasks,
        },
        id,
        player,
        ActorTaskSlot::Primary,
    );
    assert_eq!(
        outcome,
        Ok(
            crate::native_actor_descriptor_contact::NativeDescriptorContactOutcome::Applied {
                rng_draws: 2
            }
        )
    );
    let Some(ActorTaskRuntime::SharedRetarget(task)) =
        entity(&manager, id).actor_task_state(ActorTaskSlot::Primary)
    else {
        panic!("retained retarget Primary");
    };
    assert_eq!(task.private_state().direction, -1);
    assert_eq!(task.private_state().reversal_timer_ms, 1500);
    // The Secondary acquisition keeps 05FF0's null +18.
    let null = crate::native_actor_descriptor_contact::resolve_native_actor_descriptor_contact(
        &mut Intro2ContactFrame {
            entities: &mut manager,
            resources: &mut session.cache,
            world_fx: &mut fx,
            static_damage: &mut StaticDamageScheduler::new(),
            notifications: &mut GameplayNotifications::new(),
            retail_tick: 700,
            actor_tasks: &mut tasks,
        },
        id,
        player,
        ActorTaskSlot::Secondary,
    );
    assert_eq!(
        null,
        Ok(crate::native_actor_descriptor_contact::NativeDescriptorContactOutcome::Null)
    );
}
