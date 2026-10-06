use super::*;
use crate::{
    damage::DamagePacket,
    entity::{AuthoredWorldConstruction, EntityConstructionResources, EntityManager},
    entity_scheduler::{
        COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT, SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
    },
    intro2_common_dying::Intro2CommonDyingOwner,
    session::GameSession,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    world_fx::{BallisticDamageRequest, ParticleEntityImpact},
};

fn session() -> GameSession {
    let data = v2k_test_support::retail_dir();
    assert!(data.join("PRELOAD.DAT").is_file(), "retail corpus required");
    let mut session = GameSession::init(&data).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session
}

fn construct(session: &GameSession, world: u32, fx: &mut WorldFx) -> EntityManager {
    let rows: Vec<_> = session
        .cache
        .global_entity_model_table()
        .iter()
        .enumerate()
        .map(|(id, _)| {
            EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(id).unwrap())
        })
        .collect();
    EntityManager::from_authored_world(
        AuthoredWorldConstruction {
            level: session.cache.level_desc().unwrap(),
            logical_world_index: world as i32 - 12,
            type_metadata: &rows,
            resources: EntityConstructionResources {
                terrain: session.cache.terrain(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&|id| {
                    session.cache.global_model(id).map(|model| model.radius)
                }),
            },
            player_arrival: None,
            retail_tick: 1000,
        },
        fx,
    )
    .unwrap_or_else(|e| panic!("world{world}: {e:?}"))
}

fn ids(manager: &EntityManager) -> Vec<u32> {
    manager
        .iter_all()
        .filter(|e| e.entity_type == 26)
        .map(|e| e.id)
        .collect()
}

fn primary_impact(id: u32) -> ParticleEntityImpact {
    ParticleEntityImpact {
        source_particle_class: 1,
        impact_position_argument_va: 0,
        target_entity_id: id,
        position_world: [0.0; 3],
        velocity_raw: [0, 0, 8192],
        damage: Some(BallisticDamageRequest {
            packet: DamagePacket {
                channels: [2, 0],
                amounts_raw: [2000, 0],
            },
            source_entity_type_at_birth: Some(34),
            source_owner_id: Some(35),
        }),
    }
}

#[v2k_test_support::retail_test]
fn authored_type26_all_campaign_births_enter_native_tasks_and_world_motion() {
    let mut session = session();
    let mut census = Vec::new();
    let mut fx = WorldFx::new();
    for world in 13..=49 {
        session.load_level_by_id(world, 1).unwrap();
        let count = session
            .cache
            .level_desc()
            .unwrap()
            .entities
            .iter()
            .filter(|e| e.entity_type == 26)
            .count();
        if count == 0 {
            continue;
        }
        let mut manager = construct(&session, world as u32, &mut fx);
        let actors = ids(&manager);
        assert_eq!(actors.len(), count);
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert_eq!(scheduler.adopt_intro2_type26(&manager), count);
        for id in actors {
            let entity = manager.entity_mut(id).unwrap();
            assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(5000));
            let receipt = entity
                .native_type26_allocation
                .expect("ordinary allocation receipt");
            assert_eq!(Some(receipt.spawn_index), entity.authored_spawn_index);
            assert!(entity.intro2_type26_sub_d_frame_owner.is_some());
            assert!(
                matches!(entity.initial_behavior, RetailRuntimeValue::Known(Some(selection)) if matches!(selection.program.class_id, 4 | 26 | 33))
            );
            assert!(entity.actor_task_state(ActorTaskSlot::Primary).is_some());
            let authored =
                session.cache.level_desc().unwrap().entities[receipt.spawn_index].position_raw();
            assert_eq!(
                entity.position_raw(),
                [
                    authored[0],
                    session
                        .cache
                        .terrain()
                        .unwrap()
                        .bilinear_height_raw(authored[0], authored[2]),
                    authored[2]
                ]
            );
            // Exercise the detailed callback rather than a parked coarse frame.
            entity.collision.state_flags_at_0x08.overwrite(
                COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT
                    | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT
                    | 0x0204_0000,
                COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT
                    | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT
                    | 0x0204_0000,
            );
            let mut owner = Intro2Type26WorldOwner::adopt(&manager, id).unwrap();
            let before = manager.entity_mut(id).unwrap().position_raw();
            for tick in 1001..=1010 {
                let result = tick_intro2_type26_world(
                    &mut manager,
                    owner,
                    Intro2Type26WorldFrame {
                        resources: &mut session.cache,
                        world_fx: &mut fx,
                        elapsed_micros: 20_000,
                        global_elapsed_micros: 20_000,
                        retail_tick: tick,
                    },
                );
                assert!(
                    matches!(
                        result.outcome,
                        Intro2Type26WorldOutcome::Advanced { .. }
                            | Intro2Type26WorldOutcome::Waiting { .. }
                    ),
                    "world{world} id{id}: {:?}",
                    result.outcome
                );
                owner = result.retained_owner.unwrap();
            }
            let entity = manager.entity_mut(id).unwrap();
            assert_ne!(
                entity.position_raw(),
                before,
                "world{world} id{id} must run its native mover"
            );
            assert_eq!(
                entity.native_type26_allocation,
                Some(receipt),
                "task changes retain birth custody"
            );
        }
        census.push((world, count));
    }
    assert_eq!(census, [(15, 6), (18, 2)]);
}

#[v2k_test_support::retail_test]
fn authored_type26_castle_and_prehistoric_hits_bleed_die_and_keep_corpse_owner() {
    let mut session = session();
    for world in [15, 18] {
        session.load_level_by_id(world, 1).unwrap();
        let mut fx = WorldFx::new();
        for _ in 0..8 {
            fx.advance_frame_pacing(20_000);
        }
        let mut manager = construct(&session, world as u32, &mut fx);
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler.adopt_intro2_type26(&manager);
        for id in ids(&manager) {
            let entity = manager.entity_mut(id).unwrap();
            let receipt = entity.native_type26_allocation;
            entity.collision.health_raw = RetailRuntimeValue::Known(400);
            entity.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(0);
            let mut corpse_primary = None;
            for shot in 0..3 {
                let particles_before = fx.particle_count();
                let hit = apply_intro2_type26_particle_hit(
                    &mut manager,
                    &session.cache,
                    &mut fx,
                    &mut scheduler,
                    primary_impact(id),
                    2000 + shot,
                );
                let Intro2Type26ImpactOutcome::Applied(hit) = hit else {
                    panic!("world{world} id{id} shot{shot}: {hit:?}")
                };
                assert_eq!(hit.filtered_damage_raw, 200);
                assert_eq!(hit.death_publication.is_some(), shot == 1);
                assert!(
                    fx.particle_count() > particles_before,
                    "accepted primary hit emits infected blood, including the corpse"
                );
                let entity = manager.entity_mut(id).unwrap();
                assert_eq!(
                    entity.collision.last_hit_presentation_tick_at_0x34,
                    RetailRuntimeValue::Known(2000 + shot)
                );
                assert_eq!(
                    entity.collision.health_raw,
                    RetailRuntimeValue::Known(if shot == 0 { 200 } else { 0 })
                );
                assert_eq!(entity.native_type26_allocation, receipt);
                if shot == 1 {
                    corpse_primary = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
                }
                if shot == 2 {
                    assert_eq!(
                        entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
                        corpse_primary
                    );
                }
                if shot >= 1 {
                    assert!(Intro2CommonDyingOwner::adopt(&manager, id).is_ok());
                }
            }
        }
    }
}

#[v2k_test_support::retail_test]
fn authored_type26_foreign_allocation_receipt_rejects_before_any_hit_prefix() {
    let mut session = session();
    session.load_level_by_id(15, 1).unwrap();
    let mut fx = WorldFx::new();
    let mut first = construct(&session, 15, &mut fx);
    let mut second = construct(&session, 15, &mut fx);
    let id = ids(&first)[0];
    let second_actor = second.entity_mut(id).unwrap();
    let foreign = second_actor.native_type26_allocation;
    let second_seed = second_actor
        .intro2_type26_sub_d_frame_owner
        .unwrap()
        .classifier_cache()
        .stagger_counter();
    let entity = first.entity_mut(id).unwrap();
    assert_ne!(
        entity.intro2_type26_sub_d_frame_owner.unwrap().classifier_cache().stagger_counter(),
        second_seed,
        "a subsequent world consumes the process allocation stream instead of replaying an Intro2 seed"
    );
    let original = entity.native_type26_allocation;
    let tick = entity.collision.last_hit_presentation_tick_at_0x34;
    let health = entity.collision.health_raw;
    assert_ne!(
        original, foreign,
        "same local id in another world allocation is not custody"
    );
    entity.native_type26_allocation = foreign;
    assert!(Intro2Type26WorldOwner::adopt(&first, id).is_err());
    let before = fx.particle_count();
    let hit = apply_intro2_type26_particle_hit(
        &mut first,
        &session.cache,
        &mut fx,
        &mut SpecializedActorTaskScheduler::new(),
        primary_impact(id),
        9999,
    );
    assert!(matches!(
        hit,
        Intro2Type26ImpactOutcome::Blocked {
            committed_prefix: false,
            ..
        }
    ));
    let entity = first.entity_mut(id).unwrap();
    assert_eq!(entity.collision.last_hit_presentation_tick_at_0x34, tick);
    assert_eq!(entity.collision.health_raw, health);
    assert_eq!(fx.particle_count(), before);
    entity.native_type26_allocation = original;
    entity.set_position_raw([1234, 567, -1234]);
    assert!(
        Intro2Type26WorldOwner::adopt(&first, id).is_ok(),
        "live pose is not birth custody"
    );
}

fn static_route_impact(id: u32, class: u8, packet: DamagePacket) -> ParticleEntityImpact {
    let mut impact = primary_impact(id);
    impact.source_particle_class = class;
    impact.damage.as_mut().unwrap().packet = packet;
    impact
}

fn class5_carrier_count(fx: &mut WorldFx) -> usize {
    fx.prepare_presentation([640, 480], 0x3000, |_| {
        v2k_render::ParticleCenterProjection {
            screen: [320, 240],
            depth_raw: 1000,
            clip: 0,
        }
    })
    .particles()
    .filter(|prepared| prepared.particle.source_class == 5)
    .count()
}

#[v2k_test_support::retail_test]
fn authored_type26_static_route_packets_replace_all_eight_native_graphs_without_primary_suffix() {
    use crate::{
        damage::{
            CLASS68_STATIC_ROUTE_DAMAGE_PACKET, DRAGON_FIREBALL_DAMAGE_PACKET,
            TYPE_47_PROJECTILE_DAMAGE_PACKET,
        },
        gameplay_notifications::GameplayNotifications,
        shared_actor_impact::{
            apply_shared_actor_particle_hit, SharedActorImpactFrame, SharedActorImpactOutcome,
        },
        specialized_actor_task_production::SpecializedActorTaskFamily,
    };

    let mut session = session();
    let mut census = [0; 3];
    for world in [15, 18] {
        session.load_level_by_id(world, 1).unwrap();
        for (case, (class, packet, expected_damage)) in [
            (52, TYPE_47_PROJECTILE_DAMAGE_PACKET, 0),
            (68, CLASS68_STATIC_ROUTE_DAMAGE_PACKET, 200),
            (85, DRAGON_FIREBALL_DAMAGE_PACKET, 4000),
        ]
        .into_iter()
        .enumerate()
        {
            let mut fx = WorldFx::new();
            let mut manager = construct(&session, world, &mut fx);
            let mut scheduler = SpecializedActorTaskScheduler::new();
            let actors = ids(&manager);
            assert_eq!(scheduler.adopt_intro2_type26(&manager), actors.len());
            let metadata = manager.type_runtime_metadata(26).unwrap().clone();
            assert_eq!(
                metadata.damage_profile.unwrap().filter(packet),
                expected_damage
            );
            let RetailRuntimeValue::Known(cue) = metadata.accepted_hit_presentation_sound_id else {
                panic!("authored Type26 +80 cue");
            };
            let mut notifications = GameplayNotifications::new();
            fx.process_pending();
            fx.take_positional_sounds();
            for id in actors {
                census[case] += 1;
                let entity = manager.entity_mut(id).unwrap();
                let receipt = entity
                    .native_type26_allocation
                    .expect("ordinary constructor custody");
                assert_eq!(Some(receipt.spawn_index), entity.authored_spawn_index);
                assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(5000));
                assert_eq!(
                    entity.collision.pre_health_damage_buffer_raw,
                    RetailRuntimeValue::Known(0)
                );
                assert_ne!(
                    entity.capability_flags & 8,
                    0,
                    "a primary hit would emit class5"
                );
                let before_primary = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
                assert!(Intro2Type26WorldOwner::adopt(&manager, id).is_ok());
                let carriers_before = class5_carrier_count(&mut fx);
                let hit = apply_shared_actor_particle_hit(
                    SharedActorImpactFrame {
                        resources: &session.cache,
                        entities: &mut manager,
                        world_fx: &mut fx,
                        scheduler: &mut scheduler,
                        notifications: &mut notifications,
                        retail_tick: 2428,
                    },
                    static_route_impact(id, class, packet),
                );
                let Some(SharedActorImpactOutcome::Type26(Intro2Type26ImpactOutcome::Applied(hit))) =
                    hit
                else {
                    panic!("world{world} id{id} class{class}: {hit:?}");
                };
                assert_eq!(hit.filtered_damage_raw, expected_damage);
                assert!(hit.death_publication.is_none());
                let entity = manager.entity_mut(id).unwrap();
                assert_eq!(entity.native_type26_allocation, Some(receipt));
                assert_eq!(
                    entity.collision.health_raw,
                    RetailRuntimeValue::Known(5000 - expected_damage)
                );
                assert_eq!(
                    entity.collision.last_hit_presentation_tick_at_0x34,
                    RetailRuntimeValue::Known(2428)
                );
                assert_ne!(
                    entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
                    before_primary,
                    "DAC0's C690 replaces the real current graph even when the filter returns zero"
                );
                assert!(Intro2Type26WorldOwner::adopt(&manager, id).is_ok());
                assert_eq!(
                    scheduler.family_for(id),
                    Some(SpecializedActorTaskFamily::Intro2Type26)
                );
                assert_eq!(
                    scheduler.adopt_intro2_type26(&manager),
                    0,
                    "completed replacement retains custody"
                );
                fx.process_pending();
                let sounds = fx.take_positional_sounds();
                if let Some(cue) = cue {
                    assert_eq!(
                        sounds.first().map(|sound| sound.sound_id),
                        Some(usize::from(cue)),
                        "11180's authored alive cue precedes C690 and generic hit presentation"
                    );
                }
                assert_eq!(
                    class5_carrier_count(&mut fx),
                    carriers_before,
                    "11180 has no 10EB0 capability8 suffix"
                );

                if class == 85 {
                    // Keep the constructor's 5000 health. A second real 4000
                    // packet kills it; a third visits the retained class12 graph.
                    let mut corpse_primary = None;
                    for tick in [2429, 2430] {
                        let hit = apply_shared_actor_particle_hit(
                            SharedActorImpactFrame {
                                resources: &session.cache,
                                entities: &mut manager,
                                world_fx: &mut fx,
                                scheduler: &mut scheduler,
                                notifications: &mut notifications,
                                retail_tick: tick,
                            },
                            static_route_impact(id, class, packet),
                        );
                        let Some(SharedActorImpactOutcome::Type26(
                            Intro2Type26ImpactOutcome::Applied(hit),
                        )) = hit
                        else {
                            panic!("world{world} id{id} corpse tick{tick}: {hit:?}");
                        };
                        assert_eq!(hit.filtered_damage_raw, 4000);
                        assert_eq!(hit.death_publication.is_some(), tick == 2429);
                        let entity = manager.entity_mut(id).unwrap();
                        assert_eq!(entity.native_type26_allocation, Some(receipt));
                        assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
                        assert_eq!(
                            entity.collision.last_hit_presentation_tick_at_0x34,
                            RetailRuntimeValue::Known(tick)
                        );
                        let primary = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
                        if tick == 2429 {
                            corpse_primary = primary;
                        } else {
                            assert_eq!(
                                primary, corpse_primary,
                                "class12's null hit hook keeps the corpse task"
                            );
                        }
                        assert!(Intro2CommonDyingOwner::adopt(&manager, id).is_ok());
                        assert_eq!(
                            scheduler.family_for(id),
                            Some(SpecializedActorTaskFamily::Intro2CommonDying)
                        );
                        fx.process_pending();
                        fx.take_positional_sounds();
                        assert_eq!(class5_carrier_count(&mut fx), carriers_before);
                    }
                }
            }
        }
    }
    assert_eq!(census, [8; 3]);
}

#[v2k_test_support::retail_test]
fn generic_type26_static_route_rejects_before_hit_prefix_or_task_mutation() {
    use crate::{
        damage::{
            CLASS68_STATIC_ROUTE_DAMAGE_PACKET, DRAGON_FIREBALL_DAMAGE_PACKET,
            TYPE_47_PROJECTILE_DAMAGE_PACKET,
        },
        gameplay_notifications::GameplayNotifications,
        shared_actor_impact::{
            apply_shared_actor_particle_hit, SharedActorImpactFrame, SharedActorImpactOutcome,
        },
    };
    let mut session = session();
    session.load_level_by_id(15, 1).unwrap();
    let rows: Vec<_> = session
        .cache
        .global_entity_model_table()
        .iter()
        .enumerate()
        .map(|(id, _)| {
            EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(id).unwrap())
        })
        .collect();
    let mut manager = EntityManager::from_level_with_type_metadata(
        session.cache.level_desc().unwrap(),
        &rows,
        EntityConstructionResources {
            terrain: session.cache.terrain(),
            terrain_objects: session.cache.terrain_objects(),
            model_extent_raw: Some(&|id| session.cache.global_model(id).map(|model| model.radius)),
        },
    );
    let id = ids(&manager)[0];
    let entity = manager.entity_mut(id).unwrap();
    assert!(entity.native_type26_allocation.is_none());
    assert!(entity.intro2_type26_sub_d_runtime.is_none());
    let collision = entity.collision.clone();
    let slots =
        ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_tasks.task_in_slot(slot));
    let mut fx = WorldFx::new();
    let mut scheduler = SpecializedActorTaskScheduler::new();
    let mut notifications = GameplayNotifications::new();
    for (class, packet) in [
        (52, TYPE_47_PROJECTILE_DAMAGE_PACKET),
        (68, CLASS68_STATIC_ROUTE_DAMAGE_PACKET),
        (85, DRAGON_FIREBALL_DAMAGE_PACKET),
    ] {
        let hit = apply_shared_actor_particle_hit(
            SharedActorImpactFrame {
                resources: &session.cache,
                entities: &mut manager,
                world_fx: &mut fx,
                scheduler: &mut scheduler,
                notifications: &mut notifications,
                retail_tick: 2428,
            },
            static_route_impact(id, class, packet),
        );
        assert!(
            matches!(
                hit,
                Some(SharedActorImpactOutcome::UnsupportedStaticRouteTarget { entity_type: 26 })
            ),
            "{hit:?}"
        );
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(entity.collision, collision);
        assert_eq!(
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_tasks.task_in_slot(slot)),
            slots
        );
        assert_eq!(fx.pending_event_count(), 0);
        assert_eq!(scheduler.family_for(id), None);
    }
}
