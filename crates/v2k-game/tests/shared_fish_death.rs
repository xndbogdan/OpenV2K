//! Real aquatic cohorts retain native allocation/task custody through class2 death.

use v2k_game::{
    actor_task_owner::ActorTaskSlot,
    damage::{DamagePacket, FUN_0043F780_DAMAGE_PACKET},
    entity::{AuthoredWorldConstruction, EntityConstructionResources, EntityManager},
    entity_behavior::{ActiveBehaviorStyle, QUIET_DEATH_STYLE},
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue, DYING_STATE_BIT},
    gameplay_notifications::{GameplayNotificationPhase, GameplayNotifications},
    session::GameSession,
    shared_actor_impact::{
        apply_shared_actor_particle_hit, SharedActorImpactFrame, SharedActorImpactOutcome,
        SharedFishImpactOutcome,
    },
    shared_fish::SharedFishOutcome,
    specialized_actor_task_production::{
        SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
        SpecializedActorTaskScheduler,
    },
    static_damage::StaticDamageScheduler,
    world_fx::{BallisticDamageRequest, ParticleEntityImpact, WorldFx},
};

#[v2k_test_support::retail_test]
fn infected_then_primary_hits_preserve_every_authored_fish_death_policy() {
    let data = v2k_test_support::retail_dir();
    assert!(data.join("PRELOAD.DAT").is_file(), "retail corpus required");
    let mut session = GameSession::init(&data).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    // Preserve process construction history across both entry paths and all worlds.
    let mut fx = WorldFx::new();
    let mut census = Vec::new();
    for source_particle_class in [16, 5] {
        for world in [13, 14, 18, 22, 23, 30, 34, 36] {
            session.load_level_by_id(world, 1).unwrap();
            let rows: Vec<_> = session
                .cache
                .global_entity_model_table()
                .iter()
                .enumerate()
                .map(|(id, _)| {
                    EntityTypeRuntimeMetadata::from_section12(
                        session.cache.global_entity_type(id).unwrap(),
                    )
                })
                .collect();
            let mut manager = EntityManager::from_authored_world(
                AuthoredWorldConstruction {
                    logical_world_index: world as i32 - 12,
                    level: session.cache.level_desc().unwrap(),
                    type_metadata: &rows,
                    resources: EntityConstructionResources {
                        terrain: session.cache.terrain(),
                        terrain_objects: session.cache.terrain_objects(),
                        model_extent_raw: Some(&|id| {
                            session.cache.global_model(id).map(|m| m.radius)
                        }),
                    },
                    player_arrival: None,
                    retail_tick: 1000,
                },
                &mut fx,
            )
            .unwrap();
            // Finish the ordinary load's existing deferred removals before Playing.
            manager.cleanup_pending_actor_deferred_destroys();
            let victims: Vec<_> = manager
                .iter_all()
                .filter(|entity| matches!(entity.entity_type, 22 | 23 | 24 | 62))
                .map(|entity| entity.id)
                .collect();
            let survivors: Vec<_> = manager
                .iter_all()
                .filter(|entity| entity.entity_type == 124)
                .map(|entity| entity.id)
                .collect();
            assert!(!victims.is_empty());
            let mut scheduler = SpecializedActorTaskScheduler::new();
            assert_eq!(
                scheduler.adopt_shared_fish(&manager),
                victims.len() + survivors.len()
            );
            let mut damage = StaticDamageScheduler::new();
            let mut notifications = GameplayNotifications::new();
            // Exercise completed live custody, rather than only newborn task graphs.
            for tick in 1001..=1010 {
                let pass = scheduler.tick(
                    &mut manager,
                    SpecializedActorTaskProductionFrame {
                        world: v2k_game::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                        hive_components: None,
                        notification_phase: GameplayNotificationPhase::Playing,
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
                assert!(pass.block.is_none(), "world{world}: {pass:?}");
                assert!(
                    pass.outcomes.iter().all(|outcome| matches!(
                        outcome,
                        SpecializedActorTaskProductionOutcome::SharedFish(
                            SharedFishOutcome::Advanced { .. } | SharedFishOutcome::Waiting { .. }
                        )
                    )),
                    "world{world}: {pass:?}"
                );
            }
            for &id in &victims {
                let entity = manager.entity_mut(id).unwrap();
                let stamp = entity.construction_stamp_at_0xb4;
                let position_world = entity.position;
                let tick_before = entity.collision.last_hit_presentation_tick_at_0x34;
                let health_before = entity.collision.health_raw;
                let entries: &[u8] = if source_particle_class == 5 {
                    &[5, 16]
                } else {
                    &[16]
                };
                for &hit_class in entries {
                    let outcome = apply_shared_actor_particle_hit(
                        SharedActorImpactFrame {
                            resources: &session.cache,
                            entities: &mut manager,
                            world_fx: &mut fx,
                            scheduler: &mut scheduler,
                            notifications: &mut notifications,
                            retail_tick: 1011,
                        },
                        ParticleEntityImpact {
                            source_particle_class: hit_class,
                            impact_position_argument_va: if hit_class == 5 {
                                0x004D_CF48
                            } else {
                                0
                            },
                            target_entity_id: id,
                            position_world,
                            velocity_raw: [0, 0, 8192],
                            damage: Some(BallisticDamageRequest {
                                packet: if hit_class == 5 {
                                    FUN_0043F780_DAMAGE_PACKET
                                } else {
                                    DamagePacket {
                                        channels: [1, 0],
                                        amounts_raw: [100_000, 0],
                                    }
                                },
                                source_entity_type_at_birth: Some(46),
                                source_owner_id: Some(1),
                            }),
                        },
                    );
                    assert!(
                        matches!(
                            outcome,
                            Some(SharedActorImpactOutcome::Fish(
                                SharedFishImpactOutcome::Applied(_)
                            ))
                        ),
                        "world{world} id{id} class{source_particle_class}: {outcome:?}"
                    );
                    let entity = manager.entity_mut(id).unwrap();
                    assert_eq!(entity.construction_stamp_at_0xb4, stamp);
                    if hit_class == 5 {
                        assert_eq!(entity.collision.health_raw, health_before);
                        assert_eq!(
                            entity.collision.last_hit_presentation_tick_at_0x34,
                            tick_before
                        );
                        assert!(matches!(outcome, Some(SharedActorImpactOutcome::Fish(
                        SharedFishImpactOutcome::Applied(ref applied)
                    )) if applied.filtered_damage_raw == 0 && applied.death_publication.is_none()));
                        continue;
                    }
                    assert!(matches!(
                    entity.current_behavior_context,
                    RetailRuntimeValue::Known(Some(context))
                        if context.active_style() == ActiveBehaviorStyle::Audited(QUIET_DEATH_STYLE)
                ), "world{world} id{id} type{} class{source_particle_class}: {outcome:?}, health {:?}, context {:?}",
                    entity.entity_type, entity.collision.health_raw, entity.current_behavior_context);
                    assert_eq!(
                        entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT),
                        RetailRuntimeValue::Known(DYING_STATE_BIT)
                    );
                    assert_eq!(
                        entity.collision.last_hit_presentation_tick_at_0x34,
                        RetailRuntimeValue::Known(1011)
                    );
                    assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
                        .into_iter()
                        .all(|slot| entity.actor_task_state(slot).is_none()));
                }
            }
            assert_eq!(manager.pending_actor_deferred_destroy_ids(), victims);
            assert_eq!(manager.cleanup_pending_actor_deferred_destroys(), victims);
            assert!(victims
                .iter()
                .all(|id| manager.iter_all().all(|entity| entity.id != *id)));
            assert_eq!(scheduler.adopt_shared_fish(&manager), 0);
            let pass = scheduler.tick(
                &mut manager,
                SpecializedActorTaskProductionFrame {
                    world: v2k_game::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                    hive_components: None,
                    notification_phase: GameplayNotificationPhase::Playing,
                    resources: &mut session.cache,
                    world_fx: &mut fx,
                    static_damage: &mut damage,
                    elapsed_micros: 20_000,
                    global_elapsed_micros: 20_000,
                    retail_tick: 1012,
                    main_base_abort_active: false,
                },
                &mut notifications,
            );
            assert!(pass.block.is_none(), "world{world}: {pass:?}");
            assert_eq!(
                pass.outcomes.len(),
                survivors.len(),
                "world{world}: {pass:?}"
            );
            assert!(
                pass.outcomes.iter().all(|outcome| matches!(
                    outcome,
                    SpecializedActorTaskProductionOutcome::SharedFish(
                        SharedFishOutcome::Advanced { .. } | SharedFishOutcome::Waiting { .. }
                    )
                )),
                "world{world}: {pass:?}"
            );
            census.push((source_particle_class, world, victims.len(), survivors.len()));
            fx.clear();
        }
    }
    eprintln!("fish death (entry, world, class2, class63) census: {census:?}");
    assert_eq!(census.iter().map(|row| row.2 + row.3).sum::<usize>(), 228);
}
