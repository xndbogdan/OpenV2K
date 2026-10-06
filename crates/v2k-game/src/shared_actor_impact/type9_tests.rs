//! Production particle routing against real ordinary-world peasant allocations.

use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{ActorTaskId, ActorTaskSlot},
    attract_attention::{
        AttractAttentionResourceTextRequest, ATTRACT_ATTENTION_INITIAL_STYLE_ADDRESS,
        ATTRACT_ATTENTION_RESOURCE_TEXT_EVENT, ATTRACT_ATTENTION_RESOURCE_TEXT_GLOBAL_ID,
        ATTRACT_ATTENTION_TARGET_STYLE_ADDRESS,
    },
    damage::{DamagePacket, FUN_0043F780_DAMAGE_PACKET},
    entity::{
        AuthoredPlayerArrival, AuthoredWorldConstruction, BeamCommand, BeamOutcome,
        CargoDropContext, Entity, EntityConstructionResources, LateTailMaterialiserFrame,
        PlayerCargoFrame,
    },
    entity_collision_state::{
        EntityTypeRuntimeMetadata, RetailRuntimeValue, CHECKED_DAMAGE_ENABLED_STATE_BIT,
        DYING_STATE_BIT,
    },
    entity_view_detail::RetailViewDetailContext,
    gameplay_notifications::GameplayNotifications,
    impact_reaction::{
        apply_impact_reaction, ImpactReactionBody, IMPACT_REACTION_ENABLED_STATE_BIT,
        IMPACT_REACTION_NETWORKED_STATE_BIT, IMPACT_REACTION_SUPPRESSED_STATE_BIT,
    },
    live_actor_checked_damage::LiveActorDamageOutcome,
    main_base_type9_abort::MainBaseType9ExplodingTaskLease,
    ordinary_type9_impact::NativeType9ImpactOutcome,
    session::GameSession,
    specialized_actor_task_production::{
        SpecializedActorTaskFamily, SpecializedActorTaskProductionFrame,
        SpecializedActorTaskProductionOutcome,
    },
    static_damage::StaticDamageScheduler,
    world_fx::BallisticDamageRequest,
};

#[derive(Clone, Copy)]
enum PlayerArrival {
    DirectLoad,
    NearFirstPeasant,
}

pub(crate) struct World {
    pub(crate) session: GameSession,
    pub(crate) entities: EntityManager,
    pub(crate) scheduler: SpecializedActorTaskScheduler,
    pub(crate) fx: WorldFx,
    pub(crate) notifications: GameplayNotifications,
    pub(crate) tick: u32,
}

impl World {
    fn new(level: u32, arrival: PlayerArrival, rng_prehistory: usize) -> Self {
        let data = v2k_test_support::retail_dir();
        assert!(
            data.join("PRELOAD.DAT").is_file(),
            "normal-tier retail corpus required"
        );
        let mut session = GameSession::init(&data).unwrap();
        session.load_auxiliary_ovl(3, 1).unwrap();
        session.load_level_by_id(level, 1).unwrap();
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
        let position_raw = match arrival {
            PlayerArrival::DirectLoad => [19_712, -500, 14_848],
            PlayerArrival::NearFirstPeasant => {
                let spawn = session
                    .cache
                    .level_desc()
                    .unwrap()
                    .entities
                    .iter()
                    .find(|spawn| spawn.entity_type == 9)
                    .unwrap();
                let raw = spawn.position_raw();
                [
                    raw[0].wrapping_add(64),
                    session
                        .cache
                        .terrain()
                        .unwrap()
                        .bilinear_height_raw(raw[0], raw[2])
                        .wrapping_sub(256),
                    raw[2],
                ]
            }
        };
        let mut fx = WorldFx::new();
        for _ in 0..rng_prehistory {
            fx.next_shared_retail_random_u16();
        }
        let mut entities = EntityManager::from_authored_world(
            AuthoredWorldConstruction {
                logical_world_index: (level - 12) as i32,
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
                    position_raw,
                    heading_raw: 0x4000,
                }),
                retail_tick: 0,
            },
            &mut fx,
        )
        .unwrap();
        let mut scheduler = SpecializedActorTaskScheduler::new();
        let count = entities
            .iter_all()
            .filter(|entity| entity.entity_type == 9)
            .count();
        assert_eq!(
            scheduler
                .adopt_fresh_level1_type9_selected(&mut entities)
                .unwrap(),
            count
        );
        assert!(entities.player().is_some());
        let mut world = Self {
            session,
            entities,
            scheduler,
            fx,
            notifications: GameplayNotifications::new(),
            tick: 1,
        };
        world.clear_effects();
        world
    }

    fn actor(&self, id: u32) -> &Entity {
        self.entities
            .iter_all()
            .find(|entity| entity.id == id)
            .unwrap()
    }

    fn ids(&self) -> Vec<u32> {
        self.entities
            .iter_all()
            .filter(|entity| entity.entity_type == 9)
            .map(|entity| entity.id)
            .collect()
    }

    fn hit(&mut self, impact: ParticleEntityImpact) -> NativeType9ImpactOutcome {
        let routed = apply_shared_actor_particle_hit(
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
        let Some(SharedActorImpactOutcome::Peasant(outcome)) = routed else {
            panic!("ordinary peasant must use the production native router: {routed:?}");
        };
        outcome
    }

    fn clear_effects(&mut self) {
        self.fx.process_pending();
        self.fx.take_positional_sounds();
        self.entities
            .take_pending_fresh_level1_type9_resource_text_receipts();
    }

    pub(crate) fn step(&mut self, elapsed_micros: u32) {
        let player = self.entities.player().unwrap().position;
        self.scheduler.publish_presented_view_detail(
            &mut self.entities,
            RetailViewDetailContext::from_world(
                [player[0], player[1] + 8., player[2] - 8.],
                0.7,
                (52, 30),
            ),
        );
        let claims: Vec<_> = self.scheduler.actor_animation_claims().collect();
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
                elapsed_micros,
                global_elapsed_micros: elapsed_micros,
                retail_tick: self.tick,
                main_base_abort_active: false,
            },
            &mut self.notifications,
        );
        assert!(pass.block.is_none(), "{:?}", pass.block);
        for outcome in &pass.outcomes {
            assert_completed(outcome);
        }
        self.entities
            .advance_unclaimed_actor_animations(elapsed_micros, &claims);
        let blocked = self
            .entities
            .update_late_tail_materialisers(LateTailMaterialiserFrame {
                elapsed_micros,
                terrain: self.session.cache.terrain().unwrap(),
                world_fx: &mut self.fx,
                scheduler: &mut self.scheduler,
                notifications: &mut self.notifications,
                retail_tick: self.tick,
            });
        assert!(blocked.is_empty(), "materialiser: {blocked:?}");
        self.tick += 1;
    }

    fn cargo_step(&mut self) -> Option<BeamOutcome> {
        let cargo = self.entities.update_player_cargo(PlayerCargoFrame {
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
        assert!(cargo.blocked.is_empty(), "{:?}", cargo.blocked);
        self.step(20_000);
        cargo.beam
    }
}

fn assert_completed(outcome: &SpecializedActorTaskProductionOutcome) {
    use crate::{
        intro2_type9_class14::Intro2Type9Class14Outcome as NativeClass14,
        main_base_type9_production::MainBaseType9ExplodingProductionOutcome as Exploding,
        ordinary_type9_attract_attention_production::OrdinaryType9AttractAttentionProductionOutcome as Attract,
        ordinary_type9_carried_production::Type9CarriedProductionOutcome as Carried,
        ordinary_type9_go_to_job_production::OrdinaryType9GoToJobProductionOutcome as Job,
        ordinary_type9_run_away_production::OrdinaryType9RunAwayProductionOutcome as RunAway,
        ordinary_type9_wander_production::OrdinaryType9WanderProductionOutcome as Wander,
    };
    use SpecializedActorTaskProductionOutcome as Outcome;
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
        Outcome::Intro2Type9Class14(value) => matches!(
            value,
            NativeClass14::Blocked { .. }
                | NativeClass14::Pending { .. }
                | NativeClass14::Dropped { .. }
        ),
        Outcome::OrdinaryType9Carried(value) => {
            matches!(value, Carried::Blocked { .. } | Carried::Dropped { .. })
        }
        _ => false,
    };
    assert!(!blocked, "{outcome:?}");
}

fn primary(id: u32, amount: i32) -> ParticleEntityImpact {
    ParticleEntityImpact {
        source_particle_class: 16,
        impact_position_argument_va: 0,
        target_entity_id: id,
        position_world: [0.; 3],
        velocity_raw: [700, -200, 900],
        damage: Some(BallisticDamageRequest {
            packet: DamagePacket {
                channels: [2, 0],
                amounts_raw: [amount, 0],
            },
            source_entity_type_at_birth: Some(46),
            // The source owner is retained packet provenance, not a target
            // allocation claim. Player handle1 is the real ordinary birth.
            source_owner_id: Some(1),
        }),
    }
}

fn infected(id: u32) -> ParticleEntityImpact {
    let mut impact = primary(id, 0);
    impact.source_particle_class = 5;
    impact.impact_position_argument_va = 0x004d_cf48;
    impact.damage.as_mut().unwrap().packet = FUN_0043F780_DAMAGE_PACKET;
    impact
}

#[v2k_test_support::retail_test]
fn static_route_bat_and_sibling_packets_use_native_peasant_death_across_worlds() {
    for level in [13, 14, 15] {
        for (class, packet, expected) in [
            (52, crate::damage::TYPE_47_PROJECTILE_DAMAGE_PACKET, 2600),
            (68, crate::damage::CLASS68_STATIC_ROUTE_DAMAGE_PACKET, 5600),
            (85, crate::damage::DRAGON_FIREBALL_DAMAGE_PACKET, 12100),
        ] {
            let mut world = World::new(level, PlayerArrival::DirectLoad, 17);
            let id = world.ids()[0];
            let mut impact = primary(id, 0);
            impact.source_particle_class = class;
            impact.damage.as_mut().unwrap().packet = packet;
            world.clear_effects();
            let result = applied(world.hit(impact));
            assert_eq!(
                result.filtered_damage_raw, expected,
                "world{level} class{class}"
            );
            assert!(result.death_publication.is_some());
            assert_eq!(
                world.actor(id).collision.health_raw,
                RetailRuntimeValue::Known(0)
            );
            assert_eq!(
                world.scheduler.family_for(id),
                Some(SpecializedActorTaskFamily::Intro2Type9Class14)
            );
            world.fx.process_pending();
            let sounds = world.fx.take_positional_sounds();
            let hit = sounds
                .iter()
                .position(|sound| sound.sound_id == 95)
                .expect("11180 pre-hit cue");
            let death = sounds
                .iter()
                .position(|sound| sound.sound_id == 35)
                .expect("native peasant death cue");
            assert!(hit < death);
            world.step(20_000);
        }
    }
}

fn applied(
    outcome: NativeType9ImpactOutcome,
) -> LiveActorDamageOutcome<MainBaseType9ExplodingTaskLease> {
    let NativeType9ImpactOutcome::Applied(value) = outcome else {
        panic!("{outcome:?}");
    };
    value
}

fn style(entity: &Entity) -> u32 {
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        panic!("current style");
    };
    context.active_style().style_address()
}

fn tasks(entity: &Entity) -> [Option<ActorTaskId>; 3] {
    ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_tasks.task_in_slot(slot))
}

fn expects_c690(style: u32, infected: bool) -> bool {
    match style {
        0x004c_79c0 | 0x004c_8788 | ATTRACT_ATTENTION_TARGET_STYLE_ADDRESS => true,
        0x004c_7618 | 0x004c_7660 => infected,
        0x004c_7a08
        | 0x004c_87d0
        | 0x004c_70c0
        | 0x004c_7108
        | ATTRACT_ATTENTION_INITIAL_STYLE_ADDRESS => false,
        _ => panic!("uncovered authored style{style:08x}"),
    }
}

fn reaction_body(entity: &Entity) -> ImpactReactionBody {
    let mask = IMPACT_REACTION_ENABLED_STATE_BIT
        | IMPACT_REACTION_SUPPRESSED_STATE_BIT
        | IMPACT_REACTION_NETWORKED_STATE_BIT;
    let RetailRuntimeValue::Known(flags) = entity.collision.state_flags_at_0x08.masked(mask) else {
        panic!("reaction state");
    };
    ImpactReactionBody {
        state_flags_at_0x08: flags,
        mass_raw_at_0xb0: entity.mass_raw,
        linear_velocity_xyz_raw: entity.velocity_raw(),
        angular_heading_pitch_roll_raw: entity.rotation_heading_pitch_roll_raw(),
    }
}

pub(crate) fn initial_attract() -> (World, u32) {
    // Advance the real process stream before construction, without injecting
    // a selected style, task, captured peasant identity, or RNG seed.
    for prehistory in 0..64 {
        let world = World::new(14, PlayerArrival::NearFirstPeasant, prehistory);
        let id = world
            .entities
            .iter_all()
            .find(|entity| {
                entity.entity_type == 9
                    && style(entity) == ATTRACT_ATTENTION_INITIAL_STYLE_ADDRESS
                    && matches!(
                        entity.actor_task_state(ActorTaskSlot::Secondary),
                        Some(ActorTaskRuntime::AttractAttentionCandidate(_))
                    )
            })
            .map(|entity| entity.id);
        if let Some(id) = id {
            return (world, id);
        }
    }
    panic!("nearby real player must reach the odd class45 constructor branch");
}

pub(crate) fn acquired_attract() -> (World, u32) {
    let (mut world, id) = initial_attract();
    world.entities.entity_mut(id).unwrap().collision.health_raw =
        RetailRuntimeValue::Known(1_000_000);
    for _ in 0..500 {
        let position = world.actor(id).position;
        world.entities.player_mut().unwrap().position =
            [position[0] + 0.25, position[1] - 1., position[2]];
        world.step(20_000);
        if style(world.actor(id)) == ATTRACT_ATTENTION_TARGET_STYLE_ADDRESS {
            assert!(matches!(
                world.actor(id).actor_task_state(ActorTaskSlot::Primary),
                Some(ActorTaskRuntime::AttractAttentionTargetRoute(_))
            ));
            world.clear_effects();
            return (world, id);
        }
    }
    panic!("actual class45 Candidate must acquire the real nearby player");
}

#[v2k_test_support::retail_test]
fn all_later_authored_peasants_use_particle_style_and_damage_owners() {
    for (level, expected_count) in [(14, 8), (15, 8), (25, 14), (39, 6)] {
        let mut world = World::new(level, PlayerArrival::DirectLoad, 0);
        let ids = world.ids();
        assert_eq!(ids.len(), expected_count);
        for id in ids {
            world.entities.entity_mut(id).unwrap().collision.health_raw =
                RetailRuntimeValue::Known(1_000_000);
            for infection in [false, true, false] {
                let actor = world.actor(id);
                assert!(
                    actor.ordinary_type9_native_receipt.is_some()
                        && actor.intro2_type9_runtime.is_none()
                );
                let old_receipt = actor.ordinary_type9_native_receipt;
                let old_tasks = tasks(actor);
                let should_reselect = expects_c690(style(actor), infection);
                let old_stamp = actor.collision.last_hit_presentation_tick_at_0x34;
                let old_position = actor.position_raw();
                let old_scheduler = actor.collision.callback_scheduler_accumulator_us_at_0x6c;
                let old_sub_d = actor
                    .ordinary_type9_selected_component_runtime
                    .unwrap()
                    .components()
                    .sub_d_frame_owner;
                let RetailRuntimeValue::Known(health) = actor.collision.health_raw else {
                    panic!();
                };
                let RetailRuntimeValue::Known(profile) = actor.collision.damage_profile else {
                    panic!();
                };
                let packet = if infection {
                    infected(id)
                } else {
                    primary(id, 100)
                };
                let expected_damage = profile.filter(packet.damage.unwrap().packet);
                let tick = world.tick;
                let result = applied(world.hit(packet));
                assert_eq!(result.filtered_damage_raw, expected_damage);
                assert!(result.death_publication.is_none());
                let actor = world.actor(id);
                assert_eq!(
                    actor.collision.health_raw,
                    RetailRuntimeValue::Known(health - expected_damage)
                );
                assert_eq!(
                    tasks(actor) != old_tasks,
                    should_reselect,
                    "level{level} actor{id}"
                );
                assert_eq!(
                    actor.collision.last_hit_presentation_tick_at_0x34,
                    if infection {
                        old_stamp
                    } else {
                        RetailRuntimeValue::Known(tick)
                    }
                );
                assert_eq!(actor.ordinary_type9_native_receipt, old_receipt);
                assert_eq!(actor.position_raw(), old_position);
                assert_eq!(
                    actor.collision.callback_scheduler_accumulator_us_at_0x6c,
                    old_scheduler
                );
                assert_eq!(
                    actor
                        .ordinary_type9_selected_component_runtime
                        .unwrap()
                        .components()
                        .sub_d_frame_owner,
                    old_sub_d,
                    "particle dispatch must not run a mover"
                );
                if infection {
                    assert_eq!(
                        actor.collision.active_model_slot(),
                        RetailRuntimeValue::Known(2)
                    );
                }
            }
        }
    }
}

#[v2k_test_support::retail_test]
fn initial_class45_null_hooks_preserve_candidate_cue_and_reaction_order() {
    for infection in [false, true] {
        let (mut world, id) = initial_attract();
        world.entities.entity_mut(id).unwrap().collision.health_raw =
            RetailRuntimeValue::Known(1_000_000);
        let actor = world.actor(id);
        let before_tasks = tasks(actor);
        let before_states =
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| actor.actor_task_state(slot).cloned());
        let before_animation = actor.actor_animation_runtime;
        let mut expected_body = reaction_body(actor);
        let packet = if infection {
            infected(id)
        } else {
            primary(id, 100)
        };
        let mut expected_fx = world.fx.fork_for_main_base_abort_transaction();
        apply_impact_reaction(
            &mut expected_body,
            id,
            packet
                .damage_delivery_record()
                .unwrap()
                .packet
                .impact_sum_raw(),
            packet.velocity_raw,
            || u32::from(expected_fx.next_shared_retail_random_u16()),
        )
        .unwrap();
        applied(world.hit(packet));
        let actor = world.actor(id);
        assert_eq!(style(actor), ATTRACT_ATTENTION_INITIAL_STYLE_ADDRESS);
        assert_eq!(tasks(actor), before_tasks);
        assert_eq!(
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| actor.actor_task_state(slot).cloned()),
            before_states
        );
        assert_eq!(
            actor.actor_animation_runtime, before_animation,
            "null hooks cannot retire the attention cue or clear forced-stop"
        );
        assert_eq!(actor.velocity_raw(), expected_body.linear_velocity_xyz_raw);
        assert_eq!(
            actor.rotation_heading_pitch_roll_raw(),
            expected_body.angular_heading_pitch_roll_raw
        );
        assert_eq!(
            world.fx.next_shared_retail_random_u16(),
            expected_fx.next_shared_retail_random_u16()
        );
        assert!(world
            .entities
            .pending_fresh_level1_type9_resource_text_receipts()
            .is_empty());
        // The second delivery crosses the just-mutated initial owner again.
        applied(world.hit(primary(id, 0)));
        // A null style hook must preserve a runnable owner after the packet
        // reaction, not merely admit another hit in the same frame.
        world.step(20_000);
    }
}

#[v2k_test_support::retail_test]
fn acquired_class45_both_hit_hooks_retire_the_actual_route_graph() {
    for infection in [false, true] {
        let (mut world, id) = acquired_attract();
        let old_tasks = tasks(world.actor(id));
        let allocation = world.actor(id).ordinary_type9_native_receipt;
        applied(world.hit(if infection {
            infected(id)
        } else {
            primary(id, 100)
        }));
        let actor = world.actor(id);
        assert_ne!(tasks(actor), old_tasks);
        assert_eq!(actor.ordinary_type9_native_receipt, allocation);
        for task in old_tasks.into_iter().flatten() {
            assert!(actor.actor_tasks.wrapper_flags(task).is_none());
        }
        applied(world.hit(primary(id, 0)));
        world.step(20_000);
    }
}

#[v2k_test_support::retail_test]
fn hit_reselection_into_class45_queues_its_parity_graph_text_and_sound_once() {
    let (mut world, id) = acquired_attract();
    // Observe this isolated live packet with a fresh notification owner, after
    // the acquisition fixture has completed its unrelated actor callbacks.
    world.notifications = GameplayNotifications::new();
    let mut selected = false;
    for _ in 0..64 {
        world.clear_effects();
        let mut expected_fx = world.fx.fork_for_main_base_abort_transaction();
        let before_notifications = world.notifications.clone();
        let mut expected_notifications = before_notifications.clone();
        expected_notifications
            .queue_attract_attention_resource_text(
                AttractAttentionResourceTextRequest {
                    event: ATTRACT_ATTENTION_RESOURCE_TEXT_EVENT,
                    global_resource_id: ATTRACT_ATTENTION_RESOURCE_TEXT_GLOBAL_ID,
                },
                world.tick as i32,
            )
            .unwrap();
        let packet = infected(id);
        applied(world.hit(packet));
        if style(world.actor(id)) != ATTRACT_ATTENTION_INITIAL_STYLE_ADDRESS {
            continue;
        }
        selected = true;
        expected_fx.next_shared_retail_random_u16(); //425680 weighted choice.
        let parity = expected_fx.next_shared_retail_random_u16(); //BA40.
        for _ in 0..(2 + usize::from(parity & 1 != 0)) {
            expected_fx.next_shared_retail_random_u16();
        }
        let mut body = reaction_body(world.actor(id));
        apply_impact_reaction(&mut body, id, 0, packet.velocity_raw, || {
            u32::from(expected_fx.next_shared_retail_random_u16())
        })
        .unwrap();
        assert_eq!(
            world.fx.next_shared_retail_random_u16(),
            expected_fx.next_shared_retail_random_u16()
        );
        let actor = world.actor(id);
        assert_eq!(
            matches!(
                actor.actor_task_state(ActorTaskSlot::Secondary),
                Some(ActorTaskRuntime::AttractAttentionCandidate(_))
            ),
            parity & 1 != 0
        );
        assert!(actor.actor_task_state(ActorTaskSlot::Tertiary).is_some());
        assert_ne!(
            expected_notifications, before_notifications,
            "first event10 records the current hit tick"
        );
        assert_eq!(world.notifications, expected_notifications);
        assert!(
            world
                .entities
                .pending_fresh_level1_type9_resource_text_receipts()
                .is_empty(),
            "live hit must not defer a construction receipt"
        );
        world.fx.process_pending();
        assert_eq!(
            world
                .fx
                .take_positional_sounds()
                .iter()
                .filter(|sound| sound.sound_id == 72)
                .count(),
            1
        );
        applied(world.hit(primary(id, 0)));
        assert_eq!(
            world.notifications, expected_notifications,
            "null initial hook does not refresh event10"
        );
        break;
    }
    assert!(
        selected,
        "real nearby player must remain eligible during C690 reselection"
    );
}

#[v2k_test_support::retail_test]
fn ordinary_zero_impact_and_buffered_primary_keep_reaction_and_sound_gates() {
    // Type9 channel2 only contributes above its strict400 threshold.
    for (suppressed, amount, buffered) in
        [(false, 0, false), (true, 0, false), (false, 1_000, true)]
    {
        let (mut world, id) = initial_attract();
        let actor = world.entities.entity_mut(id).unwrap();
        actor.collision.state_flags_at_0x08.overwrite(
            IMPACT_REACTION_ENABLED_STATE_BIT | IMPACT_REACTION_SUPPRESSED_STATE_BIT,
            IMPACT_REACTION_ENABLED_STATE_BIT
                | if suppressed {
                    IMPACT_REACTION_SUPPRESSED_STATE_BIT
                } else {
                    0
                },
        );
        if buffered {
            actor.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(1_000_000);
        }
        let health = actor.collision.health_raw;
        let before_tasks = tasks(actor);
        let mut expected_body = reaction_body(actor);
        let packet = primary(id, amount);
        let mut expected_fx = world.fx.fork_for_main_base_abort_transaction();
        apply_impact_reaction(&mut expected_body, id, amount, packet.velocity_raw, || {
            u32::from(expected_fx.next_shared_retail_random_u16())
        })
        .unwrap();
        let result = applied(world.hit(packet));
        let actor = world.actor(id);
        assert_eq!(actor.collision.health_raw, health);
        assert_eq!(tasks(actor), before_tasks);
        assert_eq!(actor.velocity_raw(), expected_body.linear_velocity_xyz_raw);
        assert_eq!(
            actor.rotation_heading_pitch_roll_raw(),
            expected_body.angular_heading_pitch_roll_raw
        );
        assert_eq!(result.damage_after_buffer_raw, 0);
        assert_eq!(result.filtered_damage_raw != 0, buffered);
        assert_eq!(
            world.fx.next_shared_retail_random_u16(),
            expected_fx.next_shared_retail_random_u16()
        );
        world.fx.process_pending();
        assert_eq!(
            world
                .fx
                .take_positional_sounds()
                .iter()
                .filter(|sound| sound.sound_id == 95)
                .count(),
            usize::from(buffered)
        );
    }
}

#[v2k_test_support::retail_test]
fn later_world_particle_death_adopts_class14_once_and_retires_the_allocation() {
    for level in [14, 15, 25, 39] {
        let mut world = World::new(level, PlayerArrival::DirectLoad, 0);
        let id = world.ids()[0];
        let result = applied(world.hit(infected(id)));
        assert!(result.death_publication.is_some());
        // Packet death uses the complete native12DA0/DCA0/E870 owner. The
        // older OrdinaryType9Class14 adapter has a different callback scope.
        assert_eq!(
            world.scheduler.family_for(id),
            Some(SpecializedActorTaskFamily::Intro2Type9Class14)
        );
        assert_eq!(
            world.actor(id).collision.health_raw,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            world
                .actor(id)
                .collision
                .state_flags_at_0x08
                .masked(DYING_STATE_BIT),
            RetailRuntimeValue::Known(DYING_STATE_BIT)
        );
        let corpse_tasks = tasks(world.actor(id));
        let repeat = applied(world.hit(infected(id)));
        assert!(repeat.death_publication.is_none());
        assert_eq!(tasks(world.actor(id)), corpse_tasks);
        world.fx.process_pending();
        assert_eq!(
            world
                .fx
                .take_positional_sounds()
                .iter()
                .filter(|sound| sound.sound_id == 35)
                .count(),
            1
        );
        let mut removed = false;
        for _ in 0..60 {
            world.step(100_000);
            removed |= world
                .entities
                .cleanup_pending_actor_deferred_destroys()
                .contains(&id);
            if removed {
                break;
            }
        }
        assert!(
            removed,
            "level{level} native packet death must complete class14"
        );
    }
}

#[v2k_test_support::retail_test]
fn real_beam_carried_peasants_keep_ineligible_health_and_the_none_relation() {
    for level in [14, 39] {
        let world = World::new(level, PlayerArrival::DirectLoad, 0);
        let id = world.ids()[0];
        assert_beam_carried_packet_entries(world, id);
    }
    // Exercise both reachable class45 variants and class10 explicitly. The
    // earlier first-peasant cases alone do not establish those carry hooks.
    let (world, id) = initial_attract();
    assert_eq!(
        style(world.actor(id)),
        ATTRACT_ATTENTION_INITIAL_STYLE_ADDRESS
    );
    assert_beam_carried_packet_entries(world, id);
    let (world, id) = acquired_attract();
    assert_eq!(
        style(world.actor(id)),
        ATTRACT_ATTENTION_TARGET_STYLE_ADDRESS
    );
    assert_beam_carried_packet_entries(world, id);
    let (world, id) = authored_run_away();
    assert!(matches!(style(world.actor(id)), 0x004c_7618 | 0x004c_7660));
    assert_beam_carried_packet_entries(world, id);
}

fn authored_run_away() -> (World, u32) {
    use crate::ordinary_type9_initial_selection::LEVEL_ONE_TYPE9_BADDIE_NEARBY_CAPABILITY_MASK;

    // Authored load positions need not put an enemy within C690's strict
    // 0xF00-axis range. Bring an actual living authored baddie near an actual
    // peasant, then let production particle dispatch select and construct
    // class10. No selected task, style, capability or state is injected.
    for level in [25, 39, 14, 15] {
        let mut world = World::new(level, PlayerArrival::DirectLoad, 0);
        let baddie = world.entities.iter_all().find(|entity| {
            let state = entity.collision.state_flags_at_0x08;
            entity.capability_flags & LEVEL_ONE_TYPE9_BADDIE_NEARBY_CAPABILITY_MASK != 0
                && state.known_value_bits() != 0
                && state.masked(0x5000) == RetailRuntimeValue::Known(0)
        });
        let Some(baddie_id) = baddie.map(|entity| entity.id) else {
            continue;
        };
        let Some(id) = world
            .ids()
            .into_iter()
            .find(|&id| matches!(style(world.actor(id)), 0x004c_79c0 | 0x004c_8788))
        else {
            continue;
        };
        let position = world.actor(id).position;
        world.entities.entity_mut(baddie_id).unwrap().position =
            [position[0] + 1., position[1], position[2] + 1.];
        // Keep the player outside the wrapped axis cube so repeated hits
        // cannot select the initial class45 variant whose hit hook is null.
        world.entities.player_mut().unwrap().position =
            [position[0], position[1] + 128., position[2]];
        world.entities.entity_mut(id).unwrap().collision.health_raw =
            RetailRuntimeValue::Known(1_000_000);
        for _ in 0..256 {
            let old_tasks = tasks(world.actor(id));
            let result = applied(world.hit(infected(id)));
            assert!(result.death_publication.is_none());
            assert_ne!(tasks(world.actor(id)), old_tasks);
            if matches!(style(world.actor(id)), 0x004c_7618 | 0x004c_7660) {
                assert_eq!(
                    world.scheduler.family_for(id),
                    Some(SpecializedActorTaskFamily::OrdinaryType9RunAway)
                );
                world.clear_effects();
                return (world, id);
            }
        }
        panic!("level{level} live baddie{id} selection must reach native class10");
    }
    panic!("ordinary corpus must contain a living authored baddie and selectable peasant");
}

fn assert_beam_carried_packet_entries(mut world: World, id: u32) {
    let expected_carrying_style = match style(world.actor(id)) {
        0x004c_79c0 => 0x004c_7a08,
        0x004c_8788 => 0x004c_87d0,
        0x004c_7618 | 0x004c_7660 => 0x004c_76a8,
        ATTRACT_ATTENTION_INITIAL_STYLE_ADDRESS | ATTRACT_ATTENTION_TARGET_STYLE_ADDRESS => {
            0x004c_8740
        }
        source => panic!("unexpected pre-beam style{source:08x}"),
    };
    world.entities.queue_beam(BeamCommand::Collect);
    let mut collected = false;
    for _ in 0..30 {
        // Follow the real selected actor during the beam delay, preserving
        // its authored body and complete task graph until collection.
        let position = world.actor(id).position;
        world.entities.player_mut().unwrap().position =
            [position[0] + 0.25, position[1] - 1., position[2]];
        if world.cargo_step() == Some(BeamOutcome::Collected { entity_id: id }) {
            collected = true;
            break;
        }
    }
    assert!(collected, "actor{id} must use the real beam callback");
    world.clear_effects();
    let player_id = world.entities.player().unwrap().id;
    let actor = world.actor(id);
    let old_tasks = tasks(actor);
    let health = actor.collision.health_raw;
    assert_eq!(style(actor), expected_carrying_style);
    assert_eq!(actor.attached_to, Some(player_id));
    assert!(matches!(
        actor.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::None)
    ));
    assert_eq!(
        actor
            .collision
            .state_flags_at_0x08
            .masked(CHECKED_DAMAGE_ENABLED_STATE_BIT),
        RetailRuntimeValue::Known(0)
    );
    for packet in [primary(id, 20_000), infected(id)] {
        let result = applied(world.hit(packet));
        assert_eq!(result.filtered_damage_raw, 0);
        assert!(result.death_publication.is_none());
        let actor = world.actor(id);
        assert_eq!(actor.collision.health_raw, health);
        assert_eq!(actor.attached_to, Some(player_id));
        assert_eq!(style(actor), expected_carrying_style);
        assert_eq!(tasks(actor), old_tasks);
        let RetailRuntimeValue::Known(Some(sub_j)) =
            &world.entities.player().unwrap().sub_j_attachment_runtime
        else {
            panic!("real Sub-J");
        };
        assert!(sub_j.ordered_entity_ids().contains(&id));
    }
    world.step(20_000);
    assert_eq!(
        world.scheduler.family_for(id),
        Some(SpecializedActorTaskFamily::OrdinaryType9Carried)
    );
    world.fx.process_pending();
    assert!(!world
        .fx
        .take_positional_sounds()
        .iter()
        .any(|sound| sound.sound_id == 35 || sound.sound_id == 95));
}

#[v2k_test_support::retail_test]
fn missing_scheduler_and_foreign_native_receipt_reject_before_particle_prefix() {
    for foreign_receipt in [false, true] {
        let mut world = World::new(14, PlayerArrival::DirectLoad, 0);
        let id = world.ids()[0];
        if foreign_receipt {
            let other = World::new(14, PlayerArrival::DirectLoad, 0);
            assert_eq!(other.ids()[0], id);
            world
                .entities
                .entity_mut(id)
                .unwrap()
                .ordinary_type9_native_receipt = other.actor(id).ordinary_type9_native_receipt;
        } else {
            world.scheduler.clear_after_manager_reset();
        }
        let actor = world.actor(id);
        let collision = actor.collision.clone();
        let body = (
            actor.position_raw(),
            actor.velocity_raw(),
            actor.rotation_heading_pitch_roll_raw(),
        );
        let task_ids = tasks(actor);
        let mut expected_fx = world.fx.fork_for_main_base_abort_transaction();
        let result = world.hit(infected(id));
        assert!(
            matches!(
                result,
                NativeType9ImpactOutcome::Blocked {
                    committed_prefix: false,
                    ..
                }
            ),
            "{result:?}"
        );
        let actor = world.actor(id);
        assert_eq!(actor.collision, collision);
        assert_eq!(
            (
                actor.position_raw(),
                actor.velocity_raw(),
                actor.rotation_heading_pitch_roll_raw()
            ),
            body
        );
        assert_eq!(tasks(actor), task_ids);
        assert_eq!(world.fx.pending_event_count(), 0);
        assert_eq!(
            world.fx.next_shared_retail_random_u16(),
            expected_fx.next_shared_retail_random_u16()
        );
    }
}
