//! Fresh peasant -> real beam/None owner -> materializer -> live tasks/Base.

use v2k_game::actor_task_dispatcher::ActorTaskRuntime;
use v2k_game::actor_task_owner::ActorTaskSlot;
use v2k_game::entity::{
    AuthoredPlayerArrival, AuthoredWorldConstruction, BeamCommand, BeamOutcome,
    CampaignCargoRestoreContext, CargoDropContext, EntityConstructionResources, EntityManager,
    LateTailMaterialiserFrame, PlayerCargoFrame,
};
use v2k_game::entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue};
use v2k_game::gameplay_notifications::GameplayNotifications;
use v2k_game::main_base_conversion::MainBaseConversionActionOutcome;
use v2k_game::main_base_conversion_live::{
    resolve_first_world_main_base_conversions, FirstWorldMainBaseConversionPass,
    MainBaseConversionFrame,
};
use v2k_game::retail_clock::RetailTickClock;
use v2k_game::session::GameSession;
use v2k_game::specialized_actor_task_production::{
    SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
    SpecializedActorTaskScheduler,
};
use v2k_game::static_damage::StaticDamageScheduler;
use v2k_game::world_fx::WorldFx;

struct World {
    session: GameSession,
    entities: EntityManager,
    scheduler: SpecializedActorTaskScheduler,
    effects: WorldFx,
    static_damage: StaticDamageScheduler,
    notifications: GameplayNotifications,
    tick: u32,
    clock: RetailTickClock,
}

impl World {
    fn new() -> Self {
        let data = v2k_test_support::retail_dir();
        assert!(
            data.join("PRELOAD.DAT").exists(),
            "canonical retail corpus required"
        );
        let mut session = GameSession::init(&data).unwrap();
        session.load_auxiliary_ovl(3, 1).unwrap();
        session.load_level_by_id(13, 1).unwrap();
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
        let mut effects = WorldFx::new();
        let mut entities = EntityManager::from_fresh_new_game_level_with_type_metadata(
            session.cache.level_desc().unwrap(),
            &metadata,
            EntityConstructionResources::new(
                session.cache.terrain(),
                session.cache.terrain_objects(),
            ),
            0,
            &mut effects,
        )
        .unwrap();
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert_eq!(
            scheduler
                .adopt_fresh_level1_type9_selected(&mut entities)
                .unwrap(),
            6
        );
        Self {
            session,
            entities,
            scheduler,
            effects,
            static_damage: StaticDamageScheduler::new(),
            notifications: GameplayNotifications::new(),
            tick: 0,
            clock: RetailTickClock::new(),
        }
    }

    fn replace_with_authored_level(self, level_id: u32) -> Self {
        let Self {
            mut session,
            mut effects,
            ..
        } = self;
        // The world-local owners are discarded; the actual process RNG and
        // Sub-D allocation cursor survive the same clear used by production.
        effects.clear();
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
        let mut entities = EntityManager::from_authored_world(
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
                player_arrival: Some(AuthoredPlayerArrival {
                    position_raw: [19_712, -500, 14_848],
                    heading_raw: 0x4000,
                }),
                retail_tick: 0,
            },
            &mut effects,
        )
        .unwrap();
        let peasant_count = entities
            .iter_all()
            .filter(|entity| entity.entity_type == 9)
            .count();
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert_eq!(
            scheduler
                .adopt_fresh_level1_type9_selected(&mut entities)
                .unwrap(),
            peasant_count
        );
        Self {
            session,
            entities,
            scheduler,
            effects,
            static_damage: StaticDamageScheduler::new(),
            notifications: GameplayNotifications::new(),
            tick: 0,
            clock: RetailTickClock::new(),
        }
    }

    fn step(
        &mut self,
    ) -> (
        Option<BeamOutcome>,
        Vec<SpecializedActorTaskProductionOutcome>,
    ) {
        self.step_elapsed(20_000)
    }

    fn step_elapsed(
        &mut self,
        elapsed_micros: u32,
    ) -> (
        Option<BeamOutcome>,
        Vec<SpecializedActorTaskProductionOutcome>,
    ) {
        // Keep main.rs's entry animation claims across synchronous cargo
        // replacement, the actor pass, and unclaimed-animation fallback.
        let mut claims: Vec<_> = self.scheduler.actor_animation_claims().collect();
        let cargo = self.entities.update_player_cargo(PlayerCargoFrame {
            elapsed_micros,
            drop_context: Some(CargoDropContext {
                carrier_orientation: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
                terrain: self.session.cache.terrain().unwrap(),
            }),
            retail_tick: self.tick,
            scheduler: &mut self.scheduler,
            world_fx: &mut self.effects,
            notifications: &mut self.notifications,
        });
        assert!(
            cargo.blocked.is_empty(),
            "cargo at {}: {:?}",
            self.tick,
            cargo.blocked
        );
        for claim in self.scheduler.actor_animation_claims() {
            if !claims.contains(&claim) {
                claims.push(claim);
            }
        }
        let pass = self.scheduler.tick(
            &mut self.entities,
            SpecializedActorTaskProductionFrame {
                world: v2k_game::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                hive_components: None,
                notification_phase:
                    v2k_game::gameplay_notifications::GameplayNotificationPhase::Playing,
                resources: &mut self.session.cache,
                world_fx: &mut self.effects,
                static_damage: &mut self.static_damage,
                elapsed_micros,
                global_elapsed_micros: elapsed_micros,
                retail_tick: self.tick,
                main_base_abort_active: false,
            },
            &mut self.notifications,
        );
        assert!(pass.block.is_none(), "actor pass: {:?}", pass.block);
        for outcome in &pass.outcomes {
            let diagnostic = format!("{outcome:?}");
            assert!(
                !diagnostic.contains("Blocked") && !diagnostic.contains("Dropped"),
                "actor at {}: {diagnostic}; Main Base targets: {:?}",
                self.tick,
                self.entities
                    .iter_all()
                    .filter(|entity| entity.entity_type == 6)
                    .map(|entity| (
                        entity.id,
                        entity.position_raw(),
                        entity.collision.state_flags_at_0x08
                    ))
                    .collect::<Vec<_>>()
            );
        }
        self.entities
            .advance_unclaimed_actor_animations(elapsed_micros, &claims);
        let blocked = self
            .entities
            .update_late_tail_materialisers(LateTailMaterialiserFrame {
                elapsed_micros,
                terrain: self.session.cache.terrain().unwrap(),
                world_fx: &mut self.effects,
                scheduler: &mut self.scheduler,
                notifications: &mut self.notifications,
                retail_tick: self.tick,
            });
        assert!(
            blocked.is_empty(),
            "late materialiser at {}: {blocked:?}",
            self.tick
        );
        self.tick = self.clock.advance(u64::from(elapsed_micros));
        (cargo.beam, pass.outcomes)
    }

    fn step_presented(
        &mut self,
        elapsed_micros: u32,
    ) -> (
        Option<BeamOutcome>,
        Vec<SpecializedActorTaskProductionOutcome>,
    ) {
        self.scheduler.adopt_live_type8_go_to_job(&self.entities);
        self.scheduler
            .adopt_level_one_factory_arrival(&self.entities);
        let outcome = self.step_elapsed(elapsed_micros);
        let observed = self.entities.player().unwrap().position;
        let scan = v2k_render::terrain_tiles::scan_dimensions(
            self.session.cache.level_desc().unwrap().terrain_draw_depth,
        );
        self.scheduler.publish_presented_view_detail(
            &mut self.entities,
            v2k_game::entity_view_detail::RetailViewDetailContext::from_world(
                [observed[0], observed[1] + 8.0, observed[2] - 8.0],
                0.7,
                scan,
            ),
        );
        outcome
    }

    fn collect_after_root_reselection(&mut self) -> u32 {
        for _ in 0..400 {
            self.step();
        }
        let peasant = self
            .entities
            .iter_all()
            .find(|e| e.entity_type == 9)
            .unwrap();
        let id = peasant.id;
        let position = peasant.position;
        self.entities.player_mut().unwrap().position =
            [position[0] + 0.25, position[1] - 1., position[2]];
        self.entities.queue_beam(BeamCommand::Collect);
        let mut collected = false;
        let mut carried_visits = 0;
        for _ in 0..24 {
            let (beam, outcomes) = self.step();
            if let Some(outcome) = beam {
                assert_eq!(outcome, BeamOutcome::Collected { entity_id: id });
                collected = true;
            }
            carried_visits += outcomes
                .iter()
                .filter(|o| {
                    matches!(
                        o,
                        SpecializedActorTaskProductionOutcome::OrdinaryType9Carried(_)
                    ) && o.entity_id() == id
                })
                .count();
        }
        assert!(
            collected && carried_visits >= 20,
            "collection must install a continuing carried scheduler owner"
        );
        let entity = self.entities.iter_all().find(|e| e.id == id).unwrap();
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::None)
        ));
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
        let RetailRuntimeValue::Known(Some(animation)) = entity.actor_animation_runtime else {
            panic!("Sub-I")
        };
        assert_eq!(
            animation.linked_handle(),
            self.entities.player().map(|e| e.id)
        );
        id
    }

    fn drop_at(&mut self, id: u32, x: f32, z: f32) -> [i16; 3] {
        // Level carrier facing +Z: the original low-altitude branch drops
        // 400 raw units behind it. Move the player, never the carried peasant.
        let height = self.session.cache.terrain().unwrap().height_at(x, z);
        self.entities.player_mut().unwrap().position = [x, height - 1., z + 400. / 256.];
        self.entities.queue_beam(BeamCommand::Drop);
        let mut proxy = None;
        for _ in 0..100 {
            let (beam, _) = self.step();
            if let Some(outcome) = beam {
                let BeamOutcome::DropStarted { cargo_id, proxy_id } = outcome else {
                    panic!("drop: {outcome:?}")
                };
                assert_eq!(cargo_id, id);
                proxy = Some(proxy_id);
            }
            let entity = self.entities.iter_all().find(|e| e.id == id).unwrap();
            if proxy.is_some() && entity.attached_to.is_none() {
                assert!(!matches!(
                    entity.actor_task_state(ActorTaskSlot::Primary),
                    Some(ActorTaskRuntime::None)
                ));
                assert!(!self.entities.iter_all().any(|e| Some(e.id) == proxy));
                return entity.position_raw();
            }
        }
        panic!("materializer did not release");
    }
}

#[v2k_test_support::retail_test]
fn campaign_restore_skips_a_real_carried_peasant_and_keeps_each_authored_cohort() {
    use v2k_game::ordinary_type9_carried_production::Type9CarriedProductionOutcome;

    // Exercise the shared loader after a real first-world construction, then
    // the same restoration policy in 14 -> 39 -> 14. This is a controller
    // restoration fixture, not a claim that those worlds share a retail gate.
    let mut world = World::new().replace_with_authored_level(14);
    let id = world
        .entities
        .iter_all()
        .find(|entity| entity.entity_type == 9)
        .unwrap()
        .id;
    let initial_task = world
        .entities
        .iter_all()
        .find(|entity| entity.id == id)
        .unwrap()
        .actor_task_state(ActorTaskSlot::Primary);
    assert!(initial_task.is_some() && !matches!(initial_task, Some(ActorTaskRuntime::None)));
    world.entities.queue_beam(BeamCommand::Collect);
    let mut collected = false;
    let mut carrying_callbacks = 0;
    for _ in 0..30 {
        if !collected {
            let position = world
                .entities
                .iter_all()
                .find(|entity| entity.id == id)
                .unwrap()
                .position;
            world.entities.player_mut().unwrap().position =
                [position[0] + 0.25, position[1] - 1.0, position[2]];
        }
        let (beam, outcomes) = world.step();
        if let Some(beam) = beam {
            assert_eq!(beam, BeamOutcome::Collected { entity_id: id });
            collected = true;
        }
        carrying_callbacks += outcomes
            .iter()
            .filter(|outcome| {
                matches!(outcome,
                SpecializedActorTaskProductionOutcome::OrdinaryType9Carried(
                    Type9CarriedProductionOutcome::Continuing {
                        entity_id, callback_enabled: true, ..
                    }
                ) if *entity_id == id)
            })
            .count();
    }
    assert!(
        collected && carrying_callbacks > 0,
        "the beam must publish a live carried owner"
    );
    let peasant = world
        .entities
        .iter_all()
        .find(|entity| entity.id == id)
        .unwrap();
    assert!(peasant.authored_spawn_index.is_some());
    assert_eq!(
        peasant.attached_to,
        world.entities.player().map(|entity| entity.id)
    );
    assert!(matches!(
        peasant.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::None)
    ));
    assert!(peasant.actor_task_state(ActorTaskSlot::Secondary).is_none());
    assert!(peasant.actor_task_state(ActorTaskSlot::Tertiary).is_none());
    let RetailRuntimeValue::Known(Some(animation)) = peasant.actor_animation_runtime else {
        panic!("carried peasant owns Sub-I")
    };
    assert_eq!(
        animation.linked_handle(),
        world.entities.player().map(|entity| entity.id)
    );
    let RetailRuntimeValue::Known(mut state) = world.entities.campaign_cargo_controller_state()
    else {
        panic!("source controller snapshot")
    };
    assert_eq!(
        state.occupied_slots(),
        1,
        "43260 serializes the directly carried peasant"
    );
    let unlock_raw = state.unlock_raw;

    for level_id in [39, 14] {
        world = world.replace_with_authored_level(level_id);
        let before: Vec<_> = world
            .entities
            .iter_all()
            .map(|entity| (entity.id, entity.entity_type, entity.authored_spawn_index))
            .collect();
        let authored_peasants: Vec<_> = world
            .session
            .cache
            .level_desc()
            .unwrap()
            .entities
            .iter()
            .filter(|spawn| spawn.entity_type == 9)
            .map(|spawn| Some(spawn.index))
            .collect();
        assert!(
            !authored_peasants.is_empty(),
            "destination{level_id} must contain its own peasants"
        );
        world
            .entities
            .restore_campaign_cargo_controller_state(
                state,
                CampaignCargoRestoreContext {
                    level: world.session.cache.level_desc().unwrap(),
                    resources: EntityConstructionResources::new(
                        world.session.cache.terrain(),
                        world.session.cache.terrain_objects(),
                    ),
                    retail_tick: world.tick,
                    scheduler: &mut world.scheduler,
                    world_fx: &mut world.effects,
                    notifications: &mut world.notifications,
                },
            )
            .unwrap();
        assert_eq!(
            world
                .entities
                .iter_all()
                .map(|entity| (entity.id, entity.entity_type, entity.authored_spawn_index))
                .collect::<Vec<_>>(),
            before,
            "51C00's type-9 skip must neither append cargo nor remove an authored actor"
        );
        assert_eq!(
            world
                .entities
                .iter_all()
                .filter(|entity| entity.entity_type == 9)
                .map(|entity| entity.authored_spawn_index)
                .collect::<Vec<_>>(),
            authored_peasants
        );
        assert!(world
            .entities
            .iter_all()
            .filter(|entity| entity.entity_type == 9)
            .all(|entity| entity.attached_to.is_none()));
        let RetailRuntimeValue::Known(Some(sub_j)) =
            &world.entities.player().unwrap().sub_j_attachment_runtime
        else {
            panic!("destination player owns Sub-J")
        };
        assert!(sub_j.ordered_entity_ids().is_empty());
        let RetailRuntimeValue::Known(next) = world.entities.campaign_cargo_controller_state()
        else {
            panic!("destination controller snapshot")
        };
        assert_eq!(
            next.occupied_slots(),
            0,
            "51710 reserializes the restored direct list"
        );
        assert_eq!(next.unlock_raw, unlock_raw);
        state = next;
    }
}

#[v2k_test_support::retail_test]
fn moving_attention_peasant_can_be_collected_after_presented_task_visits() {
    use v2k_game::entity_view_detail::RetailViewDetailContext;

    let mut world = World::new();
    assert_eq!(
        world
            .scheduler
            .adopt_fresh_level1_type17_follow_beacons(&world.entities),
        1
    );
    assert_eq!(
        world
            .scheduler
            .adopt_level_one_factory_arrival(&world.entities),
        1
    );
    assert_eq!(
        world
            .scheduler
            .adopt_fresh_level1_type47_scheduler(&mut world.entities)
            .unwrap(),
        3
    );
    let id = world
        .entities
        .iter_all()
        .find(|entity| entity.entity_type == 9)
        .unwrap()
        .id;
    let scan = v2k_render::terrain_tiles::scan_dimensions(
        world.session.cache.level_desc().unwrap().terrain_draw_depth,
    );
    let mut following_moves = 0;
    let mut queued = false;
    let mut collected = false;
    let mut carried_visits = 0;

    // The original crash occurred between custody preparation and the Sub-J
    // append. Reach a real retained follow receipt through authored tasks and
    // presentation; fresh/coarse-only cargo fixtures do not exercise it.
    for _ in 0..1_250 {
        let entity = world
            .entities
            .iter_all()
            .find(|entity| entity.id == id)
            .unwrap();
        let position = entity.position;
        let before = entity.position_raw();
        let following = matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::AttractAttentionTargetRoute(_))
        );
        if !collected {
            world.entities.player_mut().unwrap().position =
                [position[0] + 0.25, position[1] - 1.0, position[2]];
        }
        if following_moves >= 2 && !queued {
            world.entities.queue_beam(BeamCommand::Collect);
            queued = true;
        }
        world.scheduler.adopt_live_type8_go_to_job(&world.entities);
        world
            .scheduler
            .adopt_level_one_factory_arrival(&world.entities);
        let (beam, outcomes) = world.step();
        let entity = world
            .entities
            .iter_all()
            .find(|entity| entity.id == id)
            .unwrap();
        let after = entity.position_raw();
        if following && [before[0], before[2]] != [after[0], after[2]] {
            following_moves += 1;
        }
        if let Some(beam) = beam {
            assert_eq!(beam, BeamOutcome::Collected { entity_id: id });
            collected = true;
        }
        carried_visits += outcomes
            .iter()
            .filter(|outcome| {
                outcome.entity_id() == id
                    && matches!(
                        outcome,
                        SpecializedActorTaskProductionOutcome::OrdinaryType9Carried(_)
                    )
            })
            .count();
        if carried_visits >= 20 {
            assert!(collected && following_moves >= 2);
            assert_eq!(
                entity.attached_to,
                world.entities.player().map(|player| player.id)
            );
            assert!(matches!(
                entity.actor_task_state(ActorTaskSlot::Primary),
                Some(ActorTaskRuntime::None)
            ));
            assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
            assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
            return;
        }
        let observed = world.entities.player().unwrap().position;
        world.scheduler.publish_presented_view_detail(
            &mut world.entities,
            RetailViewDetailContext::from_world(
                [observed[0], observed[1] + 8.0, observed[2] - 8.0],
                0.7,
                scan,
            ),
        );
    }
    panic!("follow/collection did not complete: moves={following_moves}, queued={queued}, collected={collected}, carried={carried_visits}");
}

#[v2k_test_support::retail_test]
fn peasant_reacquires_nearby_player_after_an_earlier_attention_target() {
    use v2k_game::entity_view_detail::RetailViewDetailContext;
    use v2k_game::ordinary_type9_attract_attention_production::OrdinaryType9AttractAttentionProductionOutcome;
    use v2k_game::ordinary_type9_go_to_job_production::OrdinaryType9GoToJobProductionOutcome;
    use v2k_game::ordinary_type9_wander_production::OrdinaryType9WanderProductionOutcome;

    let mut world = World::new();
    assert_eq!(
        world
            .scheduler
            .adopt_fresh_level1_type17_follow_beacons(&world.entities),
        1
    );
    assert_eq!(
        world
            .scheduler
            .adopt_level_one_factory_arrival(&world.entities),
        1
    );
    assert_eq!(
        world
            .scheduler
            .adopt_fresh_level1_type47_scheduler(&mut world.entities)
            .unwrap(),
        3
    );
    let id = world
        .entities
        .iter_all()
        .find(|entity| entity.entity_type == 9)
        .unwrap()
        .id;
    let player_id = world.entities.player().unwrap().id;
    let scan = v2k_render::terrain_tiles::scan_dimensions(
        world.session.cache.level_desc().unwrap().terrain_draw_depth,
    );
    let mut acquisitions = 0;
    let mut waiting_candidate_since = None;

    // Exercise the authored initial population and shared RNG, changing only
    // player/camera inputs. The first follow leaves the player's handle in
    // behavior +0x08. A later odd Attract initializer must still acquire at
    // the common-axis radius; that retained handle is not a distance limit.
    for frame in 0..1_250 {
        let entity = world
            .entities
            .iter_all()
            .find(|entity| entity.id == id)
            .unwrap();
        let position = entity.position;
        let context_before = entity.current_behavior_context;
        world.entities.player_mut().unwrap().position =
            [position[0] + 0.25, position[1] - 1.0, position[2]];
        world.scheduler.adopt_live_type8_go_to_job(&world.entities);
        world
            .scheduler
            .adopt_level_one_factory_arrival(&world.entities);
        let (_, outcomes) = world.step();
        let entity = world
            .entities
            .iter_all()
            .find(|entity| entity.id == id)
            .unwrap();
        for outcome in outcomes {
            let (entity_id, target_id) = match outcome {
                SpecializedActorTaskProductionOutcome::OrdinaryType9AttractAttention(
                    OrdinaryType9AttractAttentionProductionOutcome::TargetRoutePublished {
                        entity_id,
                        target_id,
                        ..
                    },
                )
                | SpecializedActorTaskProductionOutcome::OrdinaryType9Wander(
                    OrdinaryType9WanderProductionOutcome::RootAttractAttentionTargetRoutePublished {
                        entity_id,
                        target_id,
                        ..
                    },
                )
                | SpecializedActorTaskProductionOutcome::OrdinaryType9GoToJob(
                    OrdinaryType9GoToJobProductionOutcome::RootAttractAttentionTargetRoutePublished {
                        entity_id,
                        target_id,
                        ..
                    },
                ) => (entity_id, target_id),
                _ => continue,
            };
            if entity_id != id {
                continue;
            }
            assert_eq!(target_id, player_id);
            assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
            assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
            let RetailRuntimeValue::Known(Some(animation)) = entity.actor_animation_runtime else {
                panic!("target publication retains Sub-I")
            };
            assert!(!animation.forced_stop(), "acquisition retires the help cue");
            if acquisitions != 0 {
                let RetailRuntimeValue::Known(Some(context)) = context_before else {
                    panic!("previous target context survives root reselection")
                };
                assert_eq!(
                    context.target_handle_at_0x08(),
                    RetailRuntimeValue::Known(Some(player_id))
                );
                return;
            }
            acquisitions += 1;
        }
        if acquisitions != 0
            && matches!(
                entity.actor_task_state(ActorTaskSlot::Secondary),
                Some(ActorTaskRuntime::AttractAttentionCandidate(_))
            )
        {
            let first_wait = *waiting_candidate_since.get_or_insert(frame);
            assert!(
                frame - first_wait < 50,
                "a nearby eligible player must not leave the repeated candidate stalled for the entire help cue"
            );
        } else {
            waiting_candidate_since = None;
        }
        let observed = world.entities.player().unwrap().position;
        world.scheduler.publish_presented_view_detail(
            &mut world.entities,
            RetailViewDetailContext::from_world(
                [observed[0], observed[1] + 8.0, observed[2] - 8.0],
                0.7,
                scan,
            ),
        );
    }
    panic!("expected an initial and repeated player acquisition; observed {acquisitions}");
}

#[v2k_test_support::retail_test]
fn presented_peasant_animates_help_then_resumes_directional_walking() {
    use std::collections::BTreeSet;
    use v2k_game::actor_animation::actor_direction_bin;
    use v2k_game::entity_view_detail::{RetailViewDetailContext, BROADER_DETAIL_STATE_BIT};

    let mut world = World::new();
    // Include main.rs's other initial actor families so their process-RNG
    // consumption participates in the same natural Type-9 root decisions.
    assert_eq!(
        world
            .scheduler
            .adopt_fresh_level1_type17_follow_beacons(&world.entities),
        1
    );
    assert_eq!(
        world
            .scheduler
            .adopt_level_one_factory_arrival(&world.entities),
        1
    );
    assert_eq!(
        world
            .scheduler
            .adopt_fresh_level1_type47_scheduler(&mut world.entities)
            .unwrap(),
        3
    );
    let id = world
        .entities
        .iter_all()
        .find(|entity| entity.entity_type == 9)
        .unwrap()
        .id;
    let player_start = world.entities.player().unwrap().position;
    let scan = v2k_render::terrain_tiles::scan_dimensions(
        world.session.cache.level_desc().unwrap().terrain_draw_depth,
    );
    let mut camera = v2k_render::Camera::new(4.0 / 3.0);
    // A fixed -Z test observer first follows the player, then remains by the
    // peasant as the player leaves. Its exact view matrix feeds the production
    // presentation classifier; no scheduler bits are forged.
    camera.yaw = std::f32::consts::PI;
    camera.pitch = -std::f32::consts::FRAC_PI_4;
    camera.left_handed = true;
    let mut help_frames = BTreeSet::new();
    let mut resumed_frames = BTreeSet::new();
    let mut resumed_directions = BTreeSet::new();
    let mut resumed_moves = 0;
    let mut later_moves = 0;
    let mut coarse_animation = None;
    let mut saw_help = false;
    let mut leave_at = None;
    let mut previous_position = world
        .entities
        .iter_all()
        .find(|entity| entity.id == id)
        .unwrap()
        .position_raw();

    for frame in 0..3_000 {
        if frame >= 400 && leave_at.is_none_or(|leave_at| frame < leave_at) {
            let position = world
                .entities
                .iter_all()
                .find(|entity| entity.id == id)
                .unwrap()
                .position;
            world.entities.player_mut().unwrap().position =
                [position[0] + 0.25, position[1] - 1.0, position[2]];
        } else if leave_at == Some(frame) {
            world.entities.player_mut().unwrap().position = player_start;
        }
        world.step();
        let entity = world
            .entities
            .iter_all()
            .find(|entity| entity.id == id)
            .unwrap();
        let position = entity.position_raw();
        let moved = position[0] != previous_position[0] || position[2] != previous_position[2];
        previous_position = position;
        let RetailRuntimeValue::Known(Some(animation)) = entity.actor_animation_runtime else {
            panic!("the retained peasant owns its authored Sub-I controller")
        };
        let detailed = entity
            .collision
            .state_flags_at_0x08
            .masked(BROADER_DETAIL_STATE_BIT)
            == RetailRuntimeValue::Known(BROADER_DETAIL_STATE_BIT);
        if frame >= 400 && animation.forced_stop() && (32..=35).contains(&animation.output()) {
            assert!(
                detailed,
                "help output must come from the nearby live callback"
            );
            saw_help = true;
            help_frames.insert(animation.output());
            if help_frames.len() >= 3 && leave_at.is_none() {
                leave_at = Some(frame + 50);
            }
        }
        // 420830 deliberately preserves the last published help output until
        // the next Sub-I visit; require eventual live walking publication.
        if saw_help
            && detailed
            && animation.is_neutral_runtime()
            && animation.output() < 32
            && moved
        {
            assert_eq!(
                animation.output() / 4,
                u16::from(actor_direction_bin(entity.heading_raw())),
                "frame {frame}: walking must follow the live heading"
            );
            resumed_frames.insert(animation.phase());
            resumed_directions.insert(animation.cached_direction());
            resumed_moves += 1;
        }
        if frame >= 2_900 {
            assert!(
                !detailed,
                "the final distant observer selects coarse update"
            );
            let sample = (animation.output(), animation.phase());
            assert_eq!(
                *coarse_animation.get_or_insert(sample),
                sample,
                "coarse update preserves the last presentation instead of advancing Sub-I"
            );
            if moved {
                later_moves += 1;
            }
        }

        // Keep the observer near after the player leaves, as with a detached
        // gameplay camera, so the cue destructor and subsequent walking are
        // visible. The final four seconds return the observer to the player.
        let observed = if (400..2_800).contains(&frame) {
            world
                .entities
                .iter_all()
                .find(|entity| entity.id == id)
                .unwrap()
                .position
        } else {
            world.entities.player().unwrap().position
        };
        camera.position = [observed[0], observed[1] + 8.0, observed[2] - 8.0];
        world.scheduler.publish_presented_view_detail(
            &mut world.entities,
            RetailViewDetailContext::from_world(camera.position, camera.view_matrix()[9], scan),
        );
    }

    assert!(
        help_frames.len() >= 3,
        "nearby help must animate: {help_frames:?}"
    );
    assert!(resumed_moves >= 8 && resumed_frames.len() >= 2 && resumed_directions.len() >= 2,
        "retired help must resume moving direction/frames: moves={resumed_moves}, phases={resumed_frames:?}, directions={resumed_directions:?}");
    assert!(
        later_moves > 0,
        "the same peasant must still move near the end of the 60-second run"
    );
    assert!(world
        .scheduler
        .actor_animation_claims()
        .any(|lease| lease.entity_id == id));
}

#[v2k_test_support::retail_test]
fn nonlocal_or_unresolved_cargo_drop_keeps_relation_tasks_and_rng() {
    for remote_known in [true, false] {
        let mut world = World::new();
        let peasant = world
            .entities
            .iter_all()
            .find(|entity| entity.entity_type == 9)
            .unwrap();
        let (id, position) = (peasant.id, peasant.position);
        world.entities.player_mut().unwrap().position =
            [position[0] + 0.25, position[1] - 1., position[2]];
        world.entities.queue_beam(BeamCommand::Collect);
        let collected = world.entities.update_player_cargo(PlayerCargoFrame {
            elapsed_micros: 80_000,
            drop_context: Some(CargoDropContext {
                carrier_orientation: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
                terrain: world.session.cache.terrain().unwrap(),
            }),
            retail_tick: 0,
            scheduler: &mut world.scheduler,
            world_fx: &mut world.effects,
            notifications: &mut world.notifications,
        });
        assert_eq!(
            collected.beam,
            Some(BeamOutcome::Collected { entity_id: id })
        );
        assert!(collected.blocked.is_empty());
        let player_id = world.entities.player().unwrap().id;
        let terrain_y = world
            .session
            .cache
            .terrain()
            .unwrap()
            .height_at(position[0], position[2]);
        world.entities.player_mut().unwrap().position =
            [position[0], terrain_y - 1., position[2] + 400. / 256.];
        let entity = world.entities.entity_mut(id).unwrap();
        if remote_known {
            entity
                .collision
                .state_flags_at_0x08
                .overwrite(0x8000_0000, 0x8000_0000);
        } else {
            entity.collision.state_flags_at_0x08.invalidate(0x8000_0000);
        }
        let state = entity.collision.state_flags_at_0x08;
        let context = entity.current_behavior_context;
        let animation = entity.actor_animation_runtime;
        let tasks =
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_task_state(slot).copied());
        let entity_ids: Vec<_> = world.entities.iter_all().map(|entity| entity.id).collect();
        let RetailRuntimeValue::Known(Some(sub_j)) =
            &world.entities.player().unwrap().sub_j_attachment_runtime
        else {
            panic!()
        };
        let attachments = sub_j.ordered_entity_ids().to_vec();
        assert_eq!(attachments, [id]);
        // Isolate the drop boundary's process cursor; neither the beam delay nor
        // a rejected local constructor may consume even its singleton selector.
        let mut drop_effects = WorldFx::new();
        let mut untouched_effects = WorldFx::new();
        world.entities.queue_beam(BeamCommand::Drop);
        let dropped = world.entities.update_player_cargo(PlayerCargoFrame {
            elapsed_micros: 80_000,
            drop_context: Some(CargoDropContext {
                carrier_orientation: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
                terrain: world.session.cache.terrain().unwrap(),
            }),
            retail_tick: 1,
            scheduler: &mut world.scheduler,
            world_fx: &mut drop_effects,
            notifications: &mut world.notifications,
        });
        assert_eq!(dropped.beam, Some(BeamOutcome::DropConstructionUnavailable));
        assert!(!dropped.blocked.is_empty());
        assert_eq!(
            drop_effects.next_shared_retail_random_u16(),
            untouched_effects.next_shared_retail_random_u16()
        );
        assert_eq!(
            world
                .entities
                .iter_all()
                .map(|entity| entity.id)
                .collect::<Vec<_>>(),
            entity_ids
        );
        let entity = world
            .entities
            .iter_all()
            .find(|entity| entity.id == id)
            .unwrap();
        assert_eq!(entity.attached_to, Some(player_id));
        assert_eq!(entity.collision.state_flags_at_0x08, state);
        assert_eq!(entity.current_behavior_context, context);
        assert_eq!(entity.actor_animation_runtime, animation);
        assert_eq!(
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_task_state(slot).copied()),
            tasks
        );
        let RetailRuntimeValue::Known(Some(sub_j)) =
            &world.entities.player().unwrap().sub_j_attachment_runtime
        else {
            panic!()
        };
        assert_eq!(sub_j.ordered_entity_ids(), attachments);
        assert!(world.entities.take_cargo_proxy_events().is_empty());
    }
}

#[v2k_test_support::retail_test]
fn released_peasant_keeps_moving_after_another_task_expiry() {
    let mut world = World::new();
    let id = world.collect_after_root_reselection();
    let position = world
        .entities
        .iter_all()
        .find(|e| e.id == id)
        .unwrap()
        .position;
    let landing = world.drop_at(id, position[0] + 3., position[2]);
    let mut later_xz_moves = 0;
    let mut last = landing;
    for frame in 0..600 {
        world.step();
        let now = world
            .entities
            .iter_all()
            .find(|e| e.id == id)
            .unwrap()
            .position_raw();
        if frame >= 300 && [now[0], now[2]] != [last[0], last[2]] {
            later_xz_moves += 1;
        }
        last = now;
    }
    assert!(
        later_xz_moves > 20,
        "dropped peasant stalled after reselection: {landing:?} -> {last:?}"
    );
}

#[v2k_test_support::retail_test]
fn deep_water_drop_keeps_carried_surface_time_through_release_and_class14_cleanup() {
    use v2k_game::entity_collision_state::DYING_STATE_BIT;
    use v2k_game::ordinary_type9_carried_production::Type9CarriedProductionOutcome;
    use v2k_game::specialized_actor_task_production::SpecializedActorTaskFamily;

    let mut world = World::new();
    let id = world.collect_after_root_reselection();

    // V2000-nocd03.run's seabed area. The fixture's level carrier basis has
    // no horizontal up component, so place the real player at the target X/Z.
    // Only the player moves: beam callbacks and Type93 own every child pose,
    // relation, task replacement, timer write, and eventual death publication.
    let sea_raw = world.session.cache.level_terrain().unwrap().sea_level_raw();
    assert_eq!(sea_raw, -847);
    world.entities.player_mut().unwrap().position =
        [24_613.0, -622.0, -25_925.0].map(|raw| raw / 256.0);
    world.entities.queue_beam(BeamCommand::Drop);

    let mut proxy = None;
    let mut submerged_carried_millis = 0;
    let mut landed = false;
    for _ in 0..120 {
        // The existing child runs before its tail-appended proxy. E370 reads
        // this entry pose, not the pose18640 publishes later in the same step.
        let child_was_submerged = world
            .entities
            .iter_all()
            .find(|entity| entity.id == id)
            .unwrap()
            .position_raw()[1]
            < sea_raw - (165 >> 2);
        let (beam, outcomes) = world.step();
        if let Some(beam) = beam {
            let BeamOutcome::DropStarted { cargo_id, proxy_id } = beam else {
                panic!("deep-water drop: {beam:?}");
            };
            assert_eq!(cargo_id, id);
            assert!(proxy.replace(proxy_id).is_none());
        }
        let entity = world
            .entities
            .iter_all()
            .find(|entity| entity.id == id)
            .unwrap();
        let Some(proxy_id) = proxy else {
            continue;
        };
        let Some(SpecializedActorTaskProductionOutcome::OrdinaryType9Carried(carried)) =
            outcomes.iter().find(|outcome| outcome.entity_id() == id)
        else {
            panic!(
                "the child must keep its carried scheduler owner through the late release frame"
            );
        };
        if child_was_submerged {
            if let Type9CarriedProductionOutcome::Continuing {
                callback_elapsed_micros,
                callback_enabled: true,
                ..
            } = carried
            {
                submerged_carried_millis += callback_elapsed_micros / 1_000;
            }
        }
        if entity.attached_to == Some(proxy_id) {
            assert_eq!(
                world.scheduler.family_for(id),
                Some(SpecializedActorTaskFamily::OrdinaryType9Carried)
            );
            assert!(matches!(
                entity.actor_task_state(ActorTaskSlot::Primary),
                Some(ActorTaskRuntime::None)
            ));
        } else {
            assert!(entity.attached_to.is_none());
            assert!(
                submerged_carried_millis > 125,
                "E370 must run while the Type93 relation is live"
            );
            assert!(
                submerged_carried_millis < 5_000,
                "this capture releases before the nested carried-expiry boundary"
            );
            assert!(entity.position_raw()[1] < sea_raw - (165 >> 2));
            assert!(!matches!(
                entity.actor_task_state(ActorTaskSlot::Primary),
                Some(ActorTaskRuntime::None)
            ));
            assert_ne!(
                world.scheduler.family_for(id),
                Some(SpecializedActorTaskFamily::OrdinaryType9Carried)
            );
            assert!(!world
                .entities
                .iter_all()
                .any(|entity| entity.id == proxy_id));
            landed = true;
            break;
        }
    }
    assert!(landed, "native Type93 did not finish its landing release");

    let mut saw_death = false;
    let mut saw_class14_tick = false;
    for frame in 0..800 {
        let (_, outcomes) = world.step();
        saw_class14_tick |= outcomes.iter().any(|outcome| {
            outcome.entity_id() == id
                && matches!(
                    outcome,
                    SpecializedActorTaskProductionOutcome::OrdinaryType9Class14(_)
                )
        });
        if let Some(entity) = world.entities.iter_all().find(|entity| entity.id == id) {
            if entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT)
                == RetailRuntimeValue::Known(DYING_STATE_BIT)
            {
                if !saw_death {
                    // Observe retained timer custody through behavior, without
                    // exposing the private +48 field. A reset at landing would
                    // need another full five seconds. Allow one maximum125-ms
                    // callback gate delay when comparing wall and actor time.
                    let after_landing_millis = (frame + 1) * 20;
                    assert!(after_landing_millis < 5_000);
                    assert!(
                        after_landing_millis <= 5_000 - submerged_carried_millis + 125,
                        "carried {submerged_carried_millis}ms, death {after_landing_millis}ms after release"
                    );
                }
                saw_death = true;
                assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
                assert!(entity.attached_to.is_none());
            }
        }
        // main.rs's deferred entity sweep follows the scheduler. Complete
        // that real phase here so the new class14 owner must retire cleanly.
        let removed = world.entities.cleanup_pending_actor_deferred_destroys();
        if removed.contains(&id) {
            assert!(saw_death && saw_class14_tick);
            assert!(!world.entities.iter_all().any(|entity| entity.id == id));
            assert!(world.scheduler.family_for(id).is_none());
            assert!(!world
                .entities
                .iter_all()
                .any(|entity| Some(entity.id) == proxy));
            return;
        }
    }
    panic!("submerged released peasant did not complete class14 cleanup; carried time={submerged_carried_millis}ms, death={saw_death}, class14={saw_class14_tick}");
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NearBaseDropExpectation {
    Converts,
    DrownsBeforeReachingBase,
}

fn assert_presented_peasant_drop_outcome(
    dx: f32,
    dz: f32,
    elapsed_micros: u32,
    expected: NearBaseDropExpectation,
) {
    let mut world = World::new();
    assert_eq!(
        world
            .scheduler
            .adopt_fresh_level1_type17_follow_beacons(&world.entities),
        1
    );
    assert_eq!(
        world
            .scheduler
            .adopt_level_one_factory_arrival(&world.entities),
        1
    );
    assert_eq!(
        world
            .scheduler
            .adopt_fresh_level1_type47_scheduler(&mut world.entities)
            .unwrap(),
        3
    );
    // Preserve the authored initial population and process RNG while reaching
    // a live selected owner. Only the player moves during collection/drop.
    for _ in 0..400 {
        world.step_presented(20_000);
    }
    let peasant = world
        .entities
        .iter_all()
        .find(|entity| entity.entity_type == 9)
        .unwrap();
    let (id, position) = (peasant.id, peasant.position);
    world.entities.player_mut().unwrap().position =
        [position[0] + 0.25, position[1] - 1.0, position[2]];
    world.entities.queue_beam(BeamCommand::Collect);
    let mut collected = false;
    for _ in 0..24 {
        let (beam, _) = world.step_presented(20_000);
        if let Some(outcome) = beam {
            assert_eq!(outcome, BeamOutcome::Collected { entity_id: id });
            collected = true;
        }
    }
    assert!(collected);
    let base = world
        .entities
        .iter_all()
        .find(|entity| entity.entity_type == 6 && entity.authored_spawn_index == Some(6))
        .unwrap();
    let (base_id, base_position) = (base.id, base.position);
    let x = base_position[0] + dx;
    let z = base_position[2] + dz;
    let height = world.session.cache.terrain().unwrap().height_at(x, z);
    world.entities.player_mut().unwrap().position = [x, height - 1.0, z + 400.0 / 256.0];
    world.entities.queue_beam(BeamCommand::Drop);
    let mut proxy_id = None;
    let mut previous_released_position = None;
    let mut moved = false;
    let mut saw_go_to_job = false;
    let mut landed_at_micros = None;
    for frame in 0..(30_000_000_u32.div_ceil(elapsed_micros)) {
        let since_drop_micros = u64::from(frame + 1) * u64::from(elapsed_micros);
        let (beam, _) = world.step_presented(elapsed_micros);
        if let Some(outcome) = beam {
            let BeamOutcome::DropStarted {
                cargo_id,
                proxy_id: materializer,
            } = outcome
            else {
                panic!("drop at [{dx}, {dz}], dt={elapsed_micros}: {outcome:?}")
            };
            assert_eq!(cargo_id, id);
            proxy_id = Some(materializer);
        }
        let result = resolve_first_world_main_base_conversions(MainBaseConversionFrame {
            entities: &mut world.entities,
            actor_tasks: &mut world.scheduler,
            model_pool: &world.session.cache,
            terrain: world.session.cache.terrain(),
            world_fx: &mut world.effects,
            notifications: &mut world.notifications,
            retail_tick: world.tick,
            world_style: RetailRuntimeValue::Known(
                world.session.cache.level_desc().unwrap().world_style,
            ),
        })
        .unwrap();
        let FirstWorldMainBaseConversionPass::Resolved(pass) = result else {
            panic!("Main Base absent")
        };
        let replacement = pass
            .visits
            .iter()
            .find(|visit| visit.entity_id == id)
            .and_then(|visit| {
                visit
                    .action_outcomes
                    .iter()
                    .find_map(|action| match action {
                        MainBaseConversionActionOutcome::ReplacementSpawned {
                            replacement_entity_id,
                        } => Some(*replacement_entity_id),
                        _ => None,
                    })
            });
        if let Some(replacement) = replacement {
            assert_eq!(
                expected,
                NearBaseDropExpectation::Converts,
                "underwater approach at [{dx}, {dz}], dt={elapsed_micros} unexpectedly converted"
            );
            assert!(moved && saw_go_to_job, "conversion at [{dx}, {dz}], dt={elapsed_micros} must follow live Go-To-Job movement");
            assert!(proxy_id.is_some());
            assert!(!world
                .entities
                .iter_all()
                .any(|entity| Some(entity.id) == proxy_id));
            let scientist = world
                .entities
                .iter_all()
                .find(|entity| entity.id == replacement)
                .unwrap();
            assert_eq!(scientist.entity_type, 8);
            assert_eq!(
                scientist.collision.recent_relation_id_at_0x60,
                RetailRuntimeValue::Known(Some(base_id))
            );
            return;
        }
        let entity = world
            .entities
            .iter_all()
            .find(|entity| entity.id == id)
            .unwrap();
        if entity.attached_to.is_none() {
            assert!(
                proxy_id.is_some(),
                "detachment must follow the real Type93 drop"
            );
            landed_at_micros.get_or_insert(since_drop_micros);
            let position = entity.position_raw();
            moved |= previous_released_position.is_some_and(|previous: [i16; 3]| {
                [previous[0], previous[2]] != [position[0], position[2]]
            });
            previous_released_position = Some(position);
            saw_go_to_job |= matches!(
                entity.actor_task_state(ActorTaskSlot::Primary),
                Some(ActorTaskRuntime::GoToJob(_))
            );
        }
        if entity
            .collision
            .state_flags_at_0x08
            .masked(v2k_game::entity_collision_state::DYING_STATE_BIT)
            == RetailRuntimeValue::Known(v2k_game::entity_collision_state::DYING_STATE_BIT)
        {
            assert_eq!(
                expected,
                NearBaseDropExpectation::DrownsBeforeReachingBase,
                "conversion approach at [{dx}, {dz}], dt={elapsed_micros} unexpectedly drowned"
            );
            assert!(
                moved && saw_go_to_job,
                "drowning must follow live Go-To-Job movement"
            );
            assert!(entity.attached_to.is_none());
            assert!(!world
                .entities
                .iter_all()
                .any(|entity| Some(entity.id) == proxy_id));
            let sea_raw = world.session.cache.level_terrain().unwrap().sea_level_raw();
            let extent_raw = world.session.cache.global_model(558).unwrap().radius;
            assert!(
                i32::from(entity.position_raw()[1])
                    < i32::from(sea_raw) - i32::from(extent_raw >> 2)
            );
            assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
            assert_eq!(world.scheduler.family_for(id), Some(
                v2k_game::specialized_actor_task_production::SpecializedActorTaskFamily::OrdinaryType9Class14
            ));
            assert!(matches!(entity.current_behavior_context,
                RetailRuntimeValue::Known(Some(context)) if context.descriptor()
                    == v2k_game::entity_behavior::BehaviorDescriptorIdentity::Named(
                        &v2k_game::entity_behavior::EXPLODING_PERSON_BEHAVIOR_PROGRAM
                    )
            ));
            let after_landing_micros = since_drop_micros
                - landed_at_micros.expect("the materialiser must release before drowning");
            assert!(after_landing_micros < 5_000_000,
                "the retained carried surface timer must cause death before five more seconds: {after_landing_micros}us");
            return;
        }
    }
    panic!(
        "drop at [{dx}, {dz}], dt={elapsed_micros} did not reach {expected:?}: {:?}",
        world
            .entities
            .iter_all()
            .find(|entity| entity.id == id)
            .map(|entity| (
                entity.position_raw(),
                entity.actor_task_state(ActorTaskSlot::Primary),
                entity.current_behavior_context
            ))
    );
}

#[v2k_test_support::retail_test]
fn peasant_dropped_near_main_base_walks_into_scientist_conversion() {
    // These approaches exercise both sides of each axis, detailed
    // presentation, materializer release and different render-frame cadences.
    for elapsed_micros in [20_000, 16_667, 6_944] {
        for [dx, dz] in [
            [8.0, 0.0],
            [-8.0, 0.0],
            [0.0, 8.0],
            [0.0, -8.0],
            [12.0, 0.0],
            [0.0, 12.0],
            [0.0, -12.0],
        ] {
            assert_presented_peasant_drop_outcome(
                dx,
                dz,
                elapsed_micros,
                NearBaseDropExpectation::Converts,
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn west_coastal_peasant_approach_preserves_cadence_dependent_surface_expiry() {
    // The west12-cell target is rawY-1440, below E370's -888 threshold.
    // With 20-ms frames the peasant walks underwater long enough to exhaust
    // its five-second timer, including time carried by Type93. The two finer
    // frame cadences reach the Base first. Preserve all three exact outcomes.
    for (elapsed_micros, expected) in [
        (20_000, NearBaseDropExpectation::DrownsBeforeReachingBase),
        (16_667, NearBaseDropExpectation::Converts),
        (6_944, NearBaseDropExpectation::Converts),
    ] {
        assert_presented_peasant_drop_outcome(-12.0, 0.0, elapsed_micros, expected);
    }
}
