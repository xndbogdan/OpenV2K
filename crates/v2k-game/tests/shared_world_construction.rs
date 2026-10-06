use v2k_game::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    campaign_transition::collect_authored_warp_markers,
    damage::PRIMARY_PROJECTILE_DAMAGE_PACKET,
    entity::{
        AuthoredPlayerArrival, AuthoredWorldConstruction, Class0ActorOwner,
        EntityConstructionResources, EntityManager,
    },
    entity_collision_state::{
        EntityTypeRuntimeMetadata, RetailRuntimeValue, FULLY_ABOVE_SURFACE_STATE_BIT,
        FULLY_BELOW_SURFACE_STATE_BIT, SURFACE_STATE_MASK,
    },
    intro2_type66::impact::Intro2Type66ImpactOutcome,
    main_base_type54_abort::MainBaseType54SeaDeltaSource,
    session::GameSession,
    shared_actor_impact::{
        apply_shared_actor_particle_hit, SharedActorImpactFrame, SharedActorImpactOutcome,
    },
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    world_fx::{BallisticDamageRequest, ParticleEntityImpact, WorldFx},
};

fn session() -> GameSession {
    let path = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&path).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session
}

fn metadata(session: &GameSession) -> Vec<EntityTypeRuntimeMetadata> {
    session
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
        .collect()
}

#[v2k_test_support::retail_test]
fn ordinary_corpus_constructs_shared_families_and_preserves_process_seeds() {
    let mut session = session();
    let mut fx = WorldFx::new();
    let mut expected_seed = 0u8;
    let mut counts = [0; 7];
    let mut type62_count = 0;
    let mut wet_class0_counts = [0; 2];
    let mut wave_disabled_wet_weights = Vec::new();
    const CONSTRUCTOR_TICK: u32 = 4793;
    for level_id in 13..=49 {
        session.load_level_by_id(level_id, 1).unwrap();
        let rows = metadata(&session);
        let level = session.cache.level_desc().unwrap();
        // Same explicit persistent controller pose used by direct production.
        let arrival = AuthoredPlayerArrival {
            position_raw: [19_712, -500, 14_848],
            heading_raw: 0x4000,
        };
        let manager = EntityManager::from_authored_world(
            AuthoredWorldConstruction {
                logical_world_index: (level_id - 12) as i32,
                level,
                type_metadata: &rows,
                resources: EntityConstructionResources {
                    terrain: session.cache.terrain(),
                    terrain_objects: session.cache.terrain_objects(),
                    model_extent_raw: Some(&|id| {
                        session.cache.global_model(id).map(|model| model.radius)
                    }),
                },
                player_arrival: Some(arrival),
                retail_tick: CONSTRUCTOR_TICK,
            },
            &mut fx,
        )
        .unwrap_or_else(|error| panic!("world{level_id}: {error:?}"));
        let stamp_base = ((level_id - 12) << 10) as u16;
        // Every 33BD0 marker construction consumes a body ordinal, including
        // helpers that 42EFB0 subsequently marks for deferred destruction.
        let marker_attempts = collect_authored_warp_markers(
            session.cache.terrain().unwrap(),
            session.cache.terrain_objects().unwrap(),
        )
        .unwrap()
        .len();
        assert_eq!(
            manager.player().unwrap().construction_stamp_at_0xb4,
            RetailRuntimeValue::Known(stamp_base)
        );
        assert_eq!(
            manager.next_common_body_ordinal(),
            RetailRuntimeValue::Known((level.entities.len() + marker_attempts + 1) as u16)
        );
        for (ordinal, entity) in manager
            .iter_all()
            .filter(|entity| entity.authored_spawn_index.is_some())
            .enumerate()
        {
            assert_eq!(
                entity.construction_stamp_at_0xb4,
                RetailRuntimeValue::Known(
                    stamp_base.wrapping_add((ordinal + marker_attempts + 1) as u16)
                ),
                "world{level_id}, authored birth{ordinal}, type{}",
                entity.entity_type
            );
        }
        assert_eq!(
            manager.retail_live_order_ids().next(),
            manager.player().map(|e| e.id)
        );
        let allocations = level
            .entities
            .iter()
            .map(|spawn| spawn.entity_type)
            .chain([46])
            .filter(|&ty| {
                matches!(
                    rows[ty as usize].sub_d_steering_descriptor,
                    RetailRuntimeValue::Known(Some(_))
                )
            })
            .count();
        expected_seed = expected_seed.wrapping_add(allocations as u8);
        assert_eq!(
            fx.next_sub_d_allocation_seed(),
            expected_seed,
            "world{level_id}"
        );
        for entity in manager.iter_all() {
            let family = match entity.entity_type {
                62 => {
                    type62_count += 1;
                    assert!(
                        matches!(
                            entity.current_behavior_context,
                            RetailRuntimeValue::Known(Some(_))
                        ),
                        "world{level_id} Type62 spawn{:?}",
                        entity.authored_spawn_index
                    );
                    assert!(
                        matches!(
                            entity.actor_task_state(ActorTaskSlot::Primary),
                            Some(ActorTaskRuntime::OrdinaryType9Wander(_))
                        ),
                        "world{level_id} Type62 spawn{:?}",
                        entity.authored_spawn_index
                    );
                    assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
                    assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
                    continue;
                }
                6 => 0,
                8 => 1,
                9 => 2,
                66 => 3,
                52 => 4,
                68 => 5,
                54 => 6,
                _ => continue,
            };
            counts[family] += 1;
            assert!(
                matches!(
                    entity.current_behavior_context,
                    RetailRuntimeValue::Known(Some(_))
                ),
                "world{level_id}, type{}, spawn{:?}",
                entity.entity_type,
                entity.authored_spawn_index
            );
            assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
                .into_iter()
                .any(|slot| entity.actor_task_state(slot).is_some()));
            if matches!(entity.entity_type, 52 | 54 | 68) {
                let spawn = level
                    .entities
                    .iter()
                    .find(|spawn| Some(spawn.index) == entity.authored_spawn_index)
                    .unwrap();
                let terrain = session.cache.terrain().unwrap();
                let [x, authored_y, z] = spawn.position_raw();
                let cell = terrain
                    .cell(usize::from((x as u16) >> 8), usize::from((z as u16) >> 8))
                    .unwrap();
                let active_slot = if cell.terrain_type & 0x10 != 0 { 2 } else { 0 };
                assert_eq!(
                    entity.collision.active_model_slot(),
                    RetailRuntimeValue::Known(active_slot)
                );
                let expected_model = match entity.entity_type {
                    52 => 145,
                    54 => 560,
                    _ => 81,
                };
                assert_eq!(entity.model_slots, [Some(expected_model); 4]);
                assert_eq!(entity.model_index, Some(expected_model));
                let extent = if entity.entity_type == 52 { 200 } else { 0 };
                assert_eq!(
                    entity.position_raw(),
                    [x, terrain.bilinear_height_raw(x, z) + extent, z],
                    "world{level_id} spawn{}",
                    spawn.index
                );
                let floor = i16::from(cell.height as i8) * 32;
                let sea = terrain.sea_level_raw();
                let waves_enabled = level.raw_u32(0x84).unwrap() != 0;
                if matches!(entity.entity_type, 52 | 68) && floor < sea {
                    wet_class0_counts[usize::from(entity.entity_type == 68)] += 1;
                    if entity.entity_type == 68 && !waves_enabled {
                        wave_disabled_wet_weights.push(level_id);
                    }
                }
                let surface = if waves_enabled && floor < sea {
                    v2k_formats::terrain::wave_surface_raw(
                        x,
                        z,
                        CONSTRUCTOR_TICK as i32,
                        sea,
                        floor,
                    )
                } else {
                    sea
                };
                let expected_surface = match authored_y.cmp(&surface) {
                    std::cmp::Ordering::Less => FULLY_BELOW_SURFACE_STATE_BIT,
                    std::cmp::Ordering::Equal => 0,
                    std::cmp::Ordering::Greater => FULLY_ABOVE_SURFACE_STATE_BIT,
                };
                assert_eq!(
                    entity
                        .collision
                        .state_flags_at_0x08
                        .masked(SURFACE_STATE_MASK),
                    RetailRuntimeValue::Known(expected_surface),
                    "pre-grounding surface, world{level_id} spawn{}",
                    spawn.index
                );
                assert!(
                    matches!(entity.actor_task_state(ActorTaskSlot::Primary), Some(ActorTaskRuntime::Class0Timer(task)) if task.elapsed_ms() == 0)
                );
                assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
                assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
                assert_eq!(
                    Class0ActorOwner::adopt(&manager, entity.id)
                        .unwrap()
                        .entity_id(),
                    entity.id
                );
                if entity.entity_type == 54 {
                    assert_eq!(
                        entity.main_base_type54_sea_delta_source,
                        RetailRuntimeValue::Known(
                            MainBaseType54SeaDeltaSource::DeriveFromActorModelAndSea
                        )
                    );
                }
            }
        }
        let expected_class0 = manager
            .iter_all()
            // Native Type111 helpers also own Class0. A helper marked by
            // 42EFB0 retains its stamp and graph custody until the 14990 sweep;
            // count every authenticated owner still present in the manager.
            .filter(|entity| Class0ActorOwner::adopt(&manager, entity.id).is_ok())
            .count();
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert_eq!(scheduler.adopt_class0_actors(&manager), expected_class0);
        assert_eq!(
            scheduler.adopt_class0_actors(&manager),
            0,
            "duplicate adoption must retain one owner"
        );
        drop(manager);
        fx.clear();
        assert_eq!(
            fx.next_sub_d_allocation_seed(),
            expected_seed,
            "world teardown retains process history"
        );
    }
    assert_eq!(counts, [28, 7, 42, 81, 157, 44, 1]);
    assert_eq!(type62_count, 21);
    assert_eq!(wet_class0_counts, [31, 7]);
    assert_eq!(wave_disabled_wet_weights, [23, 30, 34]);
}

#[v2k_test_support::retail_test]
fn later_factory_public_primary_hits_reach_progressive_death() {
    let mut session = session();
    for level_id in [13, 14, 15, 18, 19, 43] {
        session.load_level_by_id(level_id, 1).unwrap();
        let rows = metadata(&session);
        let mut fx = WorldFx::new();
        let mut manager = EntityManager::from_authored_world(
            AuthoredWorldConstruction {
                logical_world_index: (level_id - 12) as i32,
                level: session.cache.level_desc().unwrap(),
                type_metadata: &rows,
                resources: EntityConstructionResources {
                    terrain: session.cache.terrain(),
                    terrain_objects: session.cache.terrain_objects(),
                    model_extent_raw: Some(&|id| {
                        session.cache.global_model(id).map(|model| model.radius)
                    }),
                },
                player_arrival: None,
                retail_tick: 0,
            },
            &mut fx,
        )
        .unwrap();
        let ids: Vec<_> = manager
            .iter_all()
            .filter(|e| e.entity_type == 66)
            .map(|e| e.id)
            .collect();
        assert!(!ids.is_empty());
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert_eq!(scheduler.adopt_intro2_type66(&manager), ids.len());
        for id in ids {
            for hit in 1..=56 {
                let result = apply_shared_actor_particle_hit(
                    SharedActorImpactFrame {
                        resources: &session.cache,
                        entities: &mut manager,
                        world_fx: &mut fx,
                        scheduler: &mut scheduler,
                        notifications:
                            &mut v2k_game::gameplay_notifications::GameplayNotifications::new(),
                        retail_tick: hit,
                    },
                    ParticleEntityImpact {
                        source_particle_class: 16,
                        impact_position_argument_va: 0,
                        target_entity_id: id,
                        position_world: [0.0; 3],
                        velocity_raw: [0, 0, 8192],
                        damage: Some(BallisticDamageRequest {
                            packet: PRIMARY_PROJECTILE_DAMAGE_PACKET,
                            source_entity_type_at_birth: Some(46),
                            source_owner_id: Some(1),
                        }),
                    },
                );
                let Some(SharedActorImpactOutcome::Factory(Intro2Type66ImpactOutcome::Applied(
                    applied,
                ))) = result
                else {
                    panic!("world{level_id} actor{id} hit{hit}: {result:?}");
                };
                assert_eq!(applied.filtered_damage_raw, 1800);
                assert_eq!(applied.death_publication.is_some(), hit == 56);
            }
            let entity = manager.iter_all().find(|e| e.id == id).unwrap();
            assert_eq!(
                entity.collision.health_raw,
                RetailRuntimeValue::Known(10_000_000)
            );
            assert!(
                matches!(entity.base_factory_runtime, RetailRuntimeValue::Known(Some(base)) if base.progressive_death.elapsed_micros_raw == 1)
            );
        }
    }
}
