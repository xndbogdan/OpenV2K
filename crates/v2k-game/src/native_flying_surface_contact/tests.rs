use super::*;
use crate::{
    common_mover::type9_attitude::Type9BodyBasis,
    entity::{Entity, EntityManager},
    gameplay_notifications::GameplayNotifications,
    intro2_type47_live::world::native_intro2_fixture,
    live_actor_checked_damage::{LiveActorDamageBlock, LiveActorDamageError, LiveActorDamagePhase},
    session::GameSession,
    specialized_actor_task_production::{
        SpecializedActorTaskFamily, SpecializedActorTaskScheduler,
    },
    static_damage::StaticDamageScheduler,
    world_fx::WorldFx,
};

struct Fixture {
    session: GameSession,
    entities: EntityManager,
    tasks: SpecializedActorTaskScheduler,
    fx: WorldFx,
    static_damage: StaticDamageScheduler,
    notifications: GameplayNotifications,
    id: u32,
}

impl Fixture {
    fn new(spawn: usize) -> Option<Self> {
        let (session, mut entities, _) = native_intro2_fixture()?;
        let id = entities
            .iter_all()
            .find(|e| e.authored_spawn_index == Some(spawn))
            .unwrap()
            .id;
        let entity = entities.entity_mut(id).unwrap();
        entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(u32::MAX, 0x0747_8825);
        entity.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(0);
        entity.physical_body_basis_q31 =
            RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(0, 0, 0));
        let mut tasks = SpecializedActorTaskScheduler::default();
        assert_eq!(tasks.adopt_intro2_type13_search_attack(&entities), 1);
        assert_eq!(tasks.adopt_intro2_type10(&entities), 2);
        assert_eq!(tasks.adopt_intro2_type57(&entities), 1);
        Some(Self {
            session,
            entities,
            tasks,
            fx: WorldFx::new(),
            static_damage: StaticDamageScheduler::new(),
            notifications: GameplayNotifications::new(),
            id,
        })
    }

    fn entity(&mut self) -> &mut Entity {
        self.entities.entity_mut(self.id).unwrap()
    }

    fn run(&mut self) -> NativeFlyingSurfaceContactOutcome {
        resolve_native_flying_surface_contact(
            &mut Intro2ContactFrame {
                entities: &mut self.entities,
                resources: &mut self.session.cache,
                world_fx: &mut self.fx,
                static_damage: &mut self.static_damage,
                notifications: &mut self.notifications,
                actor_tasks: &mut self.tasks,
                retail_tick: 25,
            },
            self.id,
        )
    }

    fn terrain(&mut self, height: i8) {
        for cell in &mut self.session.cache.level_terrain_mut().unwrap().cells {
            cell.height = height as u8;
        }
    }

    fn penetration(&self) -> Option<f64> {
        let entity = self.entities.iter_all().find(|e| e.id == self.id).unwrap();
        let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
            panic!()
        };
        self.session
            .cache
            .global_model(entity.model_index.unwrap())
            .unwrap()
            .collide_terrain_raw_oriented(
                self.session.cache.level_terrain().unwrap(),
                entity.position_raw(),
                basis
                    .orientation_world_from_model()
                    .map(|row| row.map(f64::from)),
                &entity.presentation_anim_vars(25),
            )
            .unwrap()
            .map(|hit| hit.penetration_raw)
    }
}

#[v2k_test_support::retail_test]
fn living_flyer_ground_phase_fills_previous_type10_and_type57_ineligible_gap() {
    for spawn in [1, 55, 56] {
        let Some(mut f) = Fixture::new(spawn) else {
            return;
        };
        f.terrain(0);
        f.entity().set_position_raw([623, -50, 1920]);
        f.entity().set_velocity_raw([0, -200, 0]);
        let before = f.entity().position_raw();
        let mut frame = Intro2ContactFrame {
            entities: &mut f.entities,
            resources: &mut f.session.cache,
            world_fx: &mut f.fx,
            static_damage: &mut f.static_damage,
            notifications: &mut f.notifications,
            actor_tasks: &mut f.tasks,
            retail_tick: 25,
        };
        if spawn == 1 {
            assert_eq!(
                crate::intro2_type57::resolve_intro2_type57_tumble_contact(&mut frame, f.id),
                crate::intro2_type57::Intro2Type57ContactOutcome::Ineligible
            );
        } else {
            assert_eq!(
                crate::intro2_type10::contact::resolve_intro2_type10_tumble_contact(
                    &mut frame, f.id
                ),
                crate::intro2_type10::contact::Intro2Type10ContactOutcome::Ineligible
            );
        }
        assert_eq!(f.entity().position_raw(), before);
        let result = f.run();
        assert!(
            matches!(
                result,
                NativeFlyingSurfaceContactOutcome::Applied {
                    solid_contact: true,
                    ..
                }
            ),
            "{result:?}"
        );
        assert!(f.entity().position_raw()[1] > before[1]);
        assert!(f.entity().velocity_raw()[1] > -200);
    }
}

#[v2k_test_support::retail_test]
fn all_native_living_flying_models_resolve_ground_on_slopes_at_three_frame_durations() {
    for spawn in [0, 1, 55, 56] {
        let Some(mut f) = Fixture::new(spawn) else {
            return;
        };
        let mut contacts = 0;
        for slope in [[0, 0], [2, -1], [-2, 2]] {
            f.terrain(0);
            for x in 0..16 {
                for z in 0..16 {
                    f.session.cache.level_terrain_mut().unwrap().cells[x * 256 + z].height =
                        (((x as i32 - 2) * slope[0] + (z as i32 - 7) * slope[1]) as i8) as u8;
                }
            }
            for elapsed_us in [16_667, 40_000, 125_000] {
                for velocity in [-200, -1000, -3000] {
                    let floor = f
                        .session
                        .cache
                        .level_terrain()
                        .unwrap()
                        .bilinear_height_raw(623, 1920);
                    let y = floor
                        .wrapping_sub(30 + (-(velocity as i64) * elapsed_us / 1_000_000) as i16);
                    f.entity().set_position_raw([623, y, 1920]);
                    f.entity().set_velocity_raw([120, velocity, -70]);
                    let initial_penetration =
                        f.penetration().expect("real model intersects terrain");
                    let result = f.run();
                    assert!(
                        matches!(
                            result,
                            NativeFlyingSurfaceContactOutcome::Applied {
                                solid_contact: true,
                                water_entry: false,
                                ..
                            }
                        ),
                        "spawn{spawn} slope{slope:?} dt{elapsed_us} vy{velocity}: {result:?}"
                    );
                    assert!(f.entity().position_raw()[1] > y);
                    assert!(f.entity().velocity_raw()[1] > velocity);
                    assert!(
                        f.penetration()
                            .is_none_or(|remaining| remaining <= initial_penetration * 0.02 + 2.0),
                        "separation must resolve the collision geometry, not just raise its center"
                    );
                    contacts += 1;
                }
            }
        }
        assert_eq!(contacts, 27);
    }
}

#[v2k_test_support::retail_test]
fn living_flyers_keep_underwater_dive_and_resurface_without_sea_clamping() {
    for spawn in [0, 1, 55, 56] {
        let Some(mut f) = Fixture::new(spawn) else {
            return;
        };
        f.terrain(-96);
        let sea = f.session.cache.level_terrain().unwrap().sea_level_raw();
        f.entity().set_position_raw([623, sea + 194, 1920]);
        f.entity().set_velocity_raw([40, -200, -30]);
        let first = f.entity().position_raw();
        let result = f.run();
        assert!(
            matches!(
                result,
                NativeFlyingSurfaceContactOutcome::Applied {
                    solid_contact: false,
                    water_entry: true,
                    ..
                }
            ),
            "spawn{spawn}: {result:?}"
        );
        assert_eq!(f.entity().position_raw(), first);
        assert_eq!(f.entity().velocity_raw(), [40, -200, -30]);
        let count = f.fx.particle_count();
        for y in [sea - 100, sea - 600, sea - 100, sea + 2000] {
            f.entity().set_position_raw([623, y, 1920]);
            let result = f.run();
            assert!(
                matches!(
                    result,
                    NativeFlyingSurfaceContactOutcome::Applied {
                        solid_contact: false,
                        water_entry: false,
                        ..
                    }
                ),
                "spawn{spawn} y{y}: {result:?}"
            );
            assert_eq!(f.entity().position_raw()[1], y);
            assert_eq!(f.entity().velocity_raw()[1], -200);
            assert_eq!(f.fx.particle_count(), count);
        }
        assert_eq!(
            f.entity().collision.state_flags_at_0x08.masked(0x600000),
            RetailRuntimeValue::Known(0x400000)
        );
    }
}

#[v2k_test_support::retail_test]
fn lethal_ground_contact_publishes_native_tumble_and_leaves_tumble_surface_owner() {
    for spawn in [1, 55, 56] {
        let Some(mut f) = Fixture::new(spawn) else {
            return;
        };
        f.terrain(0);
        f.entity().set_position_raw([623, -50, 1920]);
        f.entity().set_velocity_raw([0, -12000, 0]);
        f.entity().collision.health_raw = RetailRuntimeValue::Known(1);
        let result = f.run();
        assert!(
            matches!(
                result,
                NativeFlyingSurfaceContactOutcome::Applied {
                    solid_contact: true,
                    ..
                }
            ),
            "spawn{spawn}: {result:?}"
        );
        assert_eq!(
            f.entity().collision.health_raw,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            f.tasks.family_for(f.id),
            Some(if spawn == 1 {
                SpecializedActorTaskFamily::Intro2Type57Tumble
            } else {
                SpecializedActorTaskFamily::Intro2Type10Tumble
            })
        );
        assert_eq!(f.run(), NativeFlyingSurfaceContactOutcome::Ineligible);
    }
}

#[v2k_test_support::retail_test]
fn type13_lethal_ground_radial_failure_parks_real_class1_prefix_without_replay() {
    let Some(mut f) = Fixture::new(0) else { return };
    f.terrain(0);
    f.entity().set_position_raw([623, -50, 1920]);
    f.entity().set_velocity_raw([0, -12000, 0]);
    f.entity().collision.health_raw = RetailRuntimeValue::Known(1);
    let victim = f
        .entities
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(1))
        .unwrap()
        .id;
    let ids: Vec<_> = f.entities.retail_live_order_ids().collect();
    for id in ids {
        if id != f.id && id != victim {
            f.entities
                .entity_mut(id)
                .unwrap()
                .collision
                .state_flags_at_0x08
                .overwrite(0x8000, 0);
        }
    }
    let source_position = f.entity().position_raw();
    let entity = f.entities.entity_mut(victim).unwrap();
    entity.set_position_raw(source_position);
    entity.collision.health_raw = RetailRuntimeValue::Unresolved;
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(u32::MAX, 0x0747_8825);
    for _ in 0..8 {
        f.fx.advance_frame_pacing(20_000);
    }
    let result = f.run();
    assert!(
        matches!(
            result,
            NativeFlyingSurfaceContactOutcome::Blocked {
                reason: NativeFlyingSurfaceContactBlock::Damage(LiveActorDamageError {
                    phase: LiveActorDamagePhase::Death,
                    reason: LiveActorDamageBlock::Death(NativeFlyingSurfaceDeathBlock::Class1(_)),
                    ..
                }),
                committed_prefix: true,
            }
        ),
        "{result:?}"
    );
    assert_eq!(
        f.entity().collision.health_raw,
        RetailRuntimeValue::Known(0)
    );
    assert!(
        matches!(f.entity().current_behavior_context, RetailRuntimeValue::Known(Some(context))
        if context.active_style().style_address() == 0x4c7150)
    );
    assert!(!f
        .entities
        .pending_actor_deferred_destroy_ids()
        .contains(&f.id));
    assert!(!crate::class49_death::finished_terminal_hit_authenticates(
        &f.entities,
        f.id
    ));
    assert!(f.tasks.has_native_contact_prefix(f.id));
    let position = f.entity().position_raw();
    let velocity = f.entity().velocity_raw();
    let count = f.fx.particle_count();
    assert!(matches!(
        f.run(),
        NativeFlyingSurfaceContactOutcome::Blocked {
            committed_prefix: false,
            ..
        }
    ));
    assert_eq!(f.entity().position_raw(), position);
    assert_eq!(f.entity().velocity_raw(), velocity);
    assert_eq!(f.fx.particle_count(), count);
}

#[v2k_test_support::retail_test]
fn ground_induced_tumble_uses_changed_water_hook_in_the_same_contact_walk() {
    for spawn in [1, 55, 56] {
        let Some(mut f) = Fixture::new(spawn) else {
            return;
        };
        // Isolate the source's self-radial continuation, keeping real native
        // allocations while excluding unrelated actors from this blast scan.
        let ids: Vec<_> = f
            .entities
            .iter_all()
            .map(|e| e.id)
            .filter(|id| *id != f.id)
            .collect();
        for id in ids {
            f.entities
                .entity_mut(id)
                .unwrap()
                .collision
                .state_flags_at_0x08
                .overwrite(0x8000, 0);
        }
        f.terrain(-34);
        for _ in 0..8 {
            f.fx.advance_frame_pacing(20_000);
        }
        f.entity().set_position_raw([623, -1138, 1920]);
        f.entity().set_velocity_raw([0, -12000, 0]);
        f.entity().collision.health_raw = RetailRuntimeValue::Known(1);
        let result = f.run();
        assert!(
            matches!(
                result,
                NativeFlyingSurfaceContactOutcome::Applied {
                    solid_contact: true,
                    water_entry: true,
                    ..
                }
            ),
            "spawn{spawn}: {result:?}"
        );
        assert!(
            f.entities
                .pending_actor_deferred_destroy_ids()
                .contains(&f.id),
            "new Tumble's C750 water hook must select native terminal death synchronously"
        );
        assert!(f
            .fx
            .test_particles_in_virgin_birth_order()
            .iter()
            .any(|particle| particle.source_class == 37
                && particle.source_entity_type_at_birth == Some(if spawn == 1 { 57 } else { 10 })));
        assert!(
            f.tasks.family_for(f.id).is_none(),
            "completed native terminal retires its Tumble owner"
        );
    }
}

#[v2k_test_support::retail_test]
fn native_flying_mover_iterations_keep_solid_separation_across_runtime_rng_offsets() {
    use crate::{
        intro2_contacts::{resolve_intro2_contacts, Intro2ContactReport},
        specialized_actor_task_production::SpecializedActorTaskProductionFrame,
    };

    // Constructor/component custody remains the real fixture's allocation.
    // Runtime RNG offsets vary the actual Search/mover visits; this controlled
    // flat-ground stress is not an oracle for a particular cinematic route.
    for spawn in [0, 1, 44, 46, 55, 56] {
        for random_offset in [0, 17, 333] {
            for elapsed_us in [16_667, 40_000, 125_000] {
                let Some(mut f) = Fixture::new(spawn) else {
                    return;
                };
                f.tasks.adopt_intro2_flyers(&f.entities);
                f.terrain(0);
                for _ in 0..random_offset {
                    f.fx.next_shared_retail_random_u16();
                }
                // Native Intro2's dormant 12DA0 visits clear allocator-owned
                // B2 before the first enabled callback. Exercise that actual
                // write rather than manufacturing a callback mass fixture.
                f.entity().collision.state_flags_at_0x08.overwrite(
                    crate::entity_scheduler::COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT,
                    0,
                );
                let warmup = f.tasks.tick(
                    &mut f.entities,
                    SpecializedActorTaskProductionFrame {
                        world: crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                        hive_components: None,
                        resources: &mut f.session.cache,
                        world_fx: &mut f.fx,
                        static_damage: &mut f.static_damage,
                        elapsed_micros: elapsed_us,
                        global_elapsed_micros: elapsed_us,
                        retail_tick: 24,
                        notification_phase:
                            crate::gameplay_notifications::GameplayNotificationPhase::NonGameplay,
                        main_base_abort_active: false,
                    },
                    &mut f.notifications,
                );
                assert_eq!(warmup.block, None);
                assert_eq!(
                    f.entity().collision.animation_offset_at_0xb2,
                    RetailRuntimeValue::Known(0)
                );
                f.entity().collision.state_flags_at_0x08.overwrite(
                    crate::entity_scheduler::COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT,
                    crate::entity_scheduler::COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT,
                );
                f.entity()
                    .set_motion_raw([623, -80, 1920], [240, -600, -120]);
                let mut solid_visits = 0;
                let mut moved_visits = 0;
                for iteration in 0..48u32 {
                    let tick = 25 + iteration;
                    if iteration % 8 == 0 {
                        // A bounded downward impulse repeatedly exercises the
                        // physical response after the real family mover; only
                        // velocity is changed after the initial placement.
                        let mut velocity = f.entity().velocity_raw();
                        velocity[1] = -600;
                        f.entity().set_velocity_raw(velocity);
                    }
                    let before_mover = f.entity().position_raw();
                    let pass = f.tasks.tick(
                        &mut f.entities,
                        SpecializedActorTaskProductionFrame {
                            world: crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                            hive_components: None,
                            resources: &mut f.session.cache,
                            world_fx: &mut f.fx,
                            static_damage: &mut f.static_damage,
                            elapsed_micros: elapsed_us,
                            global_elapsed_micros: elapsed_us,
                            retail_tick: tick,
                            notification_phase: crate::gameplay_notifications::GameplayNotificationPhase::NonGameplay,
                            main_base_abort_active: false,
                        },
                        &mut f.notifications,
                    );
                    assert_eq!(
                        pass.block, None,
                        "spawn{spawn} rng{random_offset} dt{elapsed_us} tick{tick}"
                    );
                    moved_visits += usize::from(f.entity().position_raw() != before_mover);
                    let reports = resolve_intro2_contacts(Intro2ContactFrame {
                        entities: &mut f.entities,
                        resources: &mut f.session.cache,
                        world_fx: &mut f.fx,
                        static_damage: &mut f.static_damage,
                        notifications: &mut f.notifications,
                        retail_tick: tick,
                        actor_tasks: &mut f.tasks,
                    });
                    let surface = reports
                        .iter()
                        .find_map(|report| match report {
                            Intro2ContactReport::NativeFlyingSurface {
                                entity_id, result, ..
                            }
                            | Intro2ContactReport::Flyer { entity_id, result }
                                if *entity_id == f.id =>
                            {
                                Some(result)
                            }
                            _ => None,
                        })
                        .expect("late production dispatcher visits every admitted flying family");
                    let NativeFlyingSurfaceContactOutcome::Applied {
                        solid_contact,
                        water_entry,
                        ..
                    } = surface
                    else {
                        panic!("spawn{spawn} rng{random_offset} dt{elapsed_us} tick{tick}: {surface:?}; mover={:?}", pass.outcomes);
                    };
                    solid_visits += usize::from(*solid_contact);
                    assert!(
                        !water_entry,
                        "dry-ground control cannot become a water clamp"
                    );
                    let actor = f.entities.iter_all().find(|e| e.id == f.id).unwrap();
                    let RetailRuntimeValue::Known(basis) = actor.physical_body_basis_q31() else {
                        panic!()
                    };
                    let residual = f
                        .session
                        .cache
                        .global_model(actor.model_index.unwrap())
                        .unwrap()
                        .collide_terrain_raw_oriented(
                            f.session.cache.level_terrain().unwrap(),
                            actor.position_raw(),
                            basis
                                .orientation_world_from_model()
                                .map(|row| row.map(f64::from)),
                            &actor.presentation_anim_vars(tick),
                        )
                        .unwrap();
                    assert!(
                        residual.is_none_or(|hit| hit.penetration_raw <= 3.0),
                        "spawn{spawn} rng{random_offset} dt{elapsed_us} tick{tick}: {residual:?}"
                    );
                }
                assert!(
                    solid_visits > 0,
                    "each native model must reach solid contact"
                );
                assert!(
                    moved_visits > 0,
                    "actual family mover must advance, not just a repeatedly queried fixture"
                );
                assert!(!f.tasks.has_native_contact_prefix(f.id));
            }
        }
    }
}

#[v2k_test_support::retail_test]
fn type13_restricted_world_visit_retains_completed_late_contact_custody() {
    use crate::specialized_actor_task_production::SpecializedActorTaskProductionFrame;
    let Some(mut f) = Fixture::new(0) else { return };
    f.terrain(0);
    f.entity()
        .set_motion_raw([623, -80, 1920], [120, -200, -70]);
    f.entity().collision.state_flags_at_0x08.overwrite(
        crate::entity_scheduler::SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
        0,
    );
    // Both gates reach Continue without a random wait. The actual task callback
    // still uses Restricted mode because the authored state bit is clear.
    f.entity().collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(250_000);
    f.entity()
        .collision
        .callback_scheduler_accumulator_us_at_0x6c = RetailRuntimeValue::Known(125_000);
    let pass = f.tasks.tick(
        &mut f.entities,
        SpecializedActorTaskProductionFrame {
            world: crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
            hive_components: None,
            resources: &mut f.session.cache,
            world_fx: &mut f.fx,
            static_damage: &mut f.static_damage,
            elapsed_micros: 16_667,
            global_elapsed_micros: 16_667,
            retail_tick: 25,
            notification_phase:
                crate::gameplay_notifications::GameplayNotificationPhase::NonGameplay,
            main_base_abort_active: false,
        },
        &mut f.notifications,
    );
    assert_eq!(pass.block, None);
    assert!(
        !f.tasks.intro2_type13_has_pending_prefix(f.id),
        "{:?}",
        pass.outcomes
    );
    assert!(
        f.tasks.intro2_type13_completed_owner(&f.entities, f.id),
        "a fresh adopter's Normal mode must not reject a completed Restricted visit: {:?}",
        pass.outcomes
    );
    let result = f.run();
    assert!(
        matches!(
            result,
            NativeFlyingSurfaceContactOutcome::Applied {
                solid_contact: true,
                water_entry: false,
                ..
            }
        ),
        "{result:?}"
    );
    assert!(f.penetration().is_none_or(|remaining| remaining <= 3.0));
}

#[v2k_test_support::retail_test]
fn native_intro2_surface_birth_uses_real_clock_for_all_bodies_and_keeps_legacy_clock_boundary() {
    use crate::entity::EntityConstructionResources;
    use crate::entity_collision_state::EntityTypeRuntimeMetadata;
    let Some((session, captured, _)) = native_intro2_fixture() else {
        return;
    };
    let metadata = session
        .cache
        .global_entity_model_table()
        .iter()
        .enumerate()
        .map(|(id, slots)| {
            session
                .cache
                .global_entity_type(id)
                .map(EntityTypeRuntimeMetadata::from_section12)
                .unwrap_or(EntityTypeRuntimeMetadata {
                    model_slots: *slots,
                    ..Default::default()
                })
        })
        .collect::<Vec<_>>();
    let extent = |id| session.cache.global_model(id).map(|model| model.radius);
    let resources = EntityConstructionResources {
        terrain: session.cache.terrain(),
        terrain_objects: session.cache.terrain_objects(),
        model_extent_raw: Some(&extent),
    };
    let level = session.cache.level_desc().unwrap();
    let terrain = session.cache.level_terrain().unwrap();
    let waves = level.raw_u32(0x84).unwrap() != 0;
    for constructor_tick in [0, 100, 4793] {
        let native = EntityManager::from_native_intro2_frontend(
            level,
            &metadata,
            resources,
            constructor_tick,
            &mut WorldFx::new(),
        )
        .unwrap();
        for spawn in &level.entities {
            let expected = crate::entity_initializer::constructor_surface_bits_at_tick(
                spawn.position_raw(),
                terrain,
                constructor_tick,
                waves,
            )
            .unwrap();
            let actor = native
                .iter_all()
                .find(|entity| entity.authored_spawn_index == Some(spawn.index))
                .unwrap();
            assert_eq!(
                actor.collision.state_flags_at_0x08.masked(0x600000),
                RetailRuntimeValue::Known(expected),
                "tick{constructor_tick} spawn{} type{} uses pre-component authored Y",
                spawn.index,
                spawn.entity_type
            );
        }
    }
    for spawn in level
        .entities
        .iter()
        .filter(|spawn| matches!(spawn.entity_type, 13 | 10 | 57 | 15 | 87))
    {
        let expected = crate::entity_initializer::constructor_surface_bits_without_wave(
            spawn.position_raw(),
            terrain,
        );
        let actor = captured
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(spawn.index))
            .unwrap();
        assert_eq!(
            actor.collision.state_flags_at_0x08.masked(0x600000),
            expected.map_or(RetailRuntimeValue::Unresolved, RetailRuntimeValue::Known)
        );
    }
    let legacy_beetle = captured
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(10))
        .unwrap();
    assert_eq!(
        legacy_beetle.collision.state_flags_at_0x08.masked(0x600000),
        RetailRuntimeValue::Unresolved,
        "the legacy captured Type26 allocation has no constructor clock receipt"
    );
    let generic = EntityManager::from_level_with_type_metadata(level, &metadata, resources);
    let generic_type13 = generic
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(0))
        .unwrap();
    assert_eq!(
        generic_type13
            .collision
            .state_flags_at_0x08
            .masked(0x600000),
        RetailRuntimeValue::Unresolved,
        "generic level construction does not inherit native frontend clock custody"
    );
}

#[v2k_test_support::retail_test]
fn type13_lethal_ground_finishes_shared_class1_before_water_and_late_static_suffix() {
    let Some(mut f) = Fixture::new(0) else { return };
    f.terrain(0);
    let ids: Vec<_> = f.entities.retail_live_order_ids().collect();
    for id in ids {
        if id != f.id {
            f.entities
                .entity_mut(id)
                .unwrap()
                .collision
                .state_flags_at_0x08
                .overwrite(0x8000, 0);
        }
    }
    f.entity().set_position_raw([623, -50, 1920]);
    f.entity().set_velocity_raw([0, -12000, 0]);
    f.entity().collision.health_raw = RetailRuntimeValue::Known(1);
    for _ in 0..8 {
        f.fx.advance_frame_pacing(20_000);
    }
    let result = f.run();
    assert!(
        matches!(
            result,
            NativeFlyingSurfaceContactOutcome::Applied {
                solid_contact: true,
                water_entry: false,
                ..
            }
        ),
        "{result:?}"
    );
    assert!(crate::class49_death::finished_terminal_hit_authenticates(
        &f.entities,
        f.id
    ));
    assert!(f.tasks.family_for(f.id).is_none());
    assert_eq!(f.entities.pending_actor_deferred_destroy_ids(), &[f.id]);
    assert!(!f.entities.iter_all().any(|entity| entity.entity_type == 60));
    assert_eq!(
        f.fx.test_particles_in_virgin_birth_order()
            .iter()
            .filter(|particle| particle.source_class == 37
                && particle.source_entity_type_at_birth == Some(13))
            .count(),
        10
    );
    let mut frame = Intro2ContactFrame {
        entities: &mut f.entities,
        resources: &mut f.session.cache,
        world_fx: &mut f.fx,
        static_damage: &mut f.static_damage,
        notifications: &mut f.notifications,
        actor_tasks: &mut f.tasks,
        retail_tick: 25,
    };
    let static_suffix =
        crate::native_ground_actor::contact::resolve_flying_static_contact_continuation(
            &mut frame,
            f.id,
            crate::intro2_type13_live::INTRO2_TYPE13_MODEL_ID,
        );
    assert!(matches!(static_suffix, crate::native_ground_actor::contact::NativeGroundContactOutcome::Miss),
        "completed Class1 retains native null static hook in the same admitted11AD0 walk: {static_suffix:?}");
}
