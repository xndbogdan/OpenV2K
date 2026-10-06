//! Ordinary Type58 uses the same particle and radial owners as its real birth.

use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    damage::{DamagePacket, FUN_0043F780_DAMAGE_PACKET, PRIMARY_PROJECTILE_DAMAGE_PACKET},
    entity::{
        AuthoredPlayerArrival, AuthoredWorldConstruction, DynamicRadialLiveBlockReason,
        DynamicRadialLivePhase, Entity, EntityConstructionResources,
    },
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    entity_view_detail::RetailViewDetailContext,
    intro2_common_dying::{Intro2CommonDyingOutcome, Intro2CommonDyingOwner},
    intro2_radial::Intro2RadialTaskCustody,
    intro2_type53::authored_tests::native_fixture,
    intro2_type58::Intro2Type58Outcome,
    player_hull::PlayerHull,
    radial_damage::RadialDamageTemplate,
    session::GameSession,
    specialized_actor_task_production::{
        PlayingRadialBlock, PlayingRadialFrame, PlayingRadialOutcome, SpecializedActorTaskFamily,
        SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
    },
    static_damage::StaticDamageScheduler,
    world_fx::BallisticDamageRequest,
};

struct World {
    session: GameSession,
    entities: EntityManager,
    scheduler: SpecializedActorTaskScheduler,
    fx: WorldFx,
    notifications: GameplayNotifications,
    tick: u32,
    id: u32,
}

impl World {
    fn new() -> Self {
        let (session, _, mut fx) = native_fixture(14);
        // Keep the helper's real process history and load a Playing manager
        // with an actual persistent player, as the primary packet records.
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
        let entities = EntityManager::from_authored_world(
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
                retail_tick: 4793,
            },
            &mut fx,
        )
        .unwrap();
        assert!(entities.player().is_some());
        let id = entities
            .iter_all()
            .find(|entity| entity.entity_type == 58)
            .unwrap()
            .id;
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert!(scheduler.adopt_intro2_type58(&entities) > 0);
        let mut world = Self {
            session,
            entities,
            scheduler,
            fx,
            notifications: GameplayNotifications::new(),
            tick: 4794,
            id,
        };
        assert_eq!(
            world.actor().collision.health_raw,
            RetailRuntimeValue::Known(7000)
        );
        assert_eq!(
            world.actor().collision.pre_health_damage_buffer_raw,
            RetailRuntimeValue::Known(0)
        );
        assert!(crate::intro2_type58::type58_manager_allocation_authenticates(&world.entities, id));
        assert!(world
            .scheduler
            .prepare_native_actor_mutation(&world.entities, id));
        world.clear_effects();
        world
    }

    fn actor(&self) -> &Entity {
        self.entities
            .iter_all()
            .find(|entity| entity.id == self.id)
            .unwrap()
    }

    fn clear_effects(&mut self) {
        self.fx.process_pending();
        self.fx.take_positional_sounds();
    }

    fn hit(&mut self, infected: bool) -> Intro2Type58ImpactOutcome {
        let impact = ParticleEntityImpact {
            source_particle_class: if infected { 5 } else { 1 },
            impact_position_argument_va: if infected { 0x004d_cf48 } else { 0 },
            target_entity_id: self.id,
            position_world: self.actor().position,
            velocity_raw: [0, 0, 8192],
            damage: Some(BallisticDamageRequest {
                packet: if infected {
                    FUN_0043F780_DAMAGE_PACKET
                } else {
                    PRIMARY_PROJECTILE_DAMAGE_PACKET
                },
                source_entity_type_at_birth: Some(46),
                source_owner_id: self.entities.player().map(|entity| entity.id),
            }),
        };
        let result = apply_shared_actor_particle_hit(
            SharedActorImpactFrame {
                resources: &self.session.cache,
                entities: &mut self.entities,
                world_fx: &mut self.fx,
                scheduler: &mut self.scheduler,
                notifications: &mut self.notifications,
                retail_tick: self.tick,
            },
            impact,
        );
        self.tick += 1;
        let Some(SharedActorImpactOutcome::Type58(outcome)) = result else {
            panic!("ordinary native Type58 must use the shared particle route: {result:?}");
        };
        outcome
    }

    fn radial(&mut self, amount: i32) -> PlayingRadialOutcome {
        let origin_raw = self.actor().position_raw();
        let result = self
            .scheduler
            .apply_playing_radial_damage(PlayingRadialFrame {
                extra_lives: crate::entity_collision_state::RetailRuntimeValue::Unresolved,
                resources: &mut self.session.cache,
                static_damage: &mut crate::static_damage::StaticDamageScheduler::new(),
                active_terminal_calls: Vec::new(),
                entities: &mut self.entities,
                player_hull: &mut PlayerHull::default(),
                origin_raw,
                template: RadialDamageTemplate {
                    inner_radius_raw: 0,
                    outer_radius_raw: 1,
                    impulse_raw: 256,
                    packet: DamagePacket::collision(amount),
                    trailing_raw: [17, 0],
                },
                world_fx: &mut self.fx,
                notifications: &mut self.notifications,
                retail_tick: self.tick,
            });
        self.tick += 1;
        result
    }

    fn step(&mut self, dying: bool) {
        let position = self.actor().position;
        self.scheduler.publish_presented_view_detail(
            &mut self.entities,
            RetailViewDetailContext::from_world(
                [position[0], position[1] + 8., position[2] - 8.],
                0.7,
                (52, 30),
            ),
        );
        let pass = self.scheduler.tick(
            &mut self.entities,
            SpecializedActorTaskProductionFrame {
                world:
                    crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                hive_components: None,
                notification_phase:
                    crate::gameplay_notifications::GameplayNotificationPhase::Playing,
                resources: &mut self.session.cache,
                world_fx: &mut self.fx,
                static_damage: &mut StaticDamageScheduler::default(),
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: self.tick,
                main_base_abort_active: false,
            },
            &mut self.notifications,
        );
        self.tick += 1;
        assert!(pass.block.is_none(), "{:?}", pass.block);
        let outcome = pass
            .outcomes
            .iter()
            .find(|outcome| outcome.entity_id() == self.id)
            .unwrap();
        if dying {
            assert!(
                matches!(
                    outcome,
                    SpecializedActorTaskProductionOutcome::Intro2CommonDying(
                        Intro2CommonDyingOutcome::Advanced {
                            terminal: false,
                            ..
                        }
                    )
                ),
                "{outcome:?}"
            );
        } else {
            assert!(
                matches!(
                    outcome,
                    SpecializedActorTaskProductionOutcome::Intro2Type58(
                        Intro2Type58Outcome::Advanced {
                            callback_enabled: true,
                            ..
                        }
                    )
                ),
                "{outcome:?}"
            );
        }
        assert!(self
            .scheduler
            .prepare_native_actor_mutation(&self.entities, self.id));
    }

    fn assert_class12(&mut self) {
        assert_eq!(
            self.actor().collision.health_raw,
            RetailRuntimeValue::Known(0)
        );
        assert!(matches!(
            self.actor().actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::CommonDying(_))
        ));
        assert!(Intro2CommonDyingOwner::adopt(&self.entities, self.id).is_ok());
        assert_eq!(
            self.scheduler.family_for(self.id),
            Some(SpecializedActorTaskFamily::Intro2CommonDying)
        );
        assert!(self
            .scheduler
            .prepare_native_actor_mutation(&self.entities, self.id));
    }

    fn snapshot(&self) -> String {
        format!(
            "{:?}",
            (
                &self.actor().collision,
                self.actor().position_raw(),
                self.actor().velocity_raw(),
                self.actor().rotation_heading_pitch_roll_raw(),
                self.actor().current_behavior_context,
                &self.actor().actor_tasks,
                &self.actor().intro2_type58_runtime,
                &self.actor().sub_h_external_frame_runtime,
                &self.scheduler,
                &self.notifications,
            )
        )
    }
}

#[v2k_test_support::retail_test]
fn ordinary_type58_primary_and_infected_entries_reselect_then_complete_class12() {
    for infected in [false, true] {
        let mut world = World::new();
        let primary = world
            .actor()
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary);
        let stamp = world.actor().collision.last_hit_presentation_tick_at_0x34;
        let tick = world.tick;
        let RetailRuntimeValue::Known(profile) = world.actor().collision.damage_profile else {
            panic!()
        };
        let packet = if infected {
            FUN_0043F780_DAMAGE_PACKET
        } else {
            PRIMARY_PROJECTILE_DAMAGE_PACKET
        };
        let expected_damage = packet.filtered_raw(Some(&profile));
        assert_eq!(expected_damage, if infected { 0 } else { 1800 });
        let result = world.hit(infected);
        let Intro2Type58ImpactOutcome::Applied(applied) = result else {
            panic!("{result:?}")
        };
        assert_eq!(applied.filtered_damage_raw, expected_damage);
        assert!(applied.death_publication.is_none());
        assert_eq!(
            world.actor().collision.health_raw,
            RetailRuntimeValue::Known(7000 - expected_damage)
        );
        assert_eq!(
            world.actor().collision.last_hit_presentation_tick_at_0x34,
            if infected {
                stamp
            } else {
                RetailRuntimeValue::Known(tick)
            }
        );
        if infected {
            assert_eq!(
                world.actor().collision.state_flags_at_0x08.masked(0x2000),
                RetailRuntimeValue::Known(0x2000)
            );
            assert!(
                world.fx.take_positional_sounds().is_empty(),
                "Type58's infection cue is exact null"
            );
        }
        assert_ne!(
            world
                .actor()
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary),
            primary,
            "both current source hit slots invoke C690 before checked damage"
        );
        assert_eq!(world.scheduler.adopt_intro2_type58(&world.entities), 0);
        world.step(false);
        let mut killed = false;
        for _ in 0..4 {
            let mut expected_feedback = world.notifications.clone();
            let hit_tick = world.tick;
            let result = world.hit(false);
            let Intro2Type58ImpactOutcome::Applied(applied) = result else {
                panic!("{result:?}")
            };
            assert_eq!(applied.filtered_damage_raw, 1800);
            if applied.death_publication.is_some() {
                expected_feedback.queue_player_kill(hit_tick as i32);
                assert_eq!(world.notifications, expected_feedback);
                killed = true;
                break;
            }
            assert_eq!(world.notifications, expected_feedback);
        }
        assert!(killed, "real primary shots must exhaust authored health");
        world.assert_class12();
        world.step(true);
        let feedback = world.notifications.clone();
        assert!(matches!(
            world.hit(false),
            Intro2Type58ImpactOutcome::Applied(_)
        ));
        assert_eq!(
            world.notifications, feedback,
            "Class12 does not refresh the player kill hint"
        );
    }
}

#[v2k_test_support::retail_test]
fn ordinary_type58_same_origin_radial_zero_nonlethal_and_lethal_keep_live_custody() {
    for amount in [1999, 2100, 10000] {
        let mut world = World::new();
        let position = world.actor().position_raw();
        let velocity = world.actor().velocity_raw();
        let tasks = format!("{:?}", world.actor().actor_tasks);
        let context = world.actor().current_behavior_context;
        let stamp = world.actor().collision.last_hit_presentation_tick_at_0x34;
        let RetailRuntimeValue::Known(profile) = world.actor().collision.damage_profile else {
            panic!()
        };
        let filtered = DamagePacket::collision(amount).filtered_raw(Some(&profile));
        assert_eq!(
            filtered,
            match amount {
                1999 => 0,
                2100 => 100,
                _ => 8000,
            }
        );
        let result = world.radial(amount);
        assert!(result.blocked.is_none(), "{result:?}");
        assert_eq!(result.completed_target_ids, [world.id]);
        assert_eq!(result.accepted_targets, 1);
        assert_eq!(world.actor().position_raw(), position);
        assert_eq!(
            world.actor().collision.last_hit_presentation_tick_at_0x34,
            stamp,
            "radial uses generic checked damage, without primary's stamp/reselection"
        );
        if filtered < 7000 {
            assert_eq!(
                world.actor().velocity_raw(),
                velocity,
                "same-origin impulse is zero"
            );
            assert_eq!(
                world.actor().collision.health_raw,
                RetailRuntimeValue::Known(7000 - filtered)
            );
            assert_eq!(world.actor().current_behavior_context, context);
            assert_eq!(format!("{:?}", world.actor().actor_tasks), tasks);
            world.step(false);
        } else {
            assert_eq!(
                world.actor().velocity_raw(),
                [velocity[0], 500, velocity[2]],
                "04120 writes raw Y=500 after shared setup, preserving radial X/Z"
            );
            world.assert_class12();
            world.step(true);
        }
    }
}

#[v2k_test_support::retail_test]
fn parked_ordinary_type58_living_and_dying_block_particle_and_radial_prefixes() {
    for dying in [false, true] {
        let mut world = World::new();
        if dying {
            let result = world.radial(10000);
            assert!(result.blocked.is_none(), "{result:?}");
            world.assert_class12();
        }
        world.clear_effects();
        assert!(world
            .scheduler
            .park_native_contact_prefix(&world.entities, world.id));
        let snapshot = world.snapshot();
        if !dying {
            let owner =
                crate::intro2_type58::Intro2Type58Owner::adopt(&world.entities, world.id).unwrap();
            world.scheduler.register_intro2_type58(owner);
            assert_eq!(
                world.snapshot(),
                snapshot,
                "a new observation cannot release a committed contact prefix"
            );
        }
        let rng = world
            .fx
            .fork_for_main_base_abort_transaction()
            .next_shared_retail_random_u16();
        let particles = world.fx.particle_count();
        for _ in 0..4 {
            for infected in [false, true] {
                assert!(matches!(
                    world.hit(infected),
                    Intro2Type58ImpactOutcome::Blocked {
                        committed_prefix: false,
                        ..
                    }
                ));
                assert_eq!(world.snapshot(), snapshot);
            }
            for amount in [1999, 2100, 10000] {
                let result = world.radial(amount);
                let Some(PlayingRadialBlock::Native(block)) = result.blocked else {
                    panic!("{result:?}")
                };
                assert_eq!(block.target_id, world.id);
                assert_eq!(block.phase, DynamicRadialLivePhase::MutationCustody);
                assert_eq!(
                    block.reason,
                    DynamicRadialLiveBlockReason::NativeActorMutationCustody
                );
                assert!(!block.target_prefix_committed);
                assert!(result.completed_target_ids.is_empty());
                assert_eq!(world.snapshot(), snapshot);
            }
            assert_eq!(
                world
                    .fx
                    .fork_for_main_base_abort_transaction()
                    .next_shared_retail_random_u16(),
                rng
            );
            assert_eq!(world.fx.particle_count(), particles);
            assert_eq!(world.fx.pending_event_count(), 0);
            assert!(world.fx.take_positional_sounds().is_empty());
            assert!(!world
                .scheduler
                .prepare_native_actor_mutation(&world.entities, world.id));
        }
    }
}

#[v2k_test_support::retail_test]
fn foreign_type58_receipt_blocks_particle_and_radial_before_any_prefix() {
    let mut world = World::new();
    let foreign = World::new();
    world
        .entities
        .entity_mut(world.id)
        .unwrap()
        .intro2_type58_runtime = foreign.actor().intro2_type58_runtime;
    let snapshot = world.snapshot();
    let rng = world
        .fx
        .fork_for_main_base_abort_transaction()
        .next_shared_retail_random_u16();
    for infected in [false, true] {
        assert!(matches!(
            world.hit(infected),
            Intro2Type58ImpactOutcome::Blocked {
                committed_prefix: false,
                ..
            }
        ));
        assert_eq!(world.snapshot(), snapshot);
    }
    let result = world.radial(2100);
    assert!(
        matches!(result.blocked, Some(PlayingRadialBlock::Native(block))
        if block.phase == DynamicRadialLivePhase::MutationCustody
            && !block.target_prefix_committed)
    );
    assert_eq!(world.snapshot(), snapshot);
    assert_eq!(world.fx.next_shared_retail_random_u16(), rng);
    assert!(!world
        .scheduler
        .prepare_native_actor_mutation(&world.entities, world.id));
}
