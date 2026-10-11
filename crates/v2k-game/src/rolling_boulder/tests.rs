use super::*;
use crate::{
    damage::{DamagePacket, FUN_0043F780_DAMAGE_PACKET},
    entity_collision_state::DYING_STATE_BIT,
    gameplay_notifications::{GameplayNotificationPhase, GameplayNotifications},
    intro2_contacts::Intro2ContactFrame,
    intro2_gun_turret::authored_tests::fixture,
    native_actor_surface_contact::{
        resolve_native_actor_surface_contact, NativeActorSurfaceContactOutcome,
    },
    native_ground_actor::contact::{resolve_insect_static_contact, NativeGroundContactOutcome},
    player_hull::PlayerHull,
    session::GameSession,
    shared_actor_impact::{
        apply_playing_actor_particle_hit, PlayingActorImpactFrame, SharedActorImpactOutcome,
    },
    specialized_actor_task_production::{
        SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
        SpecializedActorTaskScheduler, SpecializedActorTaskWorld,
    },
    static_contact::{scan_deepest_static_contact, StaticContactQuery, StaticModelContact},
    static_damage::StaticDamageScheduler,
    world_fx::{BallisticDamageRequest, ParticleEntityImpact},
};

/// One authored world with only its boulders adopted by the scheduler.
struct BoulderWorld {
    session: GameSession,
    manager: EntityManager,
    fx: WorldFx,
    tasks: SpecializedActorTaskScheduler,
    static_damage: StaticDamageScheduler,
    notifications: GameplayNotifications,
    player_hull: PlayerHull,
    tick: u32,
}

struct BoulderFrame {
    task: Option<RollingBoulderOutcome>,
    surface: NativeActorSurfaceContactOutcome,
    static_contact: NativeGroundContactOutcome,
}

impl BoulderWorld {
    fn new(level: u32) -> Self {
        let (session, manager, fx) = fixture(level);
        let mut tasks = SpecializedActorTaskScheduler::new();
        tasks.adopt_rolling_boulders(&manager);
        Self {
            session,
            manager,
            fx,
            tasks,
            static_damage: StaticDamageScheduler::new(),
            notifications: GameplayNotifications::new(),
            player_hull: PlayerHull::default(),
            tick: 0,
        }
    }

    /// Leave only `id` eligible for radials, contacts and pairs.
    fn isolate(&mut self, id: u32) {
        let others: Vec<_> = self
            .manager
            .iter_all()
            .map(|entity| entity.id)
            .filter(|&other| other != id)
            .collect();
        for other in others {
            self.manager
                .entity_mut(other)
                .unwrap()
                .collision
                .state_flags_at_0x08
                .overwrite(!0x2000, 0);
        }
    }

    /// One particle impact through the Playing dispatch, as main.rs routes it.
    fn hit(
        &mut self,
        id: u32,
        class: u8,
        packet: DamagePacket,
    ) -> impact::RollingBoulderImpactOutcome {
        self.tick += 1;
        let outcome = apply_playing_actor_particle_hit(
            PlayingActorImpactFrame {
                extra_lives: RetailRuntimeValue::Unresolved,
                resources: &mut self.session.cache,
                entities: &mut self.manager,
                world_fx: &mut self.fx,
                scheduler: &mut self.tasks,
                notifications: &mut self.notifications,
                static_damage: &mut self.static_damage,
                player_hull: &mut self.player_hull,
                retail_tick: self.tick,
            },
            ParticleEntityImpact {
                source_particle_class: class,
                impact_position_argument_va: if class == 5 { 0x004d_cf48 } else { 0 },
                target_entity_id: id,
                position_world: [0.0; 3],
                velocity_raw: [0, 0, 8192],
                damage: Some(BallisticDamageRequest {
                    packet,
                    source_entity_type_at_birth: Some(34),
                    source_owner_id: Some(35),
                }),
            },
        );
        let Some(SharedActorImpactOutcome::RollingBoulder(outcome)) = outcome else {
            panic!("{outcome:?}")
        };
        outcome
    }

    fn ids(&self) -> Vec<u32> {
        self.manager.iter_all().map(|entity| entity.id).collect()
    }

    fn first(&self, entity_type: u32) -> u32 {
        self.manager
            .iter_all()
            .find(|entity| entity.entity_type == entity_type)
            .unwrap()
            .id
    }

    fn entity(&self, id: u32) -> &Entity {
        self.manager
            .iter_all()
            .find(|entity| entity.id == id)
            .unwrap()
    }

    /// Frontend activation (8000, zero +70) and a detailed visit tier
    /// (20000 callbacks, 02000000 detail), as for an actor near the player.
    fn activate(&mut self, id: u32) {
        let entity = self.manager.entity_mut(id).unwrap();
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(0x0202_8000, 0x0202_8000);
        entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
    }

    fn contact_frame(&mut self) -> Intro2ContactFrame<'_> {
        Intro2ContactFrame {
            entities: &mut self.manager,
            resources: &mut self.session.cache,
            world_fx: &mut self.fx,
            static_damage: &mut self.static_damage,
            notifications: &mut self.notifications,
            retail_tick: self.tick,
            actor_tasks: &mut self.tasks,
        }
    }

    /// One task pass, then each subject's late 11AD0 surface/static phases.
    fn frame_all(&mut self, ids: &[u32]) -> Vec<BoulderFrame> {
        let pass = self.tick_tasks();
        ids.iter()
            .map(|&id| {
                let task = pass
                    .iter()
                    .find(|outcome| outcome.entity_id() == id)
                    .cloned();
                let (surface, static_contact) = self.contacts(id);
                BoulderFrame {
                    task,
                    surface,
                    static_contact,
                }
            })
            .collect()
    }

    fn tick_tasks(&mut self) -> Vec<RollingBoulderOutcome> {
        self.tick += 1;
        let pass = self.tasks.tick(
            &mut self.manager,
            SpecializedActorTaskProductionFrame {
                world: SpecializedActorTaskWorld::Cinematic,
                hive_components: None,
                notification_phase: GameplayNotificationPhase::Playing,
                resources: &mut self.session.cache,
                world_fx: &mut self.fx,
                static_damage: &mut self.static_damage,
                elapsed_micros: 20_000,
                global_elapsed_micros: self.tick * 20_000,
                retail_tick: self.tick,
                main_base_abort_active: false,
            },
            &mut self.notifications,
        );
        assert!(pass.block.is_none(), "{:?}", pass.block);
        pass.outcomes
            .into_iter()
            .filter_map(|outcome| match outcome {
                SpecializedActorTaskProductionOutcome::RollingBoulder(outcome) => Some(outcome),
                _ => None,
            })
            .collect()
    }

    fn contacts(
        &mut self,
        id: u32,
    ) -> (NativeActorSurfaceContactOutcome, NativeGroundContactOutcome) {
        let surface = resolve_native_actor_surface_contact(&mut self.contact_frame(), id);
        let static_contact = if matches!(surface, NativeActorSurfaceContactOutcome::Blocked { .. })
        {
            NativeGroundContactOutcome::Ineligible
        } else {
            resolve_insect_static_contact(&mut self.contact_frame(), id)
        };
        (surface, static_contact)
    }

    /// One task pass, then the subject's late 11AD0 surface and static phases.
    fn frame(&mut self, id: u32) -> BoulderFrame {
        self.frame_all(&[id]).pop().unwrap()
    }

    /// First static overlap in a deterministic sweep around the boulder.
    fn find_static_overlap(&self, id: u32) -> Option<([i16; 3], StaticModelContact)> {
        let entity = self.entity(id);
        let center = entity.position_raw();
        let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
            return None;
        };
        let model = self.session.cache.global_model(entity.model_index?)?;
        for radius in (256..=8192).step_by(256) {
            for dx in (-radius..=radius).step_by(128) {
                for dz in [-radius, radius] {
                    for (dx, dz) in [(dx, dz), (dz, dx)] {
                        let position = [
                            center[0].wrapping_add(dx as i16),
                            center[1],
                            center[2].wrapping_add(dz as i16),
                        ];
                        let contact = scan_deepest_static_contact(StaticContactQuery {
                            terrain: self.session.cache.terrain()?,
                            terrain_objects: self.session.cache.terrain_objects()?,
                            model_pool: &self.session.cache,
                            tick: self.tick,
                            active_model: model,
                            active_model_to_world_basis: basis
                                .orientation_world_from_model()
                                .map(|row| row.map(f64::from)),
                            active_anim_vars: &entity.presentation_anim_vars(self.tick),
                            position_raw: position,
                        })
                        .ok()?;
                        if let Some(contact) = contact.filter(|contact| contact.response_raw < 0) {
                            return Some((position, contact));
                        }
                    }
                }
            }
        }
        None
    }
}

#[test]
fn style_install_sets_motion_and_contact_bits_only_while_rolling() {
    // 40D440(0x1005): bit1 -> clear 0x10000, bit1000 -> clear 0x40000.
    assert_eq!(
        apply_style_install_state(0, RollingBoulderStyle::Rolling),
        0x50000
    );
    assert_eq!(
        apply_style_install_state(u32::MAX, RollingBoulderStyle::Resting),
        !0x50000
    );
    // Unrelated bits survive both installs.
    assert_eq!(
        apply_style_install_state(0x8000, RollingBoulderStyle::Resting),
        0x8000
    );
    assert_eq!(RollingBoulderStyle::Rolling.effective_policy(), 0x4060);
    assert_eq!(RollingBoulderStyle::Resting.effective_policy(), 0x5065);
}

#[test]
fn resting_task_wakes_above_one_hundred_and_otherwise_stops() {
    let mut task = BoulderRestingTaskState::new();
    let mut slow = [60, 0, 80];
    assert!(!task.step(&mut slow, 20_000, true));
    assert_eq!(slow, [0; 3], "isqrt(3600+6400)=100 is not above 100");
    let mut fast = [0, -101, 0];
    assert!(task.step(&mut fast, 20_000, true));
    assert_eq!(fast, [0, -101, 0]);
    // A restricted visit neither wakes nor stops the body.
    let mut coarse = [0, 500, 0];
    assert!(!task.step(&mut coarse, 20_000, false));
    assert_eq!(coarse, [0, 500, 0]);
    assert_eq!(task.elapsed_ms, 60);
}

#[v2k_test_support::retail_test]
fn authored_boulders_publish_rolling_style_on_the_ground() {
    let mut births = 0;
    for (level, large, small) in [(27, 6, 0), (31, 7, 3), (35, 12, 2)] {
        let (session, manager, _) = fixture(level);
        let terrain = session.cache.terrain().unwrap();
        let mut counted = [0; 2];
        for entity in manager
            .iter_all()
            .filter(|entity| matches!(entity.entity_type, 3 | 27))
        {
            births += 1;
            let profile = RollingBoulderProfile::for_entity_type(entity.entity_type).unwrap();
            counted[usize::from(profile == RollingBoulderProfile::Large)] += 1;
            let runtime = entity.rolling_boulder_runtime.unwrap();
            assert_eq!(runtime.profile, profile);
            let extent = session
                .cache
                .global_model(usize::from(profile.model()))
                .unwrap()
                .radius;
            let spawn =
                &session.cache.level_desc().unwrap().entities[entity.authored_spawn_index.unwrap()];
            let [x, _, z] = spawn.position_raw();
            assert_eq!(
                entity.position_raw(),
                [
                    x,
                    terrain
                        .bilinear_height_raw(x, z)
                        .wrapping_add(extent as i16),
                    z
                ],
                "world{level} spawn{}",
                spawn.index
            );
            assert_eq!(runtime.anchor_raw, entity.position_raw());
            assert_eq!(current_style(entity), Ok(RollingBoulderStyle::Rolling));
            assert_eq!(
                entity.collision.state_flags_at_0x08.masked(0x50000),
                RetailRuntimeValue::Known(0x50000)
            );
            assert!(matches!(
                entity.actor_task_state(ActorTaskSlot::Primary),
                Some(ActorTaskRuntime::BoulderRolling(_))
            ));
            assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
            assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
            assert_eq!(
                entity.collision.health_raw,
                RetailRuntimeValue::Known(profile.health())
            );
            let owner = RollingBoulderOwner::adopt(&manager, entity.id).unwrap();
            assert_eq!(owner.style(), RollingBoulderStyle::Rolling);
        }
        assert_eq!(counted, [small, large], "world{level}");
        let mut tasks = SpecializedActorTaskScheduler::new();
        assert_eq!(tasks.adopt_rolling_boulders(&manager), small + large);
        assert_eq!(tasks.adopt_rolling_boulders(&manager), 0, "one owner each");
    }
    assert_eq!(births, 30);
}

#[v2k_test_support::retail_test]
fn a_boulder_lands_rolls_and_comes_to_rest() {
    let mut world = BoulderWorld::new(27);
    let id = world.first(27);
    world.activate(id);
    let start = world.entity(id).position_raw();
    let mut landed = false;
    let mut rested = false;
    for _ in 0..3000 {
        let frame = world.frame(id);
        assert!(
            !matches!(
                frame.surface,
                NativeActorSurfaceContactOutcome::Blocked { .. }
            ),
            "tick{}: {:?}",
            world.tick,
            frame.surface
        );
        assert!(
            !matches!(
                frame.static_contact,
                NativeGroundContactOutcome::Blocked { .. }
            ),
            "tick{}: {:?}",
            world.tick,
            frame.static_contact
        );
        landed |= matches!(
            frame.surface,
            NativeActorSurfaceContactOutcome::Applied {
                solid_contact: true,
                ..
            }
        );
        match frame.task.expect("the adopted owner is visited") {
            RollingBoulderOutcome::Advanced {
                switched: true,
                style: RollingBoulderStyle::Resting,
                ..
            } => {
                rested = true;
                break;
            }
            RollingBoulderOutcome::Advanced { switched, .. } => {
                assert!(!switched, "tick{}", world.tick)
            }
            RollingBoulderOutcome::Waiting { .. } => {}
            other => panic!("tick{}: {other:?}", world.tick),
        }
    }
    assert!(landed, "gravity brings the boulder onto its terrain");
    assert!(rested, "a grounded boulder comes to rest");
    let entity = world.entity(id);
    assert_ne!(entity.position_raw(), start, "it fell before it stopped");
    assert_eq!(current_style(entity), Ok(RollingBoulderStyle::Resting));
    assert_eq!(
        entity.collision.state_flags_at_0x08.masked(0x50000),
        RetailRuntimeValue::Known(0),
        "resting clears terrain admission and master motion"
    );
    assert!(matches!(
        entity.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::BoulderResting(_))
    ));
    // At rest there is no gravity, motion or terrain visit until a push.
    let position = entity.position_raw();
    for _ in 0..40 {
        let frame = world.frame(id);
        assert_eq!(frame.surface, NativeActorSurfaceContactOutcome::Ineligible);
        assert!(!matches!(
            frame.static_contact,
            NativeGroundContactOutcome::Blocked { .. }
        ));
    }
    let entity = world.entity(id);
    assert_eq!(entity.position_raw(), position);
    assert_eq!(entity.velocity_raw(), [0; 3]);
    assert_eq!(current_style(entity), Ok(RollingBoulderStyle::Resting));
}

#[v2k_test_support::retail_test]
fn a_pushed_resting_boulder_wakes_on_its_next_detailed_visit() {
    let mut world = BoulderWorld::new(27);
    let id = world.first(27);
    world.activate(id);
    let entity = world.manager.entity_mut(id).unwrap();
    switch_style(entity, RollingBoulderStyle::Resting).unwrap();
    entity.set_velocity_raw([0, 0, 400]);
    // An external style switch replaces the Primary; adopt the current owner.
    world.tasks = SpecializedActorTaskScheduler::new();
    assert_eq!(world.tasks.adopt_rolling_boulders(&world.manager), 6);
    let mut woke = false;
    for _ in 0..4 {
        match world.frame(id).task.unwrap() {
            RollingBoulderOutcome::Advanced {
                switched: true,
                style: RollingBoulderStyle::Rolling,
                ..
            } => {
                woke = true;
                break;
            }
            RollingBoulderOutcome::Waiting { .. } => {}
            other => panic!("{other:?}"),
        }
    }
    assert!(woke, "speed400 is above the 404B60 threshold");
    let entity = world.entity(id);
    assert_eq!(current_style(entity), Ok(RollingBoulderStyle::Rolling));
    assert_eq!(
        entity.collision.state_flags_at_0x08.masked(0x50000),
        RetailRuntimeValue::Known(0x50000)
    );
    assert!(matches!(
        entity.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::BoulderRolling(_))
    ));
}

#[v2k_test_support::retail_test]
fn a_resting_boulder_hit_through_style1_returns_to_rolling() {
    let (_, mut manager, _) = fixture(27);
    let id = manager
        .iter_all()
        .find(|entity| entity.entity_type == 27)
        .unwrap()
        .id;
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(
        wake_resting_boulder(entity),
        Ok(false),
        "style0 hooks are null"
    );
    switch_style(entity, RollingBoulderStyle::Resting).unwrap();
    assert_eq!(wake_resting_boulder(entity), Ok(true));
    assert_eq!(current_style(entity), Ok(RollingBoulderStyle::Rolling));
    assert!(RollingBoulderOwner::adopt(&manager, id).is_ok());
}

#[v2k_test_support::retail_test]
fn a_boulder_driven_into_a_static_model_takes_the_bare_11760_response() {
    for level in [27, 31, 35] {
        let mut world = BoulderWorld::new(level);
        let id = world.first(27);
        world.activate(id);
        let Some((position, contact)) = world.find_static_overlap(id) else {
            continue;
        };
        // Drive the boulder into the plane so 11760 separates and reflects.
        let velocity = contact
            .normal_q12
            .map(|n| (-(i32::from(n)) * 300 / 4096) as i16);
        let entity = world.manager.entity_mut(id).unwrap();
        entity.set_motion_raw(position, velocity);
        let health = entity.collision.health_raw;
        world.tick += 1;
        let outcome = resolve_insect_static_contact(&mut world.contact_frame(), id);
        let NativeGroundContactOutcome::Applied(applied) = outcome else {
            panic!("world{level}: {outcome:?}");
        };
        assert_eq!(applied.contact, contact);
        assert_eq!(applied.position_before_raw, position);
        assert_eq!(applied.velocity_before_raw, velocity);
        let mut expected_position = position;
        let mut expected_velocity = velocity;
        crate::static_contact::apply_contact_response_raw(
            &mut expected_position,
            &mut expected_velocity,
            contact,
        );
        assert_eq!(applied.position_after_raw, expected_position);
        assert_eq!(applied.velocity_after_raw, expected_velocity);
        assert_eq!(
            (applied.crushing_damage, applied.furniture_damage),
            (None, None),
            "effective4060 has no D9B0 crush and the hooks are null"
        );
        let entity = world.entity(id);
        assert_eq!(entity.position_raw(), expected_position);
        assert_eq!(current_style(entity), Ok(RollingBoulderStyle::Rolling));
        if applied.impact_raw == 0 {
            assert_eq!(entity.collision.health_raw, health);
        }
        return;
    }
    panic!("an authored boulder world has static geometry near a boulder");
}

#[v2k_test_support::retail_test]
fn a_block_after_the_committed_prefix_parks_the_owner() {
    let mut world = BoulderWorld::new(27);
    let id = world.first(27);
    world.activate(id);
    world
        .manager
        .entity_mut(id)
        .unwrap()
        .collision
        .animation_offset_at_0xb2 = RetailRuntimeValue::Unresolved;
    let first = world.frame(id);
    assert_eq!(
        first.task,
        Some(RollingBoulderOutcome::Blocked {
            entity_id: id,
            reason: RollingBoulderBlock::Runtime("callback B2"),
        })
    );
    let entity = world.entity(id);
    let (position, velocity, state) = (
        entity.position_raw(),
        entity.velocity_raw(),
        entity.collision.state_flags_at_0x08,
    );
    // The parked owner neither replays the visit nor admits a contact.
    let second = world.frame(id);
    assert_eq!(
        second.task,
        Some(RollingBoulderOutcome::Blocked {
            entity_id: id,
            reason: RollingBoulderBlock::Runtime("parked prefix"),
        })
    );
    assert!(matches!(
        second.surface,
        NativeActorSurfaceContactOutcome::Blocked {
            committed_prefix: false,
            ..
        }
    ));
    let entity = world.entity(id);
    assert_eq!(entity.position_raw(), position);
    assert_eq!(entity.velocity_raw(), velocity);
    assert_eq!(entity.collision.state_flags_at_0x08, state);
}

#[v2k_test_support::retail_test]
fn radial_blasts_reach_a_boulder_only_through_its_completed_owner() {
    use crate::{
        damage::DamagePacket,
        entity::{DynamicRadialLiveBlockReason, DynamicRadialLivePhase, DynamicRadialLiveRequest},
        intro2_radial::Intro2RadialTaskCustody,
        radial_damage::RadialDamageTemplate,
    };
    for parked in [false, true] {
        let mut world = BoulderWorld::new(27);
        let id = world.first(27);
        world.activate(id);
        if parked {
            assert!(world.tasks.park_native_contact_prefix(&world.manager, id));
        }
        let entity = world.entity(id);
        let center = entity.position_raw();
        let (collision, velocity) = (entity.collision.clone(), entity.velocity_raw());
        let BoulderWorld {
            manager,
            fx,
            tasks,
            notifications,
            ..
        } = &mut world;
        let result = manager.apply_dynamic_radial_damage_live(DynamicRadialLiveRequest {
            origin_raw: [center[0].wrapping_add(64), center[1], center[2]],
            template: RadialDamageTemplate {
                inner_radius_raw: 0,
                outer_radius_raw: 512,
                impulse_raw: 256,
                packet: DamagePacket::collision(2100),
                trailing_raw: [17, 0],
            },
            world_fx: fx,
            retail_tick: 100,
            notifications,
            callbacks: &mut |manager: &EntityManager, target| {
                tasks.prepare_native_actor_mutation(manager, target)
            },
        });
        let entity = world.entity(id);
        if parked {
            let block = result.blocked.expect("a parked boulder holds the blast");
            assert_eq!(block.target_id, id);
            assert_eq!(block.phase, DynamicRadialLivePhase::MutationCustody);
            assert_eq!(
                block.reason,
                DynamicRadialLiveBlockReason::NativeActorMutationCustody
            );
            assert!(!block.target_prefix_committed);
            assert_eq!(entity.collision, collision);
            assert_eq!(entity.velocity_raw(), velocity);
        } else {
            assert!(result.blocked.is_none(), "{result:?}");
            assert!(
                entity.velocity_raw()[0] < velocity[0],
                "the blast pushes the boulder away from its origin"
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn playing_blasts_take_the_native_path_through_the_completed_owner() {
    use crate::{
        damage::DamagePacket,
        entity::{DynamicRadialLiveBlockReason, DynamicRadialLivePhase},
        player_hull::PlayerHull,
        radial_damage::RadialDamageTemplate,
        specialized_actor_task_production::{PlayingRadialBlock, PlayingRadialFrame},
    };
    // Channel2 (threshold 4000): 5000 delivers 1000; a 1-health body dies.
    for (parked, lethal) in [(false, false), (true, false), (false, true)] {
        let mut world = BoulderWorld::new(27);
        let id = world.first(27);
        let others: Vec<_> = world
            .manager
            .iter_all()
            .map(|entity| entity.id)
            .filter(|&other| other != id)
            .collect();
        for other in others {
            world
                .manager
                .entity_mut(other)
                .unwrap()
                .collision
                .state_flags_at_0x08
                .overwrite(!0x2000, 0);
        }
        world.activate(id);
        if parked {
            assert!(world.tasks.park_native_contact_prefix(&world.manager, id));
        }
        if lethal {
            world.manager.entity_mut(id).unwrap().collision.health_raw =
                RetailRuntimeValue::Known(1);
        }
        let entity = world.entity(id);
        let center = entity.position_raw();
        let (collision, velocity) = (entity.collision.clone(), entity.velocity_raw());
        let mut player_hull = PlayerHull::default();
        let BoulderWorld {
            session,
            manager,
            fx,
            tasks,
            static_damage,
            notifications,
            ..
        } = &mut world;
        let outcome = tasks.apply_playing_radial_damage(PlayingRadialFrame {
            entities: manager,
            player_hull: &mut player_hull,
            extra_lives: RetailRuntimeValue::Unresolved,
            origin_raw: [center[0].wrapping_add(64), center[1], center[2]],
            template: RadialDamageTemplate {
                inner_radius_raw: 128,
                outer_radius_raw: 512,
                impulse_raw: 256,
                packet: DamagePacket {
                    channels: [2, 0],
                    amounts_raw: [5000, 0],
                },
                trailing_raw: [17, 0],
            },
            world_fx: fx,
            notifications,
            retail_tick: 100,
            resources: &mut session.cache,
            static_damage,
            active_terminal_calls: Vec::new(),
        });
        let entity = world.entity(id);
        match (parked, lethal) {
            (true, _) => {
                let Some(PlayingRadialBlock::Native(block)) = outcome.blocked else {
                    panic!("{outcome:?}")
                };
                assert_eq!(block.phase, DynamicRadialLivePhase::MutationCustody);
                assert_eq!(
                    block.reason,
                    DynamicRadialLiveBlockReason::NativeActorMutationCustody
                );
                assert_eq!(entity.collision, collision);
                assert_eq!(entity.velocity_raw(), velocity);
            }
            (false, false) => {
                assert!(outcome.blocked.is_none(), "{outcome:?}");
                assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(9000));
                assert!(
                    entity.velocity_raw()[0] < velocity[0],
                    "the blast pushes the boulder away from its origin"
                );
            }
            (false, true) => {
                // The lethal blast runs class18 inside the native radial walk.
                assert!(outcome.blocked.is_none(), "{outcome:?}");
                assert!(death::finished_split_authenticates(&world.manager, id));
                assert_eq!(
                    world
                        .manager
                        .iter_all()
                        .filter(|entity| entity.entity_type == 3)
                        .count(),
                    2,
                    "world27 authors no Type3; both are split children"
                );
            }
        }
    }
}

/// Channel2 (threshold 4000, multiplier 256): 5000 delivers 1000 damage.
const SHOT: DamagePacket = DamagePacket {
    channels: [2, 0],
    amounts_raw: [5000, 0],
};

#[v2k_test_support::retail_test]
fn a_primary_hit_wakes_a_resting_boulder_and_pushes_it() {
    let mut world = BoulderWorld::new(27);
    let id = world.first(27);
    world.isolate(id);
    world.activate(id);
    switch_style(
        world.manager.entity_mut(id).unwrap(),
        RollingBoulderStyle::Resting,
    )
    .unwrap();
    world.tasks = SpecializedActorTaskScheduler::new();
    world.tasks.adopt_rolling_boulders(&world.manager);
    let velocity = world.entity(id).velocity_raw();
    let outcome = world.hit(id, 16, SHOT);
    let impact::RollingBoulderImpactOutcome::Applied(applied) = outcome else {
        panic!("{outcome:?}")
    };
    assert_eq!(applied.filtered_damage_raw, 1000);
    let entity = world.entity(id);
    assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(9000));
    assert_eq!(
        entity.collision.last_hit_presentation_tick_at_0x34,
        RetailRuntimeValue::Known(world.tick)
    );
    assert_eq!(
        current_style(entity),
        Ok(RollingBoulderStyle::Rolling),
        "style1 +28 is 40C730"
    );
    assert_ne!(entity.velocity_raw(), velocity, "11030 pushes the boulder");
    // The scheduler now owns the replaced rolling Primary.
    let frame = world.frame(id);
    assert!(
        matches!(
            frame.task,
            Some(RollingBoulderOutcome::Advanced { .. } | RollingBoulderOutcome::Waiting { .. })
        ),
        "{:?}",
        frame.task
    );
}

#[v2k_test_support::retail_test]
fn an_infected_hit_reaches_null_slots_and_leaves_a_resting_boulder_at_rest() {
    let mut world = BoulderWorld::new(27);
    let id = world.first(27);
    world.isolate(id);
    world.activate(id);
    switch_style(
        world.manager.entity_mut(id).unwrap(),
        RollingBoulderStyle::Resting,
    )
    .unwrap();
    world.tasks = SpecializedActorTaskScheduler::new();
    world.tasks.adopt_rolling_boulders(&world.manager);
    let stamp = world
        .entity(id)
        .collision
        .last_hit_presentation_tick_at_0x34;
    let outcome = world.hit(id, 5, FUN_0043F780_DAMAGE_PACKET);
    assert!(
        matches!(outcome, impact::RollingBoulderImpactOutcome::Applied(_)),
        "{outcome:?}"
    );
    let entity = world.entity(id);
    assert_eq!(
        current_style(entity),
        Ok(RollingBoulderStyle::Resting),
        "DA00 reads the null +20"
    );
    assert_eq!(entity.collision.last_hit_presentation_tick_at_0x34, stamp);
    assert!(RollingBoulderOwner::adopt(&world.manager, id).is_ok());
}

#[v2k_test_support::retail_test]
fn a_lethal_hit_explodes_a_small_boulder_through_class1() {
    let mut world = BoulderWorld::new(31);
    let id = world.first(3);
    world.isolate(id);
    world.activate(id);
    world.manager.entity_mut(id).unwrap().collision.health_raw = RetailRuntimeValue::Known(1);
    let particles = world.fx.particle_count();
    let outcome = world.hit(id, 16, SHOT);
    assert!(
        matches!(outcome, impact::RollingBoulderImpactOutcome::Applied(_)),
        "{outcome:?}"
    );
    assert!(crate::class49_death::finished_terminal_hit_authenticates(
        &world.manager,
        id
    ));
    let entity = world.entity(id);
    assert_eq!(
        entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT),
        RetailRuntimeValue::Known(DYING_STATE_BIT)
    );
    assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
        .into_iter()
        .all(|slot| entity.actor_tasks.task_in_slot(slot).is_none()));
    assert!(world
        .manager
        .pending_actor_deferred_destroy_ids()
        .contains(&id));
    assert!(
        world.fx.particle_count() > particles,
        "BAF0 scatters class16"
    );
    // The completed corpse takes a second hit and its null-hook contacts.
    let again = world.hit(id, 16, SHOT);
    assert!(
        matches!(again, impact::RollingBoulderImpactOutcome::Applied(_)),
        "{again:?}"
    );
    let surface = resolve_native_actor_surface_contact(&mut world.contact_frame(), id);
    assert!(
        !matches!(surface, NativeActorSurfaceContactOutcome::Blocked { .. }),
        "{surface:?}"
    );
    let static_contact = resolve_insect_static_contact(&mut world.contact_frame(), id);
    assert!(
        !matches!(static_contact, NativeGroundContactOutcome::Blocked { .. }),
        "{static_contact:?}"
    );
}

#[v2k_test_support::retail_test]
fn a_lethal_hit_splits_a_large_boulder_into_two_grounded_small_boulders() {
    let mut world = BoulderWorld::new(27);
    let id = world.first(27);
    world.isolate(id);
    world.activate(id);
    world.manager.entity_mut(id).unwrap().collision.health_raw = RetailRuntimeValue::Known(1);
    let before = world.ids();
    let position = world.entity(id).position_raw();
    let mut rng = world.fx.fork_for_main_base_abort_transaction();
    let outcome = world.hit(id, 16, SHOT);
    assert!(
        matches!(outcome, impact::RollingBoulderImpactOutcome::Applied(_)),
        "{outcome:?}"
    );
    assert!(death::finished_split_authenticates(&world.manager, id));
    let heading = world.entity(id).rotation_heading_pitch_roll_raw()[0] as u16;
    let terrain = world.session.cache.terrain().unwrap();
    let extent = world.session.cache.global_model(648).unwrap().radius as i16;
    let children: Vec<_> = world
        .manager
        .iter_all()
        .filter(|entity| !before.contains(&entity.id))
        .collect();
    assert_eq!(children.len(), 2, "Type27 -> Type3 count2");
    // 11030's three samples, then 440950's one word.
    for _ in 0..4 {
        rng.next_shared_retail_random_u16();
    }
    for child in &children {
        let w: [u16; 9] = std::array::from_fn(|_| rng.next_shared_retail_random_u16());
        rng.next_shared_retail_random_u16(); // AC60's singleton class20 choice
        let x = position[0].wrapping_add(((w[0] >> 6) as i16).wrapping_sub(512));
        let z = position[2].wrapping_add(((w[2] >> 6) as i16).wrapping_sub(512));
        assert_eq!(child.entity_type, 3);
        assert_eq!(
            child.position_raw(),
            [x, terrain.bilinear_height_raw(x, z).wrapping_add(extent), z],
            "D4A0 grounds the launch position"
        );
        assert_eq!(
            child.velocity_raw(),
            [
                ((w[3] >> 5) as i16).wrapping_sub(1024),
                (w[4] >> 6) as i16,
                ((w[5] >> 5) as i16).wrapping_sub(1024),
            ]
        );
        assert_eq!(
            child.rotation_heading_pitch_roll_raw(),
            [
                heading.wrapping_add((w[6] >> 4).wrapping_sub(2048)) as i16,
                (w[7] >> 4).wrapping_sub(2048) as i16,
                (w[8] >> 4).wrapping_sub(2048) as i16,
            ]
        );
        assert_eq!(current_style(child), Ok(RollingBoulderStyle::Rolling));
        assert_eq!(
            child.collision.state_flags_at_0x08.masked(0x50000 | 4),
            RetailRuntimeValue::Known(0x50004),
            "style0 motion bits and the D720 basis"
        );
        assert!(RollingBoulderOwner::adopt(&world.manager, child.id).is_ok());
    }
    assert_eq!(
        world.fx.next_shared_retail_random_u16(),
        rng.next_shared_retail_random_u16(),
        "no other draws"
    );
}

#[v2k_test_support::retail_test]
fn split_children_join_the_scheduler_and_come_to_rest() {
    let mut world = BoulderWorld::new(27);
    let id = world.first(27);
    world.isolate(id);
    world.activate(id);
    world.manager.entity_mut(id).unwrap().collision.health_raw = RetailRuntimeValue::Known(1);
    let before = world.ids();
    world.hit(id, 16, SHOT);
    let children: Vec<_> = world
        .ids()
        .into_iter()
        .filter(|id| !before.contains(id))
        .collect();
    for &child in &children {
        world.activate(child);
    }
    let mut rested = Vec::new();
    for _ in 0..3000 {
        for (&child, frame) in children.iter().zip(world.frame_all(&children)) {
            if rested.contains(&child) {
                continue;
            }
            assert!(
                !matches!(
                    frame.surface,
                    NativeActorSurfaceContactOutcome::Blocked { .. }
                ),
                "{:?}",
                frame.surface
            );
            if let Some(RollingBoulderOutcome::Advanced {
                switched: true,
                style: RollingBoulderStyle::Resting,
                ..
            }) = frame.task
            {
                rested.push(child);
            }
        }
        if rested.len() == children.len() {
            break;
        }
    }
    assert_eq!(rested.len(), 2, "both thrown children land and rest");
}
