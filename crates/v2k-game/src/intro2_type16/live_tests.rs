use super::native::tests::{fixture, generic};
use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    common_mover::{sub_d::Type9SubDFrameOwner, type9_attitude::Type9BodyBasis},
    defecate_virus::DefecateVirusWanderTaskState,
    entity::EntityManager,
    entity_collision_state::RetailStateWord,
    infection_evolution::{InfectionCellWrite, INFECTION_TERRAIN_TYPE_BIT},
    session::GameSession,
    world_fx::WorldFx,
};

fn admit_callbacks(entity: &mut Entity, detailed: bool) {
    entity.collision.state_flags_at_0x08.overwrite(
        0x0202_0000,
        0x0002_0000 | if detailed { 0x0200_0000 } else { 0 },
    );
    entity.collision.callback_scheduler_accumulator_us_at_0x6c =
        RetailRuntimeValue::Known(if detailed { 0 } else { 125_000 });
}

fn isolated_birth(
    session: &GameSession,
    metadata: &[EntityTypeRuntimeMetadata],
    spawn: usize,
    class: u8,
    detailed: bool,
) -> (EntityManager, u32) {
    let mut manager = generic(session, metadata);
    let id = spawn as u32 + 1;
    let ids: Vec<_> = manager.iter_all().map(|entity| entity.id).collect();
    for candidate_id in ids {
        if candidate_id == id {
            continue;
        }
        let entity = manager.entity_mut(candidate_id).unwrap();
        entity.capability_flags &= !0xc01;
        entity.collision.state_flags_at_0x08 = RetailStateWord::exact(4);
    }
    let entity = manager.entity_mut(id).unwrap();
    // Other actors are controlled non-targets; this allocation keeps its
    // authored position, flags and constructor initialization.
    let mut words = [0x1700, if class == 4 { 0xffff } else { 0 }, 0x1234].into_iter();
    let publication = publish_intro2_type16(
        entity,
        &metadata[16],
        &[],
        session.cache.terrain().unwrap(),
        &mut || words.next().expect("exact native constructor draw budget"),
    )
    .unwrap();
    assert_eq!(publication.selection.program.class_id, class);
    admit_callbacks(entity, detailed);
    (manager, id)
}

fn require_advanced(tick: Intro2Type16Tick) -> Intro2Type16Owner {
    assert!(
        matches!(
            tick.outcome,
            Intro2Type16Outcome::Advanced {
                callback_enabled: true,
                ..
            }
        ),
        "{:?}",
        tick.outcome
    );
    assert!(tick.replacement_common_dying_owner.is_none());
    tick.retained_owner
        .expect("completed living owner stays scheduled")
}

#[v2k_test_support::retail_test]
fn both_native_type16_births_run_five_seconds_with_varied_frames_and_both_initial_styles() {
    let Some((mut session, metadata)) = fixture() else {
        return;
    };
    for spawn in INTRO2_TYPE16_SPAWN_INDICES {
        for class in [4, 5] {
            for detailed in [false, true] {
                let (mut manager, id) = isolated_birth(&session, &metadata, spawn, class, detailed);
                let entity = manager.entity_mut(id).unwrap();
                let initial_position = entity.position_raw();
                let initial_health = entity.collision.health_raw;
                let angles = [0x2bcd, 0x1000, -0x900];
                entity.set_rotation_heading_pitch_roll_raw(angles);
                entity.physical_body_basis_q31 = RetailRuntimeValue::Known(
                    Type9BodyBasis::from_angle_words(angles[0], angles[1], angles[2]),
                );
                entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(19);
                let mut owner = Intro2Type16Owner::adopt(&manager, id).unwrap();
                let mut fx = WorldFx::new();
                let mut total_us = 0;
                let mut frame_index = 0;
                let mut callbacks = 0;
                while total_us < 5_000_000 {
                    let elapsed = [16_667, 33_333, 20_000, 50_000][frame_index % 4];
                    let tick = tick_intro2_type16(
                        &mut manager,
                        owner,
                        Intro2Type16Frame {
                            resources: &mut session.cache,
                            world_fx: &mut fx,
                            elapsed_micros: elapsed,
                            retail_tick: 286 + total_us / 20_000,
                        },
                    );
                    match tick.outcome {
                        Intro2Type16Outcome::Waiting { .. } => {}
                        Intro2Type16Outcome::Advanced { callback_enabled: true, .. } => {
                            let entity = manager.entity_mut(id).unwrap();
                            assert_eq!(entity.mass_raw, if callbacks == 0 { 119 } else { 100 });
                            assert_eq!(entity.collision.animation_offset_at_0xb2, RetailRuntimeValue::Known(0));
                            callbacks += 1;
                        }
                        other => panic!("spawn={spawn} initial={class} detailed={detailed} elapsed={total_us}: {other:?}"),
                    }
                    assert!(tick.replacement_common_dying_owner.is_none());
                    owner = tick
                        .retained_owner
                        .expect("no silent owner loss during five-second run");
                    assert!(!owner.has_pending_prefix());
                    total_us += elapsed;
                    frame_index += 1;
                }
                assert!(callbacks > 10);
                let entity = manager.entity_mut(id).unwrap();
                assert!(intro2_type16_allocation_authenticates(entity));
                assert_ne!(entity.position_raw(), initial_position);
                assert_eq!(entity.collision.health_raw, initial_health);
                assert!(matches!(
                    entity
                        .intro2_type16_runtime
                        .unwrap()
                        .sub_d_owner
                        .classifier_cache()
                        .origin(),
                    RetailRuntimeValue::Known(_)
                ));
                let [heading, pitch, roll] = entity.rotation_heading_pitch_roll_raw();
                assert_eq!(
                    entity.physical_body_basis_q31(),
                    RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(
                        heading, pitch, roll
                    ))
                );
            }
        }
    }
}

#[v2k_test_support::retail_test]
fn native_type16_class4_strict_expiry_retires_old_graph_after_mover_and_runs_new_tertiary_only() {
    let Some((mut session, metadata)) = fixture() else {
        return;
    };
    let (mut manager, id) = isolated_birth(&session, &metadata, 5, 4, true);
    let entity = manager.entity_mut(id).unwrap();
    let old_primary = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .unwrap();
    let old_tertiary = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Tertiary)
        .unwrap();
    let position = entity.position_raw();
    let Some(ActorTaskRuntime::DefecateVirusWander(task)) =
        entity.actor_tasks.task_state_mut(old_primary)
    else {
        panic!()
    };
    let mut private = task.private_state();
    private.target_position_raw = [
        position[0].wrapping_add(2000),
        position[1],
        position[2].wrapping_add(1700),
    ];
    *task = DefecateVirusWanderTaskState::from_parts(private, 1980);
    let mut owner = Intro2Type16Owner::adopt(&manager, id).unwrap();
    let mut fx = WorldFx::new();
    owner = require_advanced(tick_intro2_type16(
        &mut manager,
        owner,
        Intro2Type16Frame {
            resources: &mut session.cache,
            world_fx: &mut fx,
            elapsed_micros: 20_000,
            retail_tick: 286,
        },
    ));
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(
        entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
        Some(old_primary)
    );
    let Some(ActorTaskRuntime::DefecateVirusWander(task)) =
        entity.actor_task_state(ActorTaskSlot::Primary)
    else {
        panic!()
    };
    assert_eq!(task.elapsed_ms(), 2000, "equality must not reselect");
    require_advanced(tick_intro2_type16(
        &mut manager,
        owner,
        Intro2Type16Frame {
            resources: &mut session.cache,
            world_fx: &mut fx,
            elapsed_micros: 1_000,
            retail_tick: 287,
        },
    ));
    let entity = manager.entity_mut(id).unwrap();
    assert_ne!(
        entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
        Some(old_primary)
    );
    assert!(entity.actor_tasks.wrapper_flags(old_primary).is_none());
    assert!(entity.actor_tasks.wrapper_flags(old_tertiary).is_none());
    match entity.actor_task_state(ActorTaskSlot::Primary).unwrap() {
        ActorTaskRuntime::SharedRetarget(task) => assert_eq!(task.elapsed_ms(), 0),
        ActorTaskRuntime::DefecateVirusWander(task) => assert_eq!(task.elapsed_ms(), 0),
        other => panic!("Only the two Always intervals qualify: {other:?}"),
    }
    if let Some(ActorTaskRuntime::DefecateVirusTerrain(task)) =
        entity.actor_task_state(ActorTaskSlot::Tertiary)
    {
        assert_eq!(
            task.elapsed_ms(),
            1,
            "a replacement slot2 is visited later in the current pass"
        );
        assert_eq!(task.lifetime_ms(), 0);
    }
}

#[v2k_test_support::retail_test]
fn native_type16_coarse_class4_writes_real_terrain_before_outer_motion() {
    let Some((mut session, metadata)) = fixture() else {
        return;
    };
    let (mut manager, id) = isolated_birth(&session, &metadata, 42, 4, false);
    let position = manager.entity_mut(id).unwrap().position_raw();
    let cell = [
        ((position[0] as u16) >> 8) as u8,
        ((position[2] as u16) >> 8) as u8,
    ];
    // 02850 adds two independent random offsets in [-256,255] raw units.
    // Clear its entire possible neighborhood; it must set exactly one cell,
    // without requiring the stochastic point to equal the actor's own cell.
    let clear: Vec<_> = (-1i8..=1)
        .flat_map(|dx| {
            (-1i8..=1).map(move |dz| InfectionCellWrite {
                cell: [
                    cell[0].wrapping_add(dx as u8),
                    cell[1].wrapping_add(dz as u8),
                ],
                infected: false,
            })
        })
        .collect();
    session.cache.apply_level_infection_writes(&clear).unwrap();
    let before: Vec<_> = session
        .cache
        .level_terrain()
        .unwrap()
        .cells
        .iter()
        .map(|cell| cell.terrain_type)
        .collect();
    let owner = Intro2Type16Owner::adopt(&manager, id).unwrap();
    let mut fx = WorldFx::new();
    require_advanced(tick_intro2_type16(
        &mut manager,
        owner,
        Intro2Type16Frame {
            resources: &mut session.cache,
            world_fx: &mut fx,
            elapsed_micros: 20_000,
            retail_tick: 767,
        },
    ));
    let changed: Vec<_> = session
        .cache
        .level_terrain()
        .unwrap()
        .cells
        .iter()
        .zip(&before)
        .enumerate()
        .filter_map(|(index, (after, before))| {
            if after.terrain_type == *before {
                return None;
            }
            assert_eq!(after.terrain_type, before | INFECTION_TERRAIN_TYPE_BIT);
            Some([(index / 256) as u8, (index % 256) as u8])
        })
        .collect();
    assert_eq!(changed.len(), 1);
    assert!(clear.iter().any(|write| write.cell == changed[0]));
    let Some(ActorTaskRuntime::DefecateVirusTerrain(task)) = manager
        .entity_mut(id)
        .unwrap()
        .actor_task_state(ActorTaskSlot::Tertiary)
    else {
        panic!()
    };
    assert_eq!(task.lifetime_ms(), 0);
    assert_eq!(
        task.elapsed_ms(),
        125,
        "coarse callback uses the real capped carry"
    );
}

fn targeted_birth(
    session: &GameSession,
    metadata: &[EntityTypeRuntimeMetadata],
    class: u8,
) -> (EntityManager, u32, u32) {
    let mut manager = generic(session, metadata);
    let mut preceding = generic(session, metadata);
    let id = 6;
    let owner = manager.entity_mut(id).unwrap();
    let candidate = preceding.entity_mut(1).unwrap();
    candidate.set_position_raw(owner.position_raw());
    candidate.capability_flags = if class == 9 { 0x800 } else { 1 };
    candidate.collision.state_flags_at_0x08 = RetailStateWord::exact(4);
    let publication = publish_intro2_type16(
        owner,
        &metadata[16],
        std::slice::from_ref(candidate),
        session.cache.terrain().unwrap(),
        &mut || 0,
    )
    .unwrap();
    assert_eq!(publication.selection.program.class_id, class);
    admit_callbacks(owner, true);
    let position = owner.position_raw();
    let target_id = manager
        .iter_all()
        .find(|entity| entity.entity_type == 9)
        .unwrap()
        .id;
    let ids: Vec<_> = manager.iter_all().map(|entity| entity.id).collect();
    for candidate_id in ids {
        let candidate = manager.entity_mut(candidate_id).unwrap();
        candidate.capability_flags &= !0xc03;
        if candidate_id != id {
            candidate.collision.state_flags_at_0x08 = RetailStateWord::exact(4);
        }
    }
    let target = manager.entity_mut(target_id).unwrap();
    target.capability_flags |= if class == 9 { 0x800 } else { 1 };
    target.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
    target.set_position_raw([
        position[0].wrapping_add(700),
        position[1],
        position[2].wrapping_add(1100),
    ]);
    (manager, id, target_id)
}

#[v2k_test_support::retail_test]
fn native_type16_search_and_capture_handoffs_keep_primary_next_pass_and_aim_same_pass() {
    let Some((mut session, metadata)) = fixture() else {
        return;
    };
    for class in [7, 9] {
        let (mut manager, id, target_id) = targeted_birth(&session, &metadata, class);
        let entity = manager.entity_mut(id).unwrap();
        let old_primary = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        let old_secondary = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Secondary)
            .unwrap();
        let initial_position = entity.position_raw();
        let mut owner = Intro2Type16Owner::adopt(&manager, id).unwrap();
        let mut fx = WorldFx::new();
        let mut primary = None;
        for frame_index in 0..3 {
            owner = require_advanced(tick_intro2_type16(
                &mut manager,
                owner,
                Intro2Type16Frame {
                    resources: &mut session.cache,
                    world_fx: &mut fx,
                    elapsed_micros: 20_000,
                    retail_tick: 321 + frame_index,
                },
            ));
            let entity = manager.entity_mut(id).unwrap();
            let current = entity
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary)
                .unwrap();
            assert_ne!(current, old_primary);
            if let Some(previous) = primary {
                assert_eq!(current, previous);
            } else {
                primary = Some(current);
            }
            assert!(entity.actor_tasks.wrapper_flags(old_primary).is_none());
            assert!(entity.actor_tasks.wrapper_flags(old_secondary).is_none());
            assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
            let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
                panic!()
            };
            assert_eq!(context.style_table_index_raw_at_0x10(), 1);
            assert_eq!(
                context.target_handle_at_0x08(),
                RetailRuntimeValue::Known(Some(target_id))
            );
            match entity.actor_task_state(ActorTaskSlot::Primary).unwrap() {
                ActorTaskRuntime::ChaseTarget(task) => {
                    assert_eq!(class, 7);
                    assert_eq!(task.elapsed_ms(), frame_index * 20);
                    let Some(ActorTaskRuntime::AimAndFire(aim)) =
                        entity.actor_task_state(ActorTaskSlot::Tertiary)
                    else {
                        panic!("Search must publish its own Aim")
                    };
                    assert_eq!(aim.elapsed_ms(), (frame_index + 1) * 20);
                }
                ActorTaskRuntime::CapturePeoplePursuit(task) => {
                    assert_eq!(class, 9);
                    assert_eq!(task.elapsed_ms(), frame_index * 20);
                    assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
                }
                other => panic!("wrong pursuit graph: {other:?}"),
            }
        }
        assert_ne!(
            manager.entity_mut(id).unwrap().position_raw(),
            initial_position
        );
    }
}

#[v2k_test_support::retail_test]
fn native_type16_missing_own_first_query_receipt_retains_prefix_without_replaying_rng_or_age() {
    let Some((mut session, metadata)) = fixture() else {
        return;
    };
    let (mut manager, id) = isolated_birth(&session, &metadata, 5, 5, true);
    manager
        .entity_mut(id)
        .unwrap()
        .intro2_type16_runtime
        .as_mut()
        .unwrap()
        .sub_d_owner = Type9SubDFrameOwner::pending_constructor_origin(5);
    let owner = Intro2Type16Owner::adopt(&manager, id).unwrap();
    let mut fx = WorldFx::new();
    let tick = tick_intro2_type16(
        &mut manager,
        owner,
        Intro2Type16Frame {
            resources: &mut session.cache,
            world_fx: &mut fx,
            elapsed_micros: 20_000,
            retail_tick: 286,
        },
    );
    assert!(
        matches!(
            tick.outcome,
            Intro2Type16Outcome::Blocked {
                reason: Intro2Type16Block::SubDFirstQuery {
                    spawn_index: 5,
                    seed: 5
                },
                prefix_committed: true,
                ..
            }
        ),
        "{:?}",
        tick.outcome
    );
    let pending = tick.retained_owner.unwrap();
    assert!(pending.has_pending_prefix());
    let entity = manager.entity_mut(id).unwrap();
    let states =
        ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_task_state(slot).cloned());
    let collision = entity.collision.clone();
    let runtime = entity.intro2_type16_runtime;
    let position = entity.position_raw();
    let mut expected = fx.fork_for_main_base_abort_transaction();
    let tick = tick_intro2_type16(
        &mut manager,
        pending,
        Intro2Type16Frame {
            resources: &mut session.cache,
            world_fx: &mut fx,
            elapsed_micros: 999_000,
            retail_tick: 900,
        },
    );
    assert!(matches!(tick.outcome, Intro2Type16Outcome::Pending { .. }));
    assert!(tick.retained_owner.is_some());
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(
        ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_task_state(slot).cloned()),
        states
    );
    assert_eq!(entity.collision, collision);
    assert_eq!(entity.intro2_type16_runtime, runtime);
    assert_eq!(entity.position_raw(), position);
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
}
