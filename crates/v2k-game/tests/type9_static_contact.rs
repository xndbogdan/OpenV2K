//! Real Level-1 fence geometry through the ordinary Type9 static owner.
//!
//! Peasants previously walked through static objects (notably into the
//! level-1 spider pen) because no Type9 static-contact pass existed. These
//! controls pin the pen-fence response against the same authored cells the
//! spider tests use.

use v2k_game::{
    actor_task_dispatcher::{ActorTaskRuntime, ActorTaskRuntimeFamily},
    actor_task_owner::ActorTaskSlot,
    entity::{
        AuthoredPlayerArrival, AuthoredWorldConstruction, Entity, EntityConstructionResources,
        EntityManager,
    },
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    gameplay_notifications::GameplayNotifications,
    ordinary_type9_static_contact::{
        resolve_ordinary_type9_late_contact, resolve_ordinary_type9_static_contact,
        OrdinaryType9PairSuffix, OrdinaryType9StaticContactApplied,
        OrdinaryType9StaticContactFrame, OrdinaryType9StaticContactOutcome,
    },
    session::GameSession,
    specialized_actor_task_production::{
        SpecializedActorTaskFamily, SpecializedActorTaskProductionFrame,
        SpecializedActorTaskScheduler,
    },
    static_damage::StaticDamageScheduler,
    world_fx::WorldFx,
};

#[derive(Clone, Copy)]
enum ConstructorCohort {
    Authored,
    Isolated,
    Player,
    Hostile,
    Base,
}

struct Fixture {
    session: GameSession,
    manager: EntityManager,
    scheduler: SpecializedActorTaskScheduler,
    metadata: EntityTypeRuntimeMetadata,
    static_damage: StaticDamageScheduler,
    notifications: GameplayNotifications,
    world_fx: WorldFx,
    peasant: u32,
}

impl Fixture {
    fn load() -> Self {
        Self::load_world(13)
    }

    fn load_world(world: u32) -> Self {
        Self::load_with_cohort(world, ConstructorCohort::Authored)
    }

    /// Actual Level1 records with a deliberately reduced/relocated candidate
    /// prefix select each native root. These are constructor controls, not
    /// authored scene trajectories or retail acceptance fixtures.
    fn load_with_cohort(world: u32, cohort: ConstructorCohort) -> Self {
        let dir = v2k_test_support::retail_dir();
        assert!(
            dir.join("PRELOAD.DAT").exists(),
            "normal-tier retail corpus required"
        );
        let mut session = GameSession::init(&dir).expect("PRELOAD");
        session
            .load_auxiliary_ovl(3, 1)
            .expect("normal system tier");
        session
            .load_level_by_id(world, 1)
            .expect("normal-tier world");
        let metadata: Vec<_> = session
            .cache
            .global_entity_model_table()
            .iter()
            .copied()
            .enumerate()
            .map(|(entity_type, model_slots)| {
                session
                    .cache
                    .global_entity_type(entity_type)
                    .map(EntityTypeRuntimeMetadata::from_section12)
                    .unwrap_or(EntityTypeRuntimeMetadata {
                        model_slots,
                        ..EntityTypeRuntimeMetadata::default()
                    })
            })
            .collect();
        let source = session.cache.level_desc().unwrap();
        let mut level = v2k_formats::levels::LevelDescriptor {
            raw_header: source.raw_header,
            name: source.name.clone(),
            world_style: source.world_style,
            terrain_sprite_base: source.terrain_sprite_base,
            sky_color_index: source.sky_color_index,
            sky_model: source.sky_model,
            main_base_abort_sky_color_index: source.main_base_abort_sky_color_index,
            main_base_abort_sky_model: source.main_base_abort_sky_model,
            terrain_draw_depth: source.terrain_draw_depth,
            sub_count: source.sub_count,
            campaign_record_count: source.campaign_record_count,
            entities: source.entities.clone(),
            campaign_records: source.campaign_records.clone(),
        };
        let mut arrival = [19_712, -500, 14_848];
        let required_class = match cohort {
            ConstructorCohort::Authored => None,
            _ => {
                assert_eq!(world, 13);
                let peasant = level
                    .entities
                    .iter()
                    .find(|spawn| spawn.entity_type == 9)
                    .unwrap()
                    .clone();
                let position = peasant.position_raw();
                let prefix_type = match cohort {
                    ConstructorCohort::Hostile => Some(17),
                    ConstructorCohort::Base => Some(6),
                    _ => None,
                };
                let prefix = prefix_type.map(|kind| {
                    let mut spawn = level
                        .entities
                        .iter()
                        .find(|spawn| spawn.entity_type == kind)
                        .unwrap()
                        .clone();
                    // This controlled list places the candidate first. Keep
                    // its index before the retained peasant index, as the
                    // native constructor authenticates authored-prefix order.
                    spawn.index = peasant.index - 1;
                    spawn.pos_data_1[..2].copy_from_slice(&position[0].to_le_bytes());
                    spawn.pos_data_1[2..].copy_from_slice(&position[1].to_le_bytes());
                    spawn.pos_data_2[..2].copy_from_slice(&position[2].to_le_bytes());
                    spawn
                });
                level.entities = prefix.into_iter().chain([peasant]).collect();
                level.sub_count = level.entities.len() as u32;
                // RunAway's authored acquisition filter84 accepts the
                // player's capability4, separately from the BaddieNearby
                // root's mask8. The spider selects the root; the live player
                // supplies an eligible target for the progressed task.
                arrival = if matches!(
                    cohort,
                    ConstructorCohort::Player | ConstructorCohort::Hostile
                ) {
                    position
                } else {
                    [0, -500, 0]
                };
                Some(match cohort {
                    ConstructorCohort::Isolated => 6,
                    ConstructorCohort::Player => 45,
                    ConstructorCohort::Hostile => 10,
                    ConstructorCohort::Base => 54,
                    ConstructorCohort::Authored => unreachable!(),
                })
            }
        };
        let mut fx = WorldFx::new();
        // Vary ordinary process history until the desired weighted branch is
        // selected. No recorded seed or spawn admission is substituted.
        let mut manager = (0..128)
            .find_map(|_| {
                let manager = EntityManager::from_authored_world(
                    AuthoredWorldConstruction {
                        logical_world_index: world as i32 - 12,
                        level: &level,
                        type_metadata: &metadata,
                        resources: EntityConstructionResources {
                            terrain: session.cache.terrain(),
                            terrain_objects: session.cache.terrain_objects(),
                            model_extent_raw: Some(&|id| {
                                session.cache.global_model(id).map(|model| model.radius)
                            }),
                        },
                        player_arrival: Some(AuthoredPlayerArrival {
                            position_raw: arrival,
                            heading_raw: 0x4000,
                        }),
                        retail_tick: 0,
                    },
                    &mut fx,
                )
                .expect("ordinary native publication");
                let selected = manager
                    .iter_all()
                    .find(|entity| entity.entity_type == 9)
                    .unwrap();
                let RetailRuntimeValue::Known(Some(context)) = selected.current_behavior_context
                else {
                    panic!("native selected context")
                };
                required_class
                    .is_none_or(|class| context.active_style().audited().unwrap().class_id == class)
                    .then_some(manager)
            })
            .expect("bounded process histories select every authored root");
        let mut scheduler = SpecializedActorTaskScheduler::new();
        let count = manager
            .iter_all()
            .filter(|entity| entity.entity_type == 9)
            .count();
        assert_eq!(
            scheduler
                .adopt_fresh_level1_type9_selected(&mut manager)
                .unwrap(),
            count
        );
        // First peasant with completed walking-task custody.
        let peasant = manager
            .iter_all()
            .filter(|entity| entity.entity_type == 9)
            .map(|entity| entity.id)
            .find(|id| scheduler.ordinary_type9_completed_walking_owner(&manager, *id))
            .expect("native walking owner");
        Self {
            session,
            manager,
            scheduler,
            metadata: metadata[9].clone(),
            static_damage: StaticDamageScheduler::new(),
            notifications: GameplayNotifications::new(),
            world_fx: fx,
            peasant,
        }
    }

    fn entity(&self) -> &Entity {
        self.manager
            .iter_all()
            .find(|entity| entity.id == self.peasant)
            .expect("retained peasant")
    }

    fn health_raw(&self) -> i32 {
        let RetailRuntimeValue::Known(health) = self.entity().collision.health_raw else {
            panic!("authenticated peasant health");
        };
        health
    }

    fn place(&mut self, position_raw: [i16; 3], velocity_raw: [i16; 3]) {
        self.manager
            .entity_mut(self.peasant)
            .expect("peasant motion owner")
            .set_motion_raw(position_raw, velocity_raw);
    }

    fn resolve(&mut self) -> OrdinaryType9StaticContactOutcome {
        resolve_ordinary_type9_static_contact(
            &mut self.manager,
            self.peasant,
            &self.metadata,
            OrdinaryType9StaticContactFrame {
                resources: &mut self.session.cache,
                static_damage: &mut self.static_damage,
                world_fx: &mut self.world_fx,
                scheduler: &mut self.scheduler,
                notifications: &mut self.notifications,
                retail_tick: 2,
            },
        )
    }

    fn step(&mut self, tick: u32) {
        let pass = self.scheduler.tick(
            &mut self.manager,
            SpecializedActorTaskProductionFrame {
                world: v2k_game::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                hive_components: None,
                resources: &mut self.session.cache,
                world_fx: &mut self.world_fx,
                static_damage: &mut self.static_damage,
                notification_phase:
                    v2k_game::gameplay_notifications::GameplayNotificationPhase::Playing,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: tick,
                main_base_abort_active: false,
            },
            &mut self.notifications,
        );
        assert!(pass.block.is_none(), "{pass:?}");
        assert!(
            !format!("{pass:?}").contains("Dropped") && !format!("{pass:?}").contains("Blocked"),
            "{pass:?}"
        );
    }

    fn contact(&mut self) -> OrdinaryType9StaticContactApplied {
        match self.resolve() {
            OrdinaryType9StaticContactOutcome::Applied(applied) => applied,
            outcome => panic!("expected authored fence contact, got {outcome:?}"),
        }
    }
}

fn static_private(
    task: ActorTaskRuntime,
) -> Option<(v2k_game::wander_near_location::WanderNearPrivateState, u32)> {
    Some(match task {
        ActorTaskRuntime::OrdinaryType9Wander(task) => (task.private_state(), task.elapsed_ms()),
        ActorTaskRuntime::SharedRetarget(task) => (task.private_state(), task.elapsed_ms()),
        ActorTaskRuntime::GoToJob(task) => (task.private_state(), task.elapsed_ms()),
        ActorTaskRuntime::RunAway(task) => (task.private_state(), task.elapsed_ms()),
        ActorTaskRuntime::AttractAttentionTargetRoute(task) => {
            (task.private_state(), task.elapsed_ms())
        }
        _ => return None,
    })
}

fn assert_native_soft_hook(mut fixture: Fixture, expected_style: u32) {
    assert!(fixture
        .scheduler
        .prepare_native_type9_external_mutation(&fixture.manager, fixture.peasant));
    fixture.place([-24961, 3, -26881], [0; 3]);
    let entity = fixture.manager.entity_mut(fixture.peasant).unwrap();
    entity.set_rotation_heading_pitch_roll_raw([0xefffu16 as i16, 123, -321]);
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        panic!("native context")
    };
    assert_eq!(context.active_style().style_address(), expected_style);
    let tasks_before =
        ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_task_state(slot).copied());
    let basis = entity.physical_body_basis_q31();
    let components = (
        entity.actor_animation_runtime,
        entity.sub_a_propulsion_runtime,
        entity.collision.callback_scheduler_accumulator_us_at_0x6c,
    );
    let position_before = entity.position_raw();
    // Independent, explicit fixture RNG starts at zero for this callback.
    // The native constructor/allocation receipts are kept intact.
    fixture.world_fx = WorldFx::new();
    let mut expected_fx = WorldFx::new();
    let x_word = expected_fx.next_shared_retail_random_u16();
    let z_word = expected_fx.next_shared_retail_random_u16();
    let late = resolve_ordinary_type9_late_contact(
        &mut v2k_game::intro2_contacts::Intro2ContactFrame {
            entities: &mut fixture.manager,
            resources: &mut fixture.session.cache,
            world_fx: &mut fixture.world_fx,
            static_damage: &mut fixture.static_damage,
            notifications: &mut fixture.notifications,
            retail_tick: 2,
            actor_tasks: &mut fixture.scheduler,
        },
        fixture.peasant,
        &fixture.metadata,
        None,
    );
    assert!(matches!(&late.pair_suffix,
        OrdinaryType9PairSuffix::Visited(v2k_game::native_actor_capture::pair::NativeCaptorPairOutcome::Resolved { visits }) if visits.is_empty()));
    let OrdinaryType9StaticContactOutcome::Applied(applied) = late.static_contact else {
        panic!("actual static-before-pair phase failed: {late:?}");
    };
    assert_eq!(applied.impact_raw, 0);
    let entity = fixture.entity();
    assert_eq!(
        entity.rotation_heading_pitch_roll_raw(),
        [0x0fff, 123, -321],
        "Sub-I heading wraps without reversing direction"
    );
    assert_eq!(
        entity.physical_body_basis_q31(),
        basis,
        "the contact plane uses the retained physical basis"
    );
    assert_eq!(
        (
            entity.actor_animation_runtime,
            entity.sub_a_propulsion_runtime,
            entity.collision.callback_scheduler_accumulator_us_at_0x6c
        ),
        components,
        "02CA0 does not execute D/I/A or scheduler ticks"
    );
    let tasks_after =
        ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_task_state(slot).copied());
    let mut hooks = 0;
    for (before, after) in tasks_before.into_iter().zip(tasks_after) {
        if let Some((mut expected, elapsed)) = before.and_then(static_private) {
            hooks += 1;
            expected.target_position_raw[0] =
                position_before[0].wrapping_add(((x_word >> 6) as i16).wrapping_sub(0x200));
            expected.target_position_raw[2] =
                position_before[2].wrapping_add(((z_word >> 6) as i16).wrapping_sub(0x200));
            assert_eq!(
                after.and_then(static_private),
                Some((expected, elapsed)),
                "only target X/Z changes; direction, timer, Y, tracked handle and elapsed survive"
            );
        } else {
            assert_eq!(
                after, before,
                "the acquisition/cue null+20 companions remain exact"
            );
        }
    }
    assert_eq!(
        hooks, 1,
        "each admitted Type9 graph owns one native02CA0 Primary"
    );
    assert_eq!(
        format!("{:?}", fixture.world_fx),
        format!("{expected_fx:?}"),
        "exactly X then Z RNG; no damage or cue suffix"
    );
    assert!(fixture
        .scheduler
        .ordinary_type9_completed_walking_owner(&fixture.manager, fixture.peasant));
    fixture.step(50);
    assert!(
        fixture
            .scheduler
            .ordinary_type9_completed_walking_owner(&fixture.manager, fixture.peasant),
        "the next native visit consumes the retained graph"
    );
}

#[v2k_test_support::retail_test]
fn every_native_type9_living_style_runs_its_current_static_task_and_null_companions() {
    for (cohort, style) in [
        (ConstructorCohort::Isolated, 0x004c_79c0),
        (ConstructorCohort::Base, 0x004c_8788),
        (ConstructorCohort::Hostile, 0x004c_7618),
        (ConstructorCohort::Player, 0x004c_86b0),
    ] {
        let fixture = Fixture::load_with_cohort(13, cohort);
        if matches!(cohort, ConstructorCohort::Player) {
            assert!(
                matches!(
                    fixture.entity().actor_task_state(ActorTaskSlot::Secondary),
                    Some(ActorTaskRuntime::AttractAttentionCandidate(_))
                ),
                "this constructor history exercises the native Candidate null+20 companion"
            );
            assert!(matches!(
                fixture.entity().actor_task_state(ActorTaskSlot::Tertiary),
                Some(ActorTaskRuntime::AttractAttentionCue(_))
            ));
        } else if matches!(cohort, ConstructorCohort::Hostile) {
            assert!(matches!(
                fixture.entity().actor_task_state(ActorTaskSlot::Secondary),
                Some(ActorTaskRuntime::TargetAcquisition(_))
            ));
        }
        assert_native_soft_hook(fixture, style);
    }
    for (cohort, family, style) in [
        (
            ConstructorCohort::Hostile,
            ActorTaskRuntimeFamily::RunAway,
            0x004c_7660,
        ),
        (
            ConstructorCohort::Player,
            ActorTaskRuntimeFamily::AttractAttentionTargetRoute,
            0x004c_86f8,
        ),
    ] {
        let mut fixture = Fixture::load_with_cohort(13, cohort);
        for tick in 1..=64 {
            fixture.step(tick);
            if fixture
                .entity()
                .actor_task_state(ActorTaskSlot::Primary)
                .unwrap()
                .family()
                == family
            {
                break;
            }
        }
        assert_eq!(
            fixture
                .entity()
                .actor_task_state(ActorTaskSlot::Primary)
                .unwrap()
                .family(),
            family,
            "native acquisition must produce the progressed callback"
        );
        assert_native_soft_hook(fixture, style);
    }
}

#[v2k_test_support::retail_test]
fn pen_fence_response_separates_peasant_without_damage_on_soft_overlap() {
    let mut fixture = Fixture::load();
    // Proven pen-fence pose from the spider controls (cell [157,151]).
    fixture.place([-24961, 3, -26881], [0; 3]);
    let health = fixture.health_raw();
    let applied = fixture.contact();
    assert_eq!(applied.contact.cell, [157, 151]);
    assert_eq!(applied.contact.kind_index, 9);
    assert!(applied.contact.penetration_raw > 0);
    assert_eq!(applied.impact_raw, 0);
    assert_eq!(applied.static_damage, None);
    assert_eq!(applied.actor_damage, None);
    assert_eq!(fixture.entity().position_raw(), applied.position_after_raw);
    assert_eq!(fixture.health_raw(), health);
}

#[v2k_test_support::retail_test]
fn pen_fence_hard_contact_delivers_static_and_actor_damage() {
    let mut fixture = Fixture::load();
    fixture.place([-24961, 3, -26881], [-256, 0, 0]);
    let health = fixture.health_raw();
    let applied = fixture.contact();
    assert_eq!(applied.contact.cell, [157, 151]);
    assert_eq!(applied.contact.model_id, 532);
    assert_eq!(applied.contact.response_raw, -256);
    assert!(applied.position_after_raw[0] > [-24961, 3, -26881][0]);
    assert!(applied.impact_raw > 0);
    assert!(applied.static_damage.is_some());
    assert!(applied.actor_damage.is_some());
    // A slow fence brush filters to zero against the peasant profile.
    assert_eq!(fixture.health_raw(), health);
}

#[v2k_test_support::retail_test]
fn separated_peasant_misses_and_keeps_pose() {
    let mut fixture = Fixture::load();
    fixture.place([0, 20_000, 0], [0; 3]);
    assert_eq!(fixture.resolve(), OrdinaryType9StaticContactOutcome::Miss);
    assert_eq!(fixture.entity().position_raw(), [0, 20_000, 0]);
    assert_eq!(fixture.entity().velocity_raw(), [0; 3]);
}

#[v2k_test_support::retail_test]
fn response_fully_separates_so_a_second_pass_misses() {
    let mut fixture = Fixture::load();
    fixture.place([-24961, 3, -26881], [-64, 0, 0]);
    fixture.contact();
    assert_eq!(
        fixture.resolve(),
        OrdinaryType9StaticContactOutcome::Miss,
        "one response pass must clear the fence overlap (pen containment)"
    );
}

#[v2k_test_support::retail_test]
fn non_peasant_subject_is_ineligible() {
    let mut fixture = Fixture::load();
    let spider = fixture
        .manager
        .iter_all()
        .find(|entity| entity.entity_type == 17)
        .expect("level-1 spider")
        .id;
    fixture.peasant = spider;
    assert_eq!(
        fixture.resolve(),
        OrdinaryType9StaticContactOutcome::Ineligible
    );
}

#[v2k_test_support::retail_test]
fn peasant_without_scheduler_custody_is_ineligible() {
    let mut fixture = Fixture::load();
    fixture.scheduler = SpecializedActorTaskScheduler::new();
    fixture.place([-24961, 3, -26881], [-64, 0, 0]);
    assert_eq!(
        fixture.resolve(),
        OrdinaryType9StaticContactOutcome::Ineligible
    );
}

#[v2k_test_support::retail_test]
fn world15_kind2_contact_filters_static_damage_then_keeps_the_real_peasant_owner() {
    let mut fixture = Fixture::load_world(15);
    fixture.peasant = 15;
    assert_eq!(fixture.entity().entity_type, 9);
    assert!(fixture
        .scheduler
        .ordinary_type9_completed_walking_owner(&fixture.manager, fixture.peasant));
    // Actual world15 model142/kind2 overlap from the release walk. A stronger
    // incoming velocity makes11760 deliver a nonzero, sub-threshold packet.
    fixture.place([30_449, -1183, 18_961], [-500, 0, 0]);
    let health = fixture.health_raw();
    // A filtered static packet consumes no chance/cue/particle RNG. A8B0's
    // earlier native02CA0 still consumes X then Z, including this branch.
    fixture.world_fx = WorldFx::new();
    let mut expected_fx = WorldFx::new();
    expected_fx.next_shared_retail_random_u16();
    expected_fx.next_shared_retail_random_u16();
    let terrain = *fixture
        .session
        .cache
        .terrain()
        .unwrap()
        .cell(117, 73)
        .unwrap();
    let applied = fixture.contact();
    assert_eq!(applied.contact.cell, [117, 73]);
    assert_eq!(
        (applied.contact.model_id, applied.contact.kind_index),
        (142, 2)
    );
    assert!((1..4000).contains(&applied.impact_raw), "{applied:?}");
    assert_eq!(
        applied.static_damage,
        Some(v2k_game::static_damage::StaticDamageOutcome::NoDamage { severity_raw: 0 })
    );
    assert!(
        applied.actor_damage.is_some(),
        "the static no-op must return to actor delivery"
    );
    assert_eq!(fixture.health_raw(), health);
    assert_eq!(fixture.static_damage.active_program_count(), 0);
    let after = fixture
        .session
        .cache
        .terrain()
        .unwrap()
        .cell(117, 73)
        .unwrap();
    assert_eq!(
        (after.height, after.attribute, after.terrain_type),
        (terrain.height, terrain.attribute, terrain.terrain_type)
    );
    assert_eq!(
        format!("{:?}", fixture.world_fx),
        format!("{expected_fx:?}"),
        "only02CA0 RNG; no static chance RNG, cues or particles"
    );
    assert!(fixture
        .scheduler
        .ordinary_type9_completed_walking_owner(&fixture.manager, fixture.peasant));
    for tick in 3..19 {
        let pass = fixture.scheduler.tick(
            &mut fixture.manager,
            SpecializedActorTaskProductionFrame {
                world: v2k_game::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                hive_components: None,
                resources: &mut fixture.session.cache,
                world_fx: &mut fixture.world_fx,
                static_damage: &mut fixture.static_damage,
                notification_phase:
                    v2k_game::gameplay_notifications::GameplayNotificationPhase::Playing,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: tick,
                main_base_abort_active: false,
            },
            &mut fixture.notifications,
        );
        assert!(pass.block.is_none(), "{pass:?}");
        for outcome in pass
            .outcomes
            .iter()
            .filter(|outcome| outcome.entity_id() == fixture.peasant)
        {
            let text = format!("{outcome:?}");
            assert!(
                !text.contains("Dropped") && !text.contains("Blocked"),
                "{text}"
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn stationary_first_world_keeps_walking_owners_after_repeated_static_responses() {
    let mut fixture = Fixture::load();
    fixture.manager.cleanup_pending_actor_deferred_destroys();
    let mut contacted = std::collections::BTreeSet::new();
    let mut contacts = 0;
    let mut soft_motion_contacts = 0;
    let initial_peasants: Vec<_> = fixture
        .manager
        .iter_all()
        .filter(|entity| entity.entity_type == 9)
        .map(|entity| entity.id)
        .collect();
    assert_eq!(initial_peasants.len(), 6);
    let mut native_terminals = std::collections::BTreeSet::new();
    // Player Sub-A and all terrain helpers consume their real constructor
    // draws before these walking owners. Preserve at least 1024 native ticks,
    // then allow bounded extra time for four natural wall-contact trajectories
    // instead of requiring the former process history to repeat by that tick.
    for tick in 0..4096 {
        let pass = fixture.scheduler.tick(
            &mut fixture.manager,
            SpecializedActorTaskProductionFrame {
                world: v2k_game::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                hive_components: None,
                notification_phase:
                    v2k_game::gameplay_notifications::GameplayNotificationPhase::Playing,
                resources: &mut fixture.session.cache,
                static_damage: &mut fixture.static_damage,
                world_fx: &mut fixture.world_fx,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: tick,
                main_base_abort_active: false,
            },
            &mut fixture.notifications,
        );
        assert!(pass.block.is_none(), "actor pass at tick{tick}: {pass:?}");
        for outcome in &pass.outcomes {
            // Only actual selected peasant owners were adopted. Their nested
            // root continuations must also remain free of blocks and drops.
            let diagnostic = format!("{outcome:?}");
            assert!(
                !diagnostic.contains("Dropped") && !diagnostic.contains("Blocked"),
                "tick{tick}: {diagnostic}"
            );
        }
        for &id in &initial_peasants {
            match fixture.scheduler.family_for(id) {
                Some(SpecializedActorTaskFamily::OrdinaryType9Class14 | SpecializedActorTaskFamily::Intro2Type9Class14) => {
                    native_terminals.insert(id);
                }
                _ if fixture.manager.iter_all().any(|entity| entity.id == id && entity.active) => {
                    assert!(fixture.scheduler.ordinary_type9_completed_walking_owner(&fixture.manager, id), "every surviving peasant retains native custody at tick{tick}: actor{id}");
                }
                _ => assert!(native_terminals.contains(&id), "removal requires an observed native Class14 continuation: actor{id} tick{tick}"),
            }
        }
        let ids: Vec<_> = fixture
            .manager
            .iter_all()
            .filter(|entity| entity.entity_type == 9)
            .map(|entity| entity.id)
            .collect();
        for id in ids {
            let outcome = resolve_ordinary_type9_static_contact(
                &mut fixture.manager,
                id,
                &fixture.metadata,
                OrdinaryType9StaticContactFrame {
                    resources: &mut fixture.session.cache,
                    static_damage: &mut fixture.static_damage,
                    world_fx: &mut fixture.world_fx,
                    scheduler: &mut fixture.scheduler,
                    notifications: &mut fixture.notifications,
                    retail_tick: tick,
                },
            );
            if let OrdinaryType9StaticContactOutcome::Applied(applied) = outcome {
                contacts += 1;
                contacted.insert(id);
                if applied.impact_raw == 0
                    && (applied.position_before_raw != applied.position_after_raw
                        || applied.velocity_before_raw != applied.velocity_after_raw)
                {
                    soft_motion_contacts += 1;
                }
                assert!(
                    fixture
                        .scheduler
                        .ordinary_type9_completed_walking_owner(&fixture.manager, id),
                    "contact must retain the current walking graph at tick{tick}"
                );
            } else {
                assert!(
                    !matches!(outcome, OrdinaryType9StaticContactOutcome::Blocked { .. }),
                    "contact tick{tick} actor{id}: {outcome:?}"
                );
            }
        }
        fixture.world_fx.process_pending();
        fixture.world_fx.take_positional_sounds();
        let player = fixture.manager.player().unwrap().position;
        fixture.scheduler.publish_presented_view_detail(
            &mut fixture.manager,
            v2k_game::entity_view_detail::RetailViewDetailContext::from_world(
                [player[0], player[1] + 8.0, player[2] - 8.0],
                0.7,
                (32, 32),
            ),
        );
        if tick >= 1023 && contacted.len() >= 4 {
            break;
        }
    }
    assert!(
        contacts >= 20,
        "real static contacts must execute: {contacts}"
    );
    assert!(
        soft_motion_contacts > 0,
        "zero-damage motion still needs custody"
    );
    // Keeping earlier actors alive also keeps their later shared RNG draws.
    // That legitimately changes which later actors reach a wall in this run.
    assert!(
        contacted.len() >= 4,
        "multiple real walking owners: {contacted:?}"
    );
    eprintln!("Level1 native contacts={contacts}, soft={soft_motion_contacts}, contacted={contacted:?}, uncontacted={:?}, Class14={native_terminals:?}", initial_peasants.into_iter().filter(|id| !contacted.contains(id)).collect::<Vec<_>>());
}

#[v2k_test_support::retail_test]
fn parked_contact_prefix_rejects_static_motion_without_touching_body_or_tasks() {
    let mut fixture = Fixture::load();
    fixture.place([-24961, 3, -26881], [-64, 0, 0]);
    assert!(fixture
        .scheduler
        .park_native_contact_prefix(&fixture.manager, fixture.peasant));
    let position = fixture.entity().position_raw();
    let velocity = fixture.entity().velocity_raw();
    let tasks = v2k_game::actor_task_owner::ActorTaskSlot::IN_RETAIL_TICK_ORDER
        .map(|slot| fixture.entity().actor_task_state(slot).copied());
    assert_eq!(
        fixture.resolve(),
        OrdinaryType9StaticContactOutcome::Ineligible
    );
    assert_eq!(fixture.entity().position_raw(), position);
    assert_eq!(fixture.entity().velocity_raw(), velocity);
    assert_eq!(
        v2k_game::actor_task_owner::ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .map(|slot| fixture.entity().actor_task_state(slot).copied()),
        tasks
    );
}

#[v2k_test_support::retail_test]
fn late_damage_failure_parks_the_committed_static_response() {
    let mut fixture = Fixture::load();
    fixture.place([-24961, 3, -26881], [-30_000, 0, 0]);
    fixture
        .manager
        .entity_mut(fixture.peasant)
        .unwrap()
        .collision
        .health_raw = RetailRuntimeValue::Unresolved;
    let position_before = fixture.entity().position_raw();
    assert!(matches!(
        fixture.resolve(),
        OrdinaryType9StaticContactOutcome::Blocked {
            reason:
                v2k_game::ordinary_type9_static_contact::OrdinaryType9StaticContactBlock::Damage(_),
            committed_prefix: true,
        }
    ));
    assert_ne!(fixture.entity().position_raw(), position_before);
    assert!(!fixture
        .scheduler
        .ordinary_type9_completed_walking_owner(&fixture.manager, fixture.peasant));
    // Restore overlap only to make the second scan reach the custody gate.
    // Parking still rejects a writer even if every wrapper is out of callback.
    fixture.place(position_before, [-30_000, 0, 0]);
    let effects = format!("{:?}", fixture.world_fx);
    let tasks = v2k_game::actor_task_owner::ActorTaskSlot::IN_RETAIL_TICK_ORDER
        .map(|slot| fixture.entity().actor_task_state(slot).copied());
    assert_eq!(
        fixture.resolve(),
        OrdinaryType9StaticContactOutcome::Ineligible
    );
    assert_eq!(fixture.entity().position_raw(), position_before);
    assert_eq!(fixture.entity().velocity_raw(), [-30_000, 0, 0]);
    assert_eq!(format!("{:?}", fixture.world_fx), effects);
    assert_eq!(
        v2k_game::actor_task_owner::ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .map(|slot| fixture.entity().actor_task_state(slot).copied()),
        tasks
    );
}
