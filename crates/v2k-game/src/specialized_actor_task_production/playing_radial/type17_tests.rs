use super::*;
use crate::{
    actor_task_owner::ActorTaskSlot,
    damage::DamagePacket,
    entity::{AuthoredPlayerArrival, AuthoredWorldConstruction, EntityConstructionResources},
    entity_collision_state::{
        EntityTypeRuntimeMetadata, RetailRuntimeValue, CHECKED_DAMAGE_ENABLED_STATE_BIT,
    },
    intro2_common_dying::{
        publish_intro2_common_standard_death, tick_intro2_common_dying, Intro2CommonDyingFrame,
        Intro2CommonDyingOutcome,
    },
    intro2_type17::{
        tick_intro2_type17, Intro2Type17Frame, Intro2Type17Outcome, Intro2Type17Owner,
    },
    session::GameSession,
};

struct Fixture {
    session: GameSession,
    manager: EntityManager,
    scheduler: SpecializedActorTaskScheduler,
    fx: WorldFx,
    target: u32,
}

fn fixture() -> Fixture {
    let data = v2k_test_support::retail_dir();
    assert!(data.join("PRELOAD.DAT").is_file(), "retail corpus required");
    let mut session = GameSession::init(&data).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(14, 1).unwrap();
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
            logical_world_index: 2,
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
                position_raw: [19_712, -500, 14_848],
                heading_raw: 0x4000,
            }),
            retail_tick: 0,
        },
        &mut fx,
    )
    .unwrap();
    let target = manager
        .iter_all()
        .find(|entity| entity.entity_type == 17)
        .unwrap()
        .id;
    let other_ids: Vec<_> = manager
        .iter_all()
        .filter(|entity| entity.id != target)
        .map(|entity| entity.id)
        .collect();
    for id in other_ids {
        manager
            .entity_mut(id)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .overwrite(CHECKED_DAMAGE_ENABLED_STATE_BIT, 0);
    }
    let mut scheduler = SpecializedActorTaskScheduler::new();
    assert_eq!(scheduler.adopt_intro2_type17(&manager), 1);
    Fixture {
        session,
        manager,
        scheduler,
        fx,
        target,
    }
}

fn deliver(f: &mut Fixture, lethal: bool) -> PlayingRadialOutcome {
    let mut origin = f.manager.entity_mut(f.target).unwrap().position_raw();
    origin[0] = origin[0].wrapping_sub(1);
    f.scheduler.apply_playing_radial_damage(PlayingRadialFrame {
        resources: &mut f.session.cache,
        static_damage: &mut crate::static_damage::StaticDamageScheduler::new(),
        active_terminal_calls: Vec::new(),
        entities: &mut f.manager,
        player_hull: &mut PlayerHull::default(),
        extra_lives: crate::entity_collision_state::RetailRuntimeValue::Unresolved,
        origin_raw: origin,
        template: RadialDamageTemplate {
            inner_radius_raw: 8,
            outer_radius_raw: 16,
            impulse_raw: 2000,
            packet: DamagePacket {
                channels: [1, 3],
                amounts_raw: if lethal { [20_000, 20_000] } else { [100, 100] },
            },
            trailing_raw: [0, 0],
        },
        world_fx: &mut f.fx,
        notifications: &mut GameplayNotifications::new(),
        retail_tick: 77,
    })
}

#[v2k_test_support::retail_test]
fn native_type17_playing_radial_publishes_class12_and_accepts_its_completed_owner() {
    let mut f = fixture();
    let receipt = f
        .manager
        .entity_mut(f.target)
        .unwrap()
        .intro2_type17_runtime;
    let outcome = deliver(&mut f, true);
    assert!(outcome.blocked.is_none(), "{outcome:?}");
    assert_eq!(outcome.completed_target_ids, [f.target]);
    let entity = f.manager.entity_mut(f.target).unwrap();
    assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
    assert_eq!(entity.intro2_type17_runtime, receipt);
    let dying_task = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .unwrap();
    assert!(matches!(
        entity.actor_task_state(ActorTaskSlot::Primary),
        Some(crate::actor_task_dispatcher::ActorTaskRuntime::CommonDying(
            _
        ))
    ));
    let mut expected = f.fx.fork_for_main_base_abort_transaction();
    let repeated = deliver(&mut f, true);
    assert!(repeated.blocked.is_none(), "{repeated:?}");
    assert_eq!(repeated.completed_target_ids, [f.target]);
    let entity = f.manager.entity_mut(f.target).unwrap();
    assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
    assert_eq!(
        entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
        Some(dying_task)
    );
    assert_eq!(
        f.fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
}

#[derive(Clone, Copy, Debug)]
enum BlockedOwner {
    Missing,
    Foreign,
    PendingLiving,
    PendingDying,
}

#[v2k_test_support::retail_test]
fn native_type17_radial_and_particles_reject_missing_foreign_and_pending_custody() {
    for mode in [
        BlockedOwner::Missing,
        BlockedOwner::Foreign,
        BlockedOwner::PendingLiving,
        BlockedOwner::PendingDying,
    ] {
        let mut f = fixture();
        match mode {
            BlockedOwner::Missing => f.scheduler = SpecializedActorTaskScheduler::new(),
            BlockedOwner::Foreign => {
                let mut foreign = fixture();
                f.manager
                    .entity_mut(f.target)
                    .unwrap()
                    .intro2_type17_runtime = foreign
                    .manager
                    .entity_mut(foreign.target)
                    .unwrap()
                    .intro2_type17_runtime;
            }
            BlockedOwner::PendingLiving | BlockedOwner::PendingDying => {
                let dying = if matches!(mode, BlockedOwner::PendingDying) {
                    Some(
                        publish_intro2_common_standard_death(&mut f.manager, f.target, &mut f.fx)
                            .unwrap()
                            .unwrap(),
                    )
                } else {
                    None
                };
                let entity = f.manager.entity_mut(f.target).unwrap();
                // Reach a real post-scheduler evidence boundary; the pending
                // state is the callback's retained result, not a forged flag.
                entity
                    .collision
                    .state_flags_at_0x08
                    .overwrite(0x0202_0000, 0x0202_0000);
                entity.physical_body_basis_q31 = RetailRuntimeValue::Unresolved;
                if let Some(owner) = dying {
                    let tick = tick_intro2_common_dying(
                        &mut f.manager,
                        owner,
                        Intro2CommonDyingFrame {
                            resources: &f.session.cache,
                            world_fx: &mut f.fx,
                            elapsed_micros: 20_000,
                            retail_tick: 76,
                        },
                    );
                    assert!(
                        matches!(
                            tick.outcome,
                            Intro2CommonDyingOutcome::Blocked {
                                prefix_committed: true,
                                ..
                            }
                        ),
                        "{mode:?}: {:?}",
                        tick.outcome
                    );
                    f.scheduler
                        .register_intro2_common_dying(tick.retained_owner.unwrap());
                } else {
                    let owner = Intro2Type17Owner::adopt(&f.manager, f.target).unwrap();
                    let tick = tick_intro2_type17(
                        &mut f.manager,
                        owner,
                        Intro2Type17Frame {
                            capture_tasks: &mut crate::specialized_actor_task_production::SpecializedActorTaskScheduler::default(),
                            notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
                            resources: &f.session.cache,
                            world_fx: &mut f.fx,
                            elapsed_micros: 20_000,
                            retail_tick: 76,
                        },
                    );
                    assert!(
                        matches!(
                            tick.outcome,
                            Intro2Type17Outcome::Blocked {
                                prefix_committed: true,
                                ..
                            }
                        ),
                        "{mode:?}: {:?}",
                        tick.outcome
                    );
                    f.scheduler
                        .register_intro2_type17(tick.retained_owner.unwrap());
                }
            }
        }
        let entity = f.manager.entity_mut(f.target).unwrap();
        let collision = entity.collision.clone();
        let velocity = entity.velocity_raw();
        let context = entity.current_behavior_context;
        let tasks =
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_task_state(slot).cloned());
        let events = f.fx.pending_event_count();
        let mut expected = f.fx.fork_for_main_base_abort_transaction();
        let outcome = deliver(&mut f, false);
        assert_eq!(
            outcome.blocked,
            Some(PlayingRadialBlock::Native(DynamicRadialLiveBlock {
                target_id: f.target,
                phase: crate::entity::DynamicRadialLivePhase::MutationCustody,
                target_prefix_committed: false,
                reason: crate::entity::DynamicRadialLiveBlockReason::NativeActorMutationCustody,
            })),
            "{mode:?}"
        );
        for source_particle_class in [5, 16] {
            let outcome = crate::shared_actor_impact::apply_shared_actor_particle_hit(
                crate::shared_actor_impact::SharedActorImpactFrame {
                    resources: &f.session.cache,
                    entities: &mut f.manager,
                    scheduler: &mut f.scheduler,
                    world_fx: &mut f.fx,
                    notifications: &mut GameplayNotifications::new(),
                    retail_tick: 77,
                },
                crate::world_fx::ParticleEntityImpact {
                    source_particle_class,
                    impact_position_argument_va: 0,
                    target_entity_id: f.target,
                    position_world: [0.0; 3],
                    velocity_raw: [0, 0, 8192],
                    damage: Some(crate::world_fx::BallisticDamageRequest {
                        packet: crate::damage::PRIMARY_PROJECTILE_DAMAGE_PACKET,
                        source_entity_type_at_birth: Some(51),
                        source_owner_id: Some(0),
                    }),
                },
            );
            assert!(
                matches!(
                    outcome,
                    Some(
                        crate::shared_actor_impact::SharedActorImpactOutcome::Spider(
                            crate::intro2_type17::impact::Intro2Type17ImpactOutcome::Blocked {
                                reason:
                                    crate::intro2_type17::impact::Intro2Type17ImpactBlock::Runtime(
                                        "completed actor custody"
                                    ),
                                committed_prefix: false,
                            }
                        )
                    )
                ),
                "{mode:?}: {outcome:?}"
            );
        }
        let entity = f.manager.entity_mut(f.target).unwrap();
        assert_eq!(entity.collision, collision);
        assert_eq!(entity.velocity_raw(), velocity);
        assert_eq!(entity.current_behavior_context, context);
        assert_eq!(
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_task_state(slot).cloned()),
            tasks
        );
        assert_eq!(f.fx.pending_event_count(), events);
        assert_eq!(
            f.fx.next_shared_retail_random_u16(),
            expected.next_shared_retail_random_u16()
        );
    }
}
