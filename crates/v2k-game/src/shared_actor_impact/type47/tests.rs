use super::*;
use crate::{
    actor_task_owner::ActorTaskSlot,
    damage::{DamagePacket, FUN_0043F780_DAMAGE_DELIVERY},
    entity::{
        AuthoredPlayerArrival, AuthoredWorldConstruction, EntityConstructionResources,
        EntityManager,
    },
    entity_collision_state::EntityTypeRuntimeMetadata,
    gameplay_notifications::GameplayNotifications,
    intro2_radial::Intro2RadialTaskCustody,
    session::GameSession,
    shared_actor_impact::{apply_shared_actor_particle_hit, SharedActorImpactOutcome},
    specialized_actor_task_production::{
        SpecializedActorTaskFamily, SpecializedActorTaskScheduler,
    },
    world_fx::{BallisticDamageRequest, WorldFx},
};

pub(crate) struct Fixture {
    pub session: GameSession,
    pub manager: EntityManager,
    pub scheduler: SpecializedActorTaskScheduler,
    pub fx: WorldFx,
    pub notifications: GameplayNotifications,
}
pub(crate) fn fixture(world: u32) -> Fixture {
    let data = v2k_test_support::retail_dir();
    assert!(data.join("PRELOAD.DAT").is_file(), "retail corpus required");
    let mut session = GameSession::init(&data).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(world, 1).unwrap();
    let metadata: Vec<_> = session
        .cache
        .global_entity_model_table()
        .iter()
        .enumerate()
        .map(|(kind, _)| {
            EntityTypeRuntimeMetadata::from_section12(
                session.cache.global_entity_type(kind).unwrap(),
            )
        })
        .collect();
    let mut fx = WorldFx::new();
    let mut manager = EntityManager::from_authored_world(
        AuthoredWorldConstruction {
            logical_world_index: (world - 12) as i32,
            level: session.cache.level_desc().unwrap(),
            type_metadata: &metadata,
            resources: EntityConstructionResources {
                terrain: session.cache.terrain(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&|id| {
                    session.cache.global_model(id).map(|model| model.radius)
                }),
            },
            player_arrival: Some(AuthoredPlayerArrival {
                position_raw: [19712, -500, 14848],
                heading_raw: 0x4000,
            }),
            retail_tick: 0,
        },
        &mut fx,
    )
    .unwrap();
    let mut scheduler = SpecializedActorTaskScheduler::new();
    let ids: Vec<_> = manager
        .iter_all()
        .filter(|entity| entity.entity_type == 47)
        .map(|entity| entity.id)
        .collect();
    assert!(!ids.is_empty());
    assert_eq!(
        scheduler
            .adopt_fresh_level1_type47_scheduler(&mut manager)
            .unwrap(),
        0
    );
    assert_eq!(scheduler.adopt_intro2_type47_guards(&manager), ids.len());
    assert_eq!(scheduler.adopt_intro2_type47_guards(&manager), 0);
    for id in ids {
        assert!(type47_manager_allocation_authenticates(&manager, id));
        let entity = manager.entity_mut(id).unwrap();
        // A fixed local hit boundary; the real birth/task/component graph is untouched.
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(u32::MAX, 0x0406_8000);
        entity.collision.last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(17);
    }
    fx.process_pending();
    fx.take_positional_sounds();
    Fixture {
        session,
        manager,
        scheduler,
        fx,
        notifications: GameplayNotifications::new(),
    }
}
pub(crate) fn first_id(f: &Fixture) -> u32 {
    f.manager
        .iter_all()
        .find(|entity| entity.entity_type == 47)
        .unwrap()
        .id
}
fn impact(id: u32, infected: bool, amount: i32) -> ParticleEntityImpact {
    ParticleEntityImpact {
        source_particle_class: if infected { 5 } else { 16 },
        impact_position_argument_va: if infected { 0x004D_CF48 } else { 0 },
        target_entity_id: id,
        position_world: [0.0; 3],
        velocity_raw: [0, 0, 8192],
        damage: Some(BallisticDamageRequest {
            packet: if infected {
                FUN_0043F780_DAMAGE_DELIVERY.packet
            } else {
                DamagePacket {
                    channels: [1, 0],
                    amounts_raw: [amount, 0],
                }
            },
            source_entity_type_at_birth: Some(34),
            source_owner_id: Some(35),
        }),
    }
}
fn deliver(f: &mut Fixture, id: u32, infected: bool, amount: i32) -> NativeType47ImpactOutcome {
    let outcome = apply_shared_actor_particle_hit(
        SharedActorImpactFrame {
            resources: &f.session.cache,
            entities: &mut f.manager,
            world_fx: &mut f.fx,
            scheduler: &mut f.scheduler,
            notifications: &mut f.notifications,
            retail_tick: 77,
        },
        impact(id, infected, amount),
    )
    .expect("native receipt must select shared entry");
    match outcome {
        SharedActorImpactOutcome::Insect(result) => result,
        other => panic!("{other:?}"),
    }
}

#[v2k_test_support::retail_test]
fn native_type47_antidote_reselects_before_filtered_damage_and_omits_primary_suffix() {
    for world in [13, 14, 15, 31] {
        let mut f = fixture(world);
        let id = first_id(&f);
        let metadata = f.manager.type_runtime_metadata(47).unwrap();
        let filtered = metadata
            .damage_profile
            .unwrap()
            .filter(crate::damage::FUN_0043F7C0_DAMAGE_PACKET);
        let entity = f.manager.entity_mut(id).unwrap();
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(0x2000, 0x2000);
        let RetailRuntimeValue::Known(health) = entity.collision.health_raw else {
            panic!()
        };
        let primary = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
        let mut impact = impact(id, false, 0);
        impact.source_particle_class = 6;
        impact.damage.as_mut().unwrap().packet = crate::damage::FUN_0043F7C0_DAMAGE_PACKET;
        let result = apply_shared_actor_particle_hit(
            SharedActorImpactFrame {
                resources: &f.session.cache,
                entities: &mut f.manager,
                world_fx: &mut f.fx,
                scheduler: &mut f.scheduler,
                notifications: &mut f.notifications,
                retail_tick: 77,
            },
            impact,
        )
        .unwrap();
        let SharedActorImpactOutcome::Insect(NativeType47ImpactOutcome::Applied(result)) = result
        else {
            panic!("{result:?}")
        };
        assert_eq!(result.filtered_damage_raw, filtered);
        let entity = f.manager.entity_mut(id).unwrap();
        assert_eq!(
            entity.collision.health_raw,
            RetailRuntimeValue::Known(health - filtered)
        );
        assert_eq!(
            entity.collision.state_flags_at_0x08.masked(0x2000),
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            entity.collision.last_hit_presentation_tick_at_0x34,
            RetailRuntimeValue::Known(17)
        );
        assert_ne!(
            entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
            primary
        );
        assert!(f.scheduler.prepare_native_actor_mutation(&f.manager, id));
        f.fx.process_pending();
        assert!(f.fx.take_positional_sounds().is_empty());
        assert_eq!(f.fx.particle_count(), 0);
    }
}

#[v2k_test_support::retail_test]
fn antidote_player_callback_boundary_blocks_before_model_prefix_and_generic_damage() {
    let mut f = fixture(13);
    let id = f.manager.player().unwrap().id;
    let collision = f.manager.player().unwrap().collision.clone();
    let mut impact = impact(id, false, 0);
    impact.source_particle_class = 6;
    impact.damage.as_mut().unwrap().packet = crate::damage::FUN_0043F7C0_DAMAGE_PACKET;
    let result = apply_shared_actor_particle_hit(
        SharedActorImpactFrame {
            resources: &f.session.cache,
            entities: &mut f.manager,
            world_fx: &mut f.fx,
            scheduler: &mut f.scheduler,
            notifications: &mut f.notifications,
            retail_tick: 77,
        },
        impact,
    );
    assert!(matches!(
        result,
        Some(SharedActorImpactOutcome::UnsupportedCuredTarget { entity_type: 46 })
    ));
    assert_eq!(f.manager.player().unwrap().collision, collision);
    f.fx.process_pending();
    assert!(f.fx.take_positional_sounds().is_empty());
    assert_eq!(f.fx.particle_count(), 0);
}

#[v2k_test_support::retail_test]
fn all_eight_ordinary_type47_births_use_shared_primary_and_infected_reselection() {
    for infected in [false, true] {
        let mut count = 0;
        for world in [13, 14, 15, 31] {
            let mut f = fixture(world);
            let ids: Vec<_> = f
                .manager
                .iter_all()
                .filter(|entity| entity.entity_type == 47)
                .map(|entity| entity.id)
                .collect();
            for id in ids {
                count += 1;
                let entity = f.manager.entity_mut(id).unwrap();
                let receipt = entity.native_type47_construction;
                let primary = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
                let outcome = deliver(&mut f, id, infected, 2500);
                let NativeType47ImpactOutcome::Applied(damage) = outcome else {
                    panic!("world {world}: {outcome:?}");
                };
                assert!(damage.death_publication.is_none());
                let entity = f.manager.entity_mut(id).unwrap();
                assert_eq!(entity.native_type47_construction, receipt);
                assert_ne!(
                    entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
                    primary,
                    "C690 runs before either damage filter"
                );
                assert_eq!(
                    entity.collision.last_hit_presentation_tick_at_0x34,
                    RetailRuntimeValue::Known(if infected { 17 } else { 77 })
                );
                assert_eq!(
                    f.scheduler.family_for(id),
                    Some(SpecializedActorTaskFamily::Intro2Type47Scheduler)
                );
                assert!(f.scheduler.prepare_native_actor_mutation(&f.manager, id));
                assert!(!f.scheduler.native_type47_has_pending_prefix(id));
            }
        }
        assert_eq!(count, 8);
    }
}

#[v2k_test_support::retail_test]
fn native_type47_lethal_and_null_class12_hits_keep_the_same_death_owner() {
    let mut f = fixture(14);
    let id = first_id(&f);
    let receipt = f.manager.entity_mut(id).unwrap().native_type47_construction;
    let outcome = deliver(&mut f, id, false, 20_000);
    assert!(
        matches!(outcome, NativeType47ImpactOutcome::Applied(ref damage) if damage.death_publication.is_some()),
        "{outcome:?}"
    );
    assert_eq!(
        f.scheduler.family_for(id),
        Some(SpecializedActorTaskFamily::Intro2CommonDying)
    );
    let primary = f
        .manager
        .entity_mut(id)
        .unwrap()
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary);
    for infected in [false, true] {
        let outcome = deliver(&mut f, id, infected, 20_000);
        assert!(
            matches!(outcome, NativeType47ImpactOutcome::Applied(ref damage) if damage.death_publication.is_none()),
            "{outcome:?}"
        );
        let entity = f.manager.entity_mut(id).unwrap();
        assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
        assert_eq!(
            entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
            primary
        );
        assert_eq!(entity.native_type47_construction, receipt);
    }
}

#[v2k_test_support::retail_test]
fn native_type47_foreign_missing_and_committed_hit_custody_blocks_before_replay() {
    for mode in 0..3 {
        let mut f = fixture(14);
        let id = first_id(&f);
        if mode == 0 {
            f.scheduler = SpecializedActorTaskScheduler::new();
        }
        if mode == 1 {
            let foreign = fixture(14);
            let foreign_id = first_id(&foreign);
            f.manager.entity_mut(id).unwrap().native_type47_construction = foreign
                .manager
                .iter_all()
                .find(|entity| entity.id == foreign_id)
                .unwrap()
                .native_type47_construction;
        }
        if mode == 2 {
            // C690 really completes before the next source-owned 11030 read blocks.
            f.manager
                .entity_mut(id)
                .unwrap()
                .collision
                .state_flags_at_0x08
                .invalidate(IMPACT_REACTION_ENABLED_STATE_BIT);
            let result = deliver(&mut f, id, false, 2500);
            assert!(
                matches!(
                    result,
                    NativeType47ImpactOutcome::Blocked {
                        committed_prefix: true,
                        ..
                    }
                ),
                "{result:?}"
            );
            assert!(f.scheduler.native_type47_has_pending_prefix(id));
        }
        let entity = f.manager.entity_mut(id).unwrap();
        let before = entity.collision.clone();
        let tasks_before = format!("{:?}", entity.actor_tasks);
        let mut expected = f.fx.fork_for_main_base_abort_transaction();
        for infected in [false, true] {
            let result = deliver(&mut f, id, infected, 2500);
            assert!(
                matches!(
                    result,
                    NativeType47ImpactOutcome::Blocked {
                        committed_prefix: false,
                        ..
                    }
                ),
                "mode {mode}: {result:?}"
            );
            let entity = f.manager.entity_mut(id).unwrap();
            assert_eq!(entity.collision, before);
            assert_eq!(format!("{:?}", entity.actor_tasks), tasks_before);
        }
        assert_eq!(
            f.fx.next_shared_retail_random_u16(),
            expected.next_shared_retail_random_u16()
        );
    }
}

#[v2k_test_support::retail_test]
fn native_type47_playing_radial_uses_current_class12_custody_and_rejects_parked_owner() {
    for parked in [false, true] {
        let mut f = fixture(31);
        let id = first_id(&f);
        let others: Vec<_> = f
            .manager
            .iter_all()
            .filter(|entity| entity.id != id)
            .map(|entity| entity.id)
            .collect();
        for other in others {
            f.manager
                .entity_mut(other)
                .unwrap()
                .collision
                .state_flags_at_0x08
                .overwrite(0x8000, 0);
        }
        if parked {
            f.scheduler.park_native_type47_external_prefix(id);
        }
        let before = f.manager.entity_mut(id).unwrap().collision.clone();
        let mut origin = f.manager.entity_mut(id).unwrap().position_raw();
        origin[0] = origin[0].wrapping_sub(1);
        let result = f.scheduler.apply_playing_radial_damage(
            crate::specialized_actor_task_production::PlayingRadialFrame {
                extra_lives: crate::entity_collision_state::RetailRuntimeValue::Unresolved,
                resources: &mut f.session.cache,
                static_damage: &mut crate::static_damage::StaticDamageScheduler::new(),
                active_terminal_calls: Vec::new(),
                entities: &mut f.manager,
                player_hull: &mut crate::player_hull::PlayerHull::default(),
                origin_raw: origin,
                template: crate::radial_damage::RadialDamageTemplate {
                    inner_radius_raw: 8,
                    outer_radius_raw: 16,
                    impulse_raw: 2000,
                    packet: DamagePacket {
                        channels: [1, 3],
                        amounts_raw: [20000, 20000],
                    },
                    trailing_raw: [0, 0],
                },
                world_fx: &mut f.fx,
                notifications: &mut f.notifications,
                retail_tick: 77,
            },
        );
        if parked {
            assert!(
                matches!(
                    result.blocked,
                    Some(
                        crate::specialized_actor_task_production::PlayingRadialBlock::Native(
                            crate::entity::DynamicRadialLiveBlock {
                                target_prefix_committed: false,
                                phase: crate::entity::DynamicRadialLivePhase::MutationCustody,
                                ..
                            }
                        )
                    )
                ),
                "{result:?}"
            );
            assert_eq!(f.manager.entity_mut(id).unwrap().collision, before);
        } else {
            assert!(result.blocked.is_none(), "{result:?}");
            assert_eq!(result.completed_target_ids, [id]);
            assert_eq!(
                f.scheduler.family_for(id),
                Some(SpecializedActorTaskFamily::Intro2CommonDying)
            );
            assert_eq!(
                f.manager.entity_mut(id).unwrap().collision.health_raw,
                RetailRuntimeValue::Known(0)
            );
            assert!(f.scheduler.prepare_native_actor_mutation(&f.manager, id));
        }
    }
}
