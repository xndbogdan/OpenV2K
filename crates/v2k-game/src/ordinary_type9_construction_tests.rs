//! Real normal-tier authored-world regressions for shared Type9 ownership.

use super::*;
use crate::{
    damage::FUN_0043F780_DAMAGE_PACKET,
    entity::{
        AuthoredPlayerArrival, AuthoredWorldConstruction, BeamCommand, BeamOutcome,
        CargoDropContext, EntityConstructionResources, Fun00411250Type9DamageOutcome,
        LateTailMaterialiserFrame, PlayerCargoFrame,
    },
    entity_collision_state::{BODY_BASIS_REBUILT_STATE_BIT, DYING_STATE_BIT},
    entity_view_detail::RetailViewDetailContext,
    gameplay_notifications::GameplayNotifications,
    main_base_type9_abort::MainBaseType9ResultScreenState,
    main_base_type9_production::MainBaseType9ExplodingProductionOutcome as Exploding,
    ordinary_type9_attract_attention_production::OrdinaryType9AttractAttentionProductionOutcome as Attract,
    ordinary_type9_go_to_job_production::OrdinaryType9GoToJobProductionOutcome as Job,
    ordinary_type9_run_away_production::OrdinaryType9RunAwayProductionOutcome as RunAway,
    ordinary_type9_standard_death::{
        OrdinaryType9StandardDeathBlock, OrdinaryType9StandardDeathEntry,
    },
    ordinary_type9_wander_production::OrdinaryType9WanderProductionOutcome as Wander,
    session::GameSession,
    specialized_actor_task_production::{
        SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome as Outcome,
        SpecializedActorTaskScheduler,
    },
    static_damage::StaticDamageScheduler,
};

fn load(level_id: u32, world_fx: &mut WorldFx) -> Option<(GameSession, EntityManager)> {
    load_with_arrival(level_id, world_fx, None)
}

fn load_with_arrival(
    level_id: u32,
    world_fx: &mut WorldFx,
    player_arrival: Option<AuthoredPlayerArrival>,
) -> Option<(GameSession, EntityManager)> {
    let data = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&data).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(level_id, 1).unwrap();
    let metadata: Vec<_> = session
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
        .collect();
    let manager = EntityManager::from_authored_world(
        AuthoredWorldConstruction {
            logical_world_index: (level_id - 12) as i32,
            level: session.cache.level_desc().unwrap(),
            type_metadata: &metadata,
            resources: EntityConstructionResources {
                terrain: session.cache.terrain(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&|id| {
                    session.cache.global_model(id).map(|model| model.radius)
                }),
            },
            player_arrival,
            retail_tick: 0,
        },
        world_fx,
    )
    .unwrap_or_else(|error| panic!("level{level_id} native construction: {error:?}"));
    Some((session, manager))
}

struct CargoWorld {
    level: u32,
    session: GameSession,
    manager: EntityManager,
    scheduler: SpecializedActorTaskScheduler,
    fx: WorldFx,
    static_damage: StaticDamageScheduler,
    notifications: GameplayNotifications,
    tick: u32,
}

impl CargoWorld {
    fn step(&mut self) -> (Option<BeamOutcome>, Vec<Outcome>) {
        let mut claims: Vec<_> = self.scheduler.actor_animation_claims().collect();
        let cargo = self.manager.update_player_cargo(PlayerCargoFrame {
            elapsed_micros: 20_000,
            drop_context: Some(CargoDropContext {
                carrier_orientation: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
                terrain: self.session.cache.terrain().unwrap(),
            }),
            retail_tick: self.tick,
            scheduler: &mut self.scheduler,
            world_fx: &mut self.fx,
            notifications: &mut self.notifications,
        });
        assert!(
            cargo.blocked.is_empty(),
            "level{} cargo: {:?}",
            self.level,
            cargo.blocked
        );
        for claim in self.scheduler.actor_animation_claims() {
            if !claims.contains(&claim) {
                claims.push(claim);
            }
        }
        let pass = self.scheduler.tick(
            &mut self.manager,
            SpecializedActorTaskProductionFrame {
                world:
                    crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                hive_components: None,
                notification_phase:
                    crate::gameplay_notifications::GameplayNotificationPhase::Playing,
                resources: &mut self.session.cache,
                world_fx: &mut self.fx,
                static_damage: &mut self.static_damage,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: self.tick,
                main_base_abort_active: false,
            },
            &mut self.notifications,
        );
        assert!(
            pass.block.is_none(),
            "level{}: {:?}",
            self.level,
            pass.block
        );
        for outcome in &pass.outcomes {
            assert_not_blocked(self.level, outcome);
            if let Outcome::OrdinaryType9Carried(value) = outcome {
                use crate::ordinary_type9_carried_production::Type9CarriedProductionOutcome as Carried;
                assert!(
                    !matches!(value, Carried::Blocked { .. } | Carried::Dropped { .. }),
                    "{value:?}"
                );
            }
        }
        self.manager
            .advance_unclaimed_actor_animations(20_000, &claims);
        let blocked = self
            .manager
            .update_late_tail_materialisers(LateTailMaterialiserFrame {
                elapsed_micros: 20_000,
                terrain: self.session.cache.terrain().unwrap(),
                world_fx: &mut self.fx,
                scheduler: &mut self.scheduler,
                notifications: &mut self.notifications,
                retail_tick: self.tick,
            });
        assert!(
            blocked.is_empty(),
            "level{} materialiser: {blocked:?}",
            self.level
        );
        // Main's presentation pass publishes11400 after simulation. The
        // fixture camera follows the carrier with a level horizon; omitting
        // this phase leaves every distant actor permanently detailed and
        // changes the shared RNG stream consumed by the next root selection.
        let camera_position = self.manager.player().unwrap().position;
        self.scheduler.publish_presented_view_detail(
            &mut self.manager,
            RetailViewDetailContext::from_world(camera_position, 0., (52, 30)),
        );
        self.tick += 1;
        (cargo.beam, pass.outcomes)
    }
}

#[v2k_test_support::retail_test]
fn later_world_native_peasant_can_be_carried_dropped_and_resume_its_live_root() {
    use crate::actor_task_dispatcher::ActorTaskRuntime;
    for level in [14, 39] {
        let mut fx = WorldFx::new();
        let Some((session, mut manager)) = load_with_arrival(
            level,
            &mut fx,
            Some(AuthoredPlayerArrival {
                // Same source-backed direct-level controller arrival as main.rs.
                position_raw: [19_712, -500, 14_848],
                heading_raw: 0x4000,
            }),
        ) else {
            return;
        };
        let mut scheduler = SpecializedActorTaskScheduler::default();
        scheduler
            .adopt_fresh_level1_type9_selected(&mut manager)
            .unwrap();
        let actor = manager
            .iter_all()
            .find(|entity| entity.entity_type == 9)
            .unwrap();
        let id = actor.id;
        let initial = actor.position;
        let original_receipt = actor.ordinary_type9_native_receipt;
        let initial_task = actor.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
        manager.player_mut().unwrap().position = [initial[0] + 0.25, initial[1] - 1., initial[2]];
        manager.queue_beam(BeamCommand::Collect);
        let mut world = CargoWorld {
            level,
            session,
            manager,
            scheduler,
            fx,
            static_damage: StaticDamageScheduler::default(),
            notifications: GameplayNotifications::new(),
            tick: 0,
        };
        let mut collected = false;
        let mut carried_visits = 0;
        for _ in 0..30 {
            let (beam, outcomes) = world.step();
            if let Some(beam) = beam {
                assert_eq!(beam, BeamOutcome::Collected { entity_id: id });
                collected = true;
            }
            carried_visits += outcomes
                .iter()
                .filter(|outcome| {
                    outcome.entity_id() == id && matches!(outcome, Outcome::OrdinaryType9Carried(_))
                })
                .count();
        }
        assert!(collected && carried_visits >= 20);
        let entity = world
            .manager
            .iter_all()
            .find(|entity| entity.id == id)
            .unwrap();
        assert_eq!(
            entity.attached_to,
            world.manager.player().map(|player| player.id)
        );
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::None)
        ));
        assert_ne!(
            entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
            initial_task
        );
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
        let carry_task = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
        let x = initial[0] + 2.;
        let z = initial[2];
        let height = world.session.cache.terrain().unwrap().height_at(x, z);
        world.manager.player_mut().unwrap().position = [x, height - 1., z + 400. / 256.];
        world.manager.queue_beam(BeamCommand::Drop);
        let mut proxy = None;
        let mut released = None;
        for _ in 0..100 {
            let (beam, _) = world.step();
            if let Some(beam) = beam {
                let BeamOutcome::DropStarted { cargo_id, proxy_id } = beam else {
                    panic!("{beam:?}");
                };
                assert_eq!(cargo_id, id);
                proxy = Some(proxy_id);
            }
            let entity = world
                .manager
                .iter_all()
                .find(|entity| entity.id == id)
                .unwrap();
            if proxy.is_some() && entity.attached_to.is_none() {
                assert_ne!(
                    entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
                    carry_task
                );
                assert!(!matches!(
                    entity.actor_task_state(ActorTaskSlot::Primary),
                    Some(ActorTaskRuntime::None)
                ));
                assert!(world
                    .manager
                    .iter_all()
                    .all(|entity| Some(entity.id) != proxy));
                released = Some(entity.position_raw());
                break;
            }
        }
        let released = released.expect("native peasant materialiser must finish and release");
        assert!(ordinary_type9_native_allocation_authenticates(
            &world.manager,
            id
        ));
        assert_eq!(
            world
                .manager
                .entity_mut(id)
                .unwrap()
                .ordinary_type9_native_receipt,
            original_receipt
        );
        let mut moved = false;
        let mut last_outcome = None;
        for _ in 0..200 {
            let (_, outcomes) = world.step();
            last_outcome = outcomes
                .into_iter()
                .find(|outcome| outcome.entity_id() == id);
            assert!(
                last_outcome.is_some(),
                "level{level}: released peasant lost scheduler custody"
            );
            moved |= world.manager.entity_mut(id).unwrap().position_raw() != released;
        }
        let entity = world.manager.entity_mut(id).unwrap();
        assert!(moved, "level{level}: dropped peasant never resumed motion; last={last_outcome:?}, context={:?}, tasks={:?}, velocity={:?}, animation={:?}, state={:?}",
            entity.current_behavior_context,
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_task_state(slot)),
            entity.velocity_raw(), entity.actor_animation_runtime, entity.collision.state_flags_at_0x08);
    }
}

fn assert_not_blocked(level: u32, outcome: &Outcome) {
    let blocked = match outcome {
        Outcome::OrdinaryType9Wander(value) => {
            matches!(value, Wander::Blocked { .. } | Wander::Dropped { .. })
        }
        Outcome::OrdinaryType9GoToJob(value) => {
            matches!(value, Job::Blocked { .. } | Job::Dropped { .. })
        }
        Outcome::OrdinaryType9AttractAttention(value) => {
            matches!(value, Attract::Blocked { .. } | Attract::Dropped { .. })
        }
        Outcome::OrdinaryType9RunAway(value) => match value {
            RunAway::RootContinuation { outcome, .. } => matches!(
                outcome.as_ref(),
                Wander::Blocked { .. } | Wander::Dropped { .. }
            ),
            RunAway::Blocked { .. } | RunAway::Dropped { .. } => true,
            _ => false,
        },
        Outcome::OrdinaryType9Class14(value) => {
            matches!(value, Exploding::Blocked { .. } | Exploding::Dropped { .. })
        }
        _ => false,
    };
    assert!(!blocked, "level{level} Type9 production: {outcome:?}");
}

#[v2k_test_support::retail_test]
fn later_worlds_publish_adopt_and_advance_real_peasants() {
    for level_id in [14, 15, 25, 39] {
        let mut world_fx = WorldFx::new();
        let Some((mut session, mut manager)) = load(level_id, &mut world_fx) else {
            return;
        };
        let before: Vec<_> = manager
            .iter_all()
            .filter(|entity| entity.entity_type == 9)
            .map(|entity| {
                assert!(ordinary_type9_native_allocation_authenticates(
                    &manager, entity.id
                ));
                assert!(matches!(
                    entity.current_behavior_context,
                    RetailRuntimeValue::Known(Some(_))
                ));
                assert_eq!(
                    entity.collision.pair_callbacks.type_hit_callback_address,
                    RetailRuntimeValue::Known(None)
                );
                assert_eq!(
                    entity
                        .collision
                        .state_flags_at_0x08
                        .masked(BODY_BASIS_REBUILT_STATE_BIT),
                    RetailRuntimeValue::Known(BODY_BASIS_REBUILT_STATE_BIT)
                );
                assert!(!entity.collision.fresh_level1_type9_first_scheduler_pending);
                (entity.id, entity.position_raw())
            })
            .collect();
        assert!(
            !before.is_empty(),
            "level{level_id} must exercise real Type9 spawns"
        );
        let mut scheduler = SpecializedActorTaskScheduler::default();
        assert_eq!(
            scheduler
                .adopt_fresh_level1_type9_selected(&mut manager)
                .unwrap(),
            before.len()
        );
        let mut static_damage = StaticDamageScheduler::default();
        let mut notifications = GameplayNotifications::new();
        let mut moved = false;
        for step in 1..=100 {
            let pass = scheduler.tick(
                &mut manager,
                SpecializedActorTaskProductionFrame {
                    world: crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                    hive_components: None,
                    notification_phase:
                        crate::gameplay_notifications::GameplayNotificationPhase::Playing,
                    resources: &mut session.cache,
                    world_fx: &mut world_fx,
                    static_damage: &mut static_damage,
                    elapsed_micros: 100_000,
                    global_elapsed_micros: 100_000,
                    retail_tick: step * 5,
                    main_base_abort_active: false,
                },
                &mut notifications,
            );
            assert!(pass.block.is_none(), "level{level_id}: {:?}", pass.block);
            for outcome in &pass.outcomes {
                assert_not_blocked(level_id, outcome);
            }
            moved |= before.iter().any(|(id, position)| {
                manager
                    .iter_all()
                    .find(|entity| entity.id == *id)
                    .is_some_and(|entity| entity.position_raw() != *position)
            });
            manager.cleanup_pending_actor_deferred_destroys();
        }
        assert!(
            moved,
            "level{level_id}: every peasant retained its initial position"
        );
    }
}

#[v2k_test_support::retail_test]
fn later_world_projectile_death_reaches_class14_and_deferred_removal() {
    for level_id in [14, 15, 25, 39] {
        let mut world_fx = WorldFx::new();
        let Some((mut session, mut manager)) = load(level_id, &mut world_fx) else {
            return;
        };
        let target = manager
            .iter_all()
            .find(|entity| entity.entity_type == 9)
            .unwrap()
            .id;
        let mut scheduler = SpecializedActorTaskScheduler::default();
        scheduler
            .adopt_fresh_level1_type9_selected(&mut manager)
            .unwrap();
        let mut notifications = GameplayNotifications::new();
        let death = manager.apply_fun_00411250_type9_checked_damage(
            target,
            FUN_0043F780_DAMAGE_PACKET,
            &mut world_fx,
            1,
            Some(&mut notifications),
        );
        let Fun00411250Type9DamageOutcome::Lethal {
            task_lease: Some(lease),
            ..
        } = death
        else {
            panic!("level{level_id} checked Type9 death: {death:?}");
        };
        scheduler
            .adopt_ordinary_type9_class14_after_checked_death(lease)
            .unwrap();
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == target)
            .unwrap();
        assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
        assert_eq!(
            entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT),
            RetailRuntimeValue::Known(DYING_STATE_BIT)
        );
        let mut static_damage = StaticDamageScheduler::default();
        let mut removed = false;
        for step in 1..=60 {
            let pass = scheduler.tick(
                &mut manager,
                SpecializedActorTaskProductionFrame {
                    world: crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                    hive_components: None,
                    notification_phase:
                        crate::gameplay_notifications::GameplayNotificationPhase::Playing,
                    resources: &mut session.cache,
                    world_fx: &mut world_fx,
                    static_damage: &mut static_damage,
                    elapsed_micros: 100_000,
                    global_elapsed_micros: 100_000,
                    retail_tick: step * 5,
                    main_base_abort_active: false,
                },
                &mut notifications,
            );
            assert!(pass.block.is_none(), "level{level_id}: {:?}", pass.block);
            for outcome in &pass.outcomes {
                assert_not_blocked(level_id, outcome);
            }
            removed |= manager
                .cleanup_pending_actor_deferred_destroys()
                .contains(&target);
            if removed {
                break;
            }
        }
        assert!(
            removed,
            "level{level_id}: class14 retained its killed allocation"
        );
        assert!(manager.iter_all().all(|entity| entity.id != target));
    }
}

#[v2k_test_support::retail_test]
fn native_receipt_cannot_authorize_another_manager_allocation() {
    let Some((_session, first)) = load(14, &mut WorldFx::new()) else {
        return;
    };
    let Some((_other_session, mut second)) = load(14, &mut WorldFx::new()) else {
        return;
    };
    let first_actor = first
        .iter_all()
        .find(|entity| entity.entity_type == 9)
        .unwrap();
    let id = first_actor.id;
    second.entity_mut(id).unwrap().ordinary_type9_native_receipt =
        first_actor.ordinary_type9_native_receipt;
    assert!(!ordinary_type9_native_allocation_authenticates(&second, id));
    let before_state = second.entity_mut(id).unwrap().collision.state_flags_at_0x08;
    let before_buffer = second
        .entity_mut(id)
        .unwrap()
        .collision
        .pre_health_damage_buffer_raw;
    let mut fx = WorldFx::new();
    let mut control_fx = WorldFx::new();
    assert_eq!(
        second.apply_fun_00411250_type9_checked_damage(
            id,
            FUN_0043F780_DAMAGE_PACKET,
            &mut fx,
            0,
            None
        ),
        Fun00411250Type9DamageOutcome::Unresolved
    );
    assert_eq!(
        second.entity_mut(id).unwrap().collision.health_raw,
        RetailRuntimeValue::Known(1500)
    );
    assert_eq!(
        second
            .entity_mut(id)
            .unwrap()
            .collision
            .pre_health_damage_buffer_raw,
        before_buffer
    );
    assert_eq!(
        second.entity_mut(id).unwrap().collision.state_flags_at_0x08,
        before_state
    );
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        control_fx.next_shared_retail_random_u16()
    );
    let result = second.publish_ordinary_type9_standard_death(
        id,
        OrdinaryType9StandardDeathEntry::GenericDeath,
        MainBaseType9ResultScreenState::NotShown,
        &mut WorldFx::new(),
        0,
        None,
    );
    assert_eq!(
        result,
        Err(OrdinaryType9StandardDeathBlock::NativeAllocationMismatch)
    );
    assert_eq!(
        second.entity_mut(id).unwrap().collision.health_raw,
        RetailRuntimeValue::Known(1500)
    );
}
