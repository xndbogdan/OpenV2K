//! Ordinary authored Type58 construction and allocation custody independent
//! of the one accepted Intro2 replay allocation.

use super::native::tests::{fixture, generic};
use super::*;
use crate::{
    actor_task_owner::ActorTaskSlot,
    common_mover::{
        sub_d::{construct_native_sub_d, NativeSubDConstruction, SubDAllocationCounter},
        type9_attitude::Type9BodyBasis,
    },
    entity::{
        AuthoredWorldConstruction, EntityConstructionResources, EntityManager, Intro2BirthSelection,
    },
    entity_collision_state::RetailStateWord,
    gameplay_notifications::GameplayNotifications,
    session::GameSession,
    specialized_actor_task_production::{
        SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
        SpecializedActorTaskScheduler,
    },
    static_damage::StaticDamageScheduler,
    world_fx::WorldFx,
};

fn construct(
    session: &GameSession,
    metadata: &[EntityTypeRuntimeMetadata],
    level: u32,
    fx: &mut WorldFx,
) -> EntityManager {
    let model_extent = |id| session.cache.global_model(id).map(|model| model.radius);
    let resources = EntityConstructionResources {
        terrain: session.cache.terrain(),
        terrain_objects: session.cache.terrain_objects(),
        model_extent_raw: Some(&model_extent),
    };
    if level == 50 {
        EntityManager::from_native_intro2_frontend(
            session.cache.level_desc().unwrap(),
            metadata,
            resources,
            4793,
            fx,
        )
        .unwrap()
    } else {
        EntityManager::from_authored_world(
            AuthoredWorldConstruction {
                level: session.cache.level_desc().unwrap(),
                logical_world_index: level as i32 - 12,
                type_metadata: metadata,
                resources,
                player_arrival: None,
                retail_tick: 4793,
            },
            fx,
        )
        .unwrap()
    }
}

fn check_births(
    session: &GameSession,
    metadata: &[EntityTypeRuntimeMetadata],
    manager: &EntityManager,
    mut seed: u8,
    native_intro: bool,
) -> usize {
    if !native_intro
        && manager.player().is_some()
        && matches!(
            metadata[46].sub_d_steering_descriptor,
            RetailRuntimeValue::Known(Some(_))
        )
    {
        seed = seed.wrapping_add(1);
    }
    let mut count = 0;
    for spawn in &session.cache.level_desc().unwrap().entities {
        if spawn.entity_type == 58 {
            let entity = manager
                .iter_all()
                .find(|entity| entity.authored_spawn_index == Some(spawn.index))
                .unwrap();
            let runtime = entity
                .intro2_type58_runtime
                .expect("actual authored Type58 constructor receipt");
            assert!(matches!(
                runtime.origin,
                Type58ConstructionOrigin::Native(_)
            ));
            assert!(type58_manager_allocation_authenticates(manager, entity.id));
            assert_eq!(
                runtime.sub_d_owner.classifier_cache().stagger_counter(),
                seed,
                "actual process allocation for spawn{}",
                spawn.index
            );
            assert_eq!(
                runtime.sub_d_owner.classifier_cache().origin(),
                RetailRuntimeValue::Unresolved
            );
            assert_eq!(runtime.sub_d_owner.classifier_cache().rows(), [0; 8]);
            assert!(runtime.sub_d_owner.classifier_cache().can_classify());
            assert_eq!(runtime.sub_d_runtime, Type9SubDRuntime::from_constructor());
            assert_eq!(entity.model_slots, [Some(MODEL); 4]);
            assert_eq!(entity.position_raw(), runtime.anchor_raw);
            assert_eq!(
                runtime.anchor_raw,
                [
                    spawn.position_raw()[0],
                    session
                        .cache
                        .terrain()
                        .unwrap()
                        .bilinear_height_raw(spawn.position_raw()[0], spawn.position_raw()[2])
                        .wrapping_add(50),
                    spawn.position_raw()[2]
                ]
            );
            let rotation = spawn.rotation.map(|word| word as i16);
            assert_eq!(entity.rotation_heading_pitch_roll_raw(), rotation);
            assert_eq!(
                entity.physical_body_basis_q31(),
                RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(
                    rotation[0],
                    rotation[1],
                    rotation[2]
                ))
            );
            assert_eq!(
                entity.collision.animation_offset_at_0xb2,
                RetailRuntimeValue::Known(0)
            );
            assert_eq!(
                entity.collision.pre_health_damage_buffer_raw,
                RetailRuntimeValue::Known(spawn.initial_damage_buffer_raw)
            );
            assert!(Intro2Type58Owner::adopt(manager, entity.id).is_ok());
            assert!(
                matches!(entity.current_behavior_context, RetailRuntimeValue::Known(Some(context))
                if matches!(context.descriptor(), crate::entity_behavior::BehaviorDescriptorIdentity::Named(program)
                    if matches!(program.class_id, 7 | 26 | 33)))
            );
            count += 1;
        }
        if matches!(
            metadata[spawn.entity_type as usize].sub_d_steering_descriptor,
            RetailRuntimeValue::Known(Some(_))
        ) {
            seed = seed.wrapping_add(1);
        }
    }
    count
}

#[v2k_test_support::retail_test]
fn every_ordinary_type58_retains_authored_birth_and_actual_process_sub_d() {
    let (mut session, metadata) = fixture().expect("normal-tier retail corpus required");
    let mut fx = WorldFx::new();
    let mut census = Vec::new();
    for level in 13..=49 {
        session.load_level_by_id(level, 1).unwrap();
        let count = session
            .cache
            .level_desc()
            .unwrap()
            .entities
            .iter()
            .filter(|spawn| spawn.entity_type == 58)
            .count();
        if count == 0 {
            continue;
        }
        let seed = fx.next_sub_d_allocation_seed();
        let manager = construct(&session, &metadata, level, &mut fx);
        assert_eq!(
            check_births(&session, &metadata, &manager, seed, false),
            count
        );
        let mut scheduler = SpecializedActorTaskScheduler::default();
        assert_eq!(scheduler.adopt_intro2_type58(&manager), count);
        assert_eq!(scheduler.adopt_intro2_type58(&manager), 0);
        census.push((level, count));
    }
    println!("canonical ordinary Type58 census: {census:?}");
    assert_eq!(census, [(14, 4), (24, 2), (31, 1)]);
}

#[v2k_test_support::retail_test]
fn native_intro_type58_uses_process_history_while_replay_keeps_captured_seed() {
    let (mut session, metadata) = fixture().expect("normal-tier retail corpus required");
    let mut fx = WorldFx::new();
    session.load_level_by_id(14, 1).unwrap();
    let _earlier = construct(&session, &metadata, 14, &mut fx);
    let seed = fx.next_sub_d_allocation_seed();
    session.load_level_by_id(50, 1).unwrap();
    let native = construct(&session, &metadata, 50, &mut fx);
    assert_eq!(check_births(&session, &metadata, &native, seed, true), 1);
    let runtime = native
        .iter_all()
        .find(|entity| entity.entity_type == 58)
        .unwrap()
        .intro2_type58_runtime
        .unwrap();
    assert_ne!(
        runtime.sub_d_owner.classifier_cache().stagger_counter(),
        0x19
    );
    let replay = EntityManager::from_captured_intro2_frontend_with_type_metadata(
        session.cache.level_desc().unwrap(),
        &metadata,
        EntityConstructionResources {
            terrain: session.cache.terrain(),
            terrain_objects: session.cache.terrain_objects(),
            model_extent_raw: Some(&|id| session.cache.global_model(id).map(|model| model.radius)),
        },
        Intro2BirthSelection::default(),
        &mut || u32::from(fx.next_shared_retail_random_u16()),
    )
    .unwrap();
    let runtime = replay
        .iter_all()
        .find(|entity| entity.entity_type == 58)
        .unwrap()
        .intro2_type58_runtime
        .unwrap();
    assert!(matches!(
        runtime.origin,
        Type58ConstructionOrigin::CapturedIntro2
    ));
    assert_eq!(
        runtime.sub_d_owner.classifier_cache().stagger_counter(),
        0x19
    );
}

fn component_allocation(metadata: &EntityTypeRuntimeMetadata) -> NativeSubDConstruction {
    let RetailRuntimeValue::Known(Some(descriptor)) = metadata.sub_d_steering_descriptor else {
        panic!("canonical Sub-D");
    };
    construct_native_sub_d(&mut SubDAllocationCounter::from_next_seed(0xd3), descriptor)
}

#[v2k_test_support::retail_test]
fn shared_type58_preserves_authored_bits_and_selects_unindexed_player_prefix() {
    let (session, metadata) = fixture().expect("normal-tier retail corpus required");
    let mut spawn = session.cache.level_desc().unwrap().entities[40].clone();
    spawn.index = 99;
    spawn.rotation = [0x9000, 0x0800, 0xfc00];
    let x = spawn.position_raw()[0].wrapping_add(19);
    let z = spawn.position_raw()[2].wrapping_sub(37);
    spawn.pos_data_1[..2].copy_from_slice(&x.to_le_bytes());
    spawn.pos_data_2[..2].copy_from_slice(&z.to_le_bytes());
    let mut terrain = TerrainGrid {
        header: session.cache.terrain().unwrap().header,
        cells: session.cache.terrain().unwrap().cells.clone(),
    };
    for cell in &mut terrain.cells {
        cell.attribute = 0;
    }
    for (param, player, furniture, class, choice, draws, final_word) in [
        (0, false, false, 33, 0, 4, 0x9876),
        (1, true, false, 7, 2, 4, 0x9876),
        (0x8000_0000, false, true, 26, 1, 3, 0x1234),
    ] {
        spawn.param = param;
        let first_x = usize::from(x.wrapping_sub(256) as u16 >> 8);
        let first_z = usize::from(z.wrapping_sub(256) as u16 >> 8);
        let first_furniture_cell = &mut terrain.cells[first_x * 256 + first_z];
        first_furniture_cell.attribute = u8::from(furniture);
        first_furniture_cell.terrain_type &= !8;
        let mut manager = generic(&session, &metadata);
        let allocation = manager.main_base_abort_actor_observation(41).unwrap().lease;
        let mut prefix = generic(&session, &metadata);
        let candidate = prefix.entity_mut(1).unwrap();
        candidate.authored_spawn_index = None;
        candidate.capability_flags = u32::from(player);
        candidate.collision.state_flags_at_0x08 = RetailStateWord::exact(4);
        candidate.set_position_raw([x, terrain.bilinear_height_raw(x, z).wrapping_add(50), z]);
        let entity = manager.entity_mut(41).unwrap();
        entity.authored_spawn_index = Some(spawn.index);
        entity.set_rotation_heading_pitch_roll_raw(spawn.rotation.map(|word| word as i16));
        let sub_d = component_allocation(&metadata[58]);
        let mut words = [0x5800, 0xffff, 0x1234, 0x9876].into_iter().take(draws);
        let publication = publish_authored_type58(
            Type58AuthoredConstructionRequest {
                entity,
                allocation,
                metadata: &metadata[58],
                spawn: &spawn,
                preceding: std::slice::from_ref(candidate),
                resources: EntityConstructionResources::new(
                    Some(&terrain),
                    session.cache.terrain_objects(),
                ),
                constructor_surface_bits: 0,
                sub_d,
            },
            &mut || words.next().expect("A, selector, two task suffixes"),
        )
        .unwrap();
        assert!(words.next().is_none());
        assert_eq!(
            (
                publication.selection.program.class_id,
                publication.selection.choice_index
            ),
            (class, choice)
        );
        assert_eq!(publication.player_nearby, player);
        assert_eq!(publication.furniture_nearby, furniture);
        assert!(!publication.initializer_fallback);
        let entity = manager.entity_mut(41).unwrap();
        let runtime = entity.intro2_type58_runtime.unwrap();
        assert_eq!(runtime.sub_d_owner, sub_d.frame_owner);
        assert_eq!(runtime.sub_d_runtime, sub_d.runtime);
        assert_eq!(runtime.anchor_raw, candidate.position_raw());
        assert_eq!(
            entity.collision.state_flags_at_0x08.masked(0x0100_0000),
            RetailRuntimeValue::Known(if param == 0 { 0 } else { 0x0100_0000 })
        );
        assert_eq!(
            entity.rotation_heading_pitch_roll_raw(),
            spawn.rotation.map(|word| word as i16)
        );
        let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
            panic!("constructed A");
        };
        assert_eq!(
            sub_a.target_speed_raw(),
            RetailRuntimeValue::Known(crate::common_mover::shared_initializer_target_speed_raw(
                300, final_word
            ))
        );
        assert!(type58_manager_allocation_authenticates(&manager, 41));
    }
}

#[v2k_test_support::retail_test]
fn shared_type58_rejects_invalid_metadata_and_prefix_before_rng_or_publication() {
    let (session, metadata) = fixture().expect("normal-tier retail corpus required");
    let spawn = &session.cache.level_desc().unwrap().entities[40];
    for invalid in 0..3 {
        let mut manager = generic(&session, &metadata);
        let allocation = manager.main_base_abort_actor_observation(41).unwrap().lease;
        let mut prefix = generic(&session, &metadata);
        let candidate = prefix.entity_mut(1).unwrap();
        candidate.authored_spawn_index = Some(spawn.index + 1);
        let mut profile = metadata[58].clone();
        if invalid == 0 {
            let RetailRuntimeValue::Known(Some(mut emitter)) =
                profile.projectile_emitter_descriptor
            else {
                panic!("canonical E");
            };
            emitter.variable_bindings[0] = 150;
            profile.projectile_emitter_descriptor = RetailRuntimeValue::Known(Some(emitter));
        }
        let entity = manager.entity_mut(41).unwrap();
        if invalid == 2 {
            entity.actor_common_axis_descriptor = RetailRuntimeValue::Unresolved;
        }
        let before = (
            entity.position_raw(),
            entity.collision.state_flags_at_0x08,
            entity.sub_a_propulsion_runtime,
            entity.sub_h_external_frame_runtime.clone(),
        );
        let mut draws = 0;
        let result = publish_authored_type58(
            Type58AuthoredConstructionRequest {
                entity,
                allocation,
                metadata: &profile,
                spawn,
                preceding: if invalid == 1 {
                    std::slice::from_ref(candidate)
                } else {
                    &[]
                },
                resources: EntityConstructionResources::new(
                    session.cache.terrain(),
                    session.cache.terrain_objects(),
                ),
                constructor_surface_bits: 0,
                sub_d: component_allocation(&metadata[58]),
            },
            &mut || {
                draws += 1;
                0x4321
            },
        );
        assert_eq!(
            result,
            Err(match invalid {
                0 => Intro2Type58Error::Metadata,
                1 => Intro2Type58Error::Prefix,
                _ => Intro2Type58Error::ComponentStorage,
            })
        );
        assert_eq!(draws, 0);
        assert_eq!(
            (
                entity.position_raw(),
                entity.collision.state_flags_at_0x08,
                entity.sub_a_propulsion_runtime,
                entity.sub_h_external_frame_runtime.clone()
            ),
            before
        );
        assert!(entity.intro2_type58_runtime.is_none());
        assert_eq!(entity.initial_behavior, RetailRuntimeValue::Unresolved);
        assert_eq!(
            entity.current_behavior_context,
            RetailRuntimeValue::Unresolved
        );
        assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .all(|slot| entity.actor_task_state(slot).is_none()));
    }
}

#[v2k_test_support::retail_test]
fn native_type58_foreign_manager_receipt_cannot_be_adopted_or_publish_class12() {
    let (mut session, metadata) = fixture().expect("normal-tier retail corpus required");
    session.load_level_by_id(14, 1).unwrap();
    let mut fx = WorldFx::new();
    let mut first = construct(&session, &metadata, 14, &mut fx);
    let mut second = construct(&session, &metadata, 14, &mut fx);
    let id = first
        .iter_all()
        .find(|entity| entity.entity_type == 58)
        .unwrap()
        .id;
    std::mem::swap(
        first.entity_mut(id).unwrap(),
        second.entity_mut(id).unwrap(),
    );
    assert!(intro2_type58_allocation_authenticates(
        second.entity_mut(id).unwrap()
    ));
    assert!(!type58_manager_allocation_authenticates(&second, id));
    assert_eq!(
        Intro2Type58Owner::adopt(&second, id),
        Err(Intro2Type58Block::Allocation)
    );
    let health = second.entity_mut(id).unwrap().collision.health_raw;
    assert!(
        crate::intro2_common_dying::publish_intro2_common_standard_death(&mut second, id, &mut fx)
            .is_err()
    );
    assert_eq!(second.entity_mut(id).unwrap().collision.health_raw, health);
}

#[v2k_test_support::retail_test]
fn ordinary_type58_runs_scheduler_frames_with_actual_component_custody() {
    let (mut session, metadata) = fixture().expect("normal-tier retail corpus required");
    let mut fx = WorldFx::new();
    for level in [14, 24, 31] {
        session.load_level_by_id(level, 1).unwrap();
        let mut manager = construct(&session, &metadata, level, &mut fx);
        let ids: Vec<_> = manager
            .iter_all()
            .filter(|entity| entity.entity_type == 58)
            .map(|entity| entity.id)
            .collect();
        assert!(!ids.is_empty());
        let before: Vec<_> = ids
            .iter()
            .map(|&id| manager.entity_mut(id).unwrap().position_raw())
            .collect();
        let mut scheduler = SpecializedActorTaskScheduler::default();
        assert_eq!(scheduler.adopt_intro2_type58(&manager), ids.len());
        let mut notifications = GameplayNotifications::new();
        let mut static_damage = StaticDamageScheduler::new();
        let mut elapsed = 0;
        let mut callbacks = 0;
        for index in 0..60 {
            let dt = [16_667, 33_333, 20_000, 50_000, 1_000, 125_000][index % 6];
            fx.advance_frame_pacing(dt);
            let pass = scheduler.tick(
                &mut manager,
                SpecializedActorTaskProductionFrame {
                    world: crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                    hive_components: None,
                    notification_phase:
                        crate::gameplay_notifications::GameplayNotificationPhase::Playing,
                    resources: &mut session.cache,
                    world_fx: &mut fx,
                    static_damage: &mut static_damage,
                    elapsed_micros: dt,
                    global_elapsed_micros: dt,
                    retail_tick: 4794 + elapsed / 20_000,
                    main_base_abort_active: false,
                },
                &mut notifications,
            );
            assert!(pass.block.is_none(), "world{level}: {pass:?}");
            for outcome in pass.outcomes {
                match outcome {
                    SpecializedActorTaskProductionOutcome::Intro2Type58(
                        Intro2Type58Outcome::Advanced {
                            callback_enabled, ..
                        },
                    ) => callbacks += usize::from(callback_enabled),
                    SpecializedActorTaskProductionOutcome::Intro2Type58(
                        Intro2Type58Outcome::Waiting { .. },
                    ) => {}
                    other => panic!("world{level}, frame{index}: {other:?}"),
                }
            }
            fx.process_pending();
            elapsed += dt;
        }
        assert!(callbacks > 0, "world{level}");
        assert!(
            ids.iter().zip(before).any(|(&id, position)| manager
                .entity_mut(id)
                .unwrap()
                .position_raw()
                != position),
            "world{level}"
        );
        for id in ids {
            assert!(type58_manager_allocation_authenticates(&manager, id));
            assert!(Intro2Type58Owner::adopt(&manager, id).is_ok());
        }
    }
}
