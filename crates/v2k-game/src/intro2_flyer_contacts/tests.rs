use super::*;
use crate::{
    common_mover::type9_attitude::Type9BodyBasis,
    entity::EntityManager,
    gameplay_notifications::GameplayNotifications,
    intro2_type47_live::world::native_intro2_fixture,
    session::GameSession,
    specialized_actor_task_production::{
        SpecializedActorTaskFamily, SpecializedActorTaskScheduler,
    },
    static_damage::StaticDamageScheduler,
    world_fx::{WorldFx, WorldParticle},
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

#[v2k_test_support::retail_test]
fn native_flyer_and_dragon_births_retain_source_dry_surface_comparison() {
    let Some((session, manager, _)) = native_intro2_fixture() else {
        return;
    };
    let terrain = session.cache.terrain().unwrap();
    for (spawn, entity_type, expected_floor) in
        [(44, 15, -640), (46, 87, 96), (55, 10, -640), (56, 10, -320)]
    {
        let entity = manager
            .iter_all()
            .find(|e| e.authored_spawn_index == Some(spawn))
            .unwrap();
        assert_eq!(entity.entity_type, entity_type);
        let [x, y, z] = entity.position_raw();
        let cell = terrain
            .cell(usize::from((x as u16) >> 8), usize::from((z as u16) >> 8))
            .unwrap();
        assert_eq!(i16::from(cell.height as i8) * 32, expected_floor);
        assert!(expected_floor >= terrain.sea_level_raw());
        assert!(y > terrain.sea_level_raw());
        assert_eq!(
            entity.collision.state_flags_at_0x08.masked(0x600000),
            RetailRuntimeValue::Known(0x400000)
        );
        // Both component and behavior publication must preserve the earlier
        // constructor comparison; none of these four native births is inert.
        assert!(entity
            .actor_task_state(crate::actor_task_owner::ActorTaskSlot::Primary)
            .is_some());
    }
    // The new binder does not silently grant a surface phase to other native
    // cohorts: the Type94 water actor keeps its existing constructor boundary.
    let water_actor = manager
        .iter_all()
        .find(|e| e.authored_spawn_index == Some(43))
        .unwrap();
    assert_eq!(
        water_actor.collision.state_flags_at_0x08.masked(0x600000),
        RetailRuntimeValue::Unresolved
    );
}

impl Fixture {
    fn new() -> Option<Self> {
        Self::for_spawn(46)
    }

    fn for_spawn(spawn: usize) -> Option<Self> {
        let (session, mut entities, _) = native_intro2_fixture()?;
        let id = entities
            .iter_all()
            .find(|e| e.authored_spawn_index == Some(spawn))
            .unwrap()
            .id;
        let entity = entities.entity_mut(id).unwrap();
        assert_eq!(entity.entity_type, if spawn == 44 { 15 } else { 87 });
        assert_eq!(
            session
                .cache
                .global_model(270)
                .unwrap()
                .collision_radius_raw,
            198
        );
        entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(u32::MAX, 0x0747_8825);
        entity.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(0);
        // July22 own bee allocation, lifecycle line52008. These are retained
        // body axes, not a fresh Euler approximation to the recorded matrix.
        entity.physical_body_basis_q31 = RetailRuntimeValue::Known(Type9BodyBasis {
            lateral: [-1787953152, 43712512, -1187971072],
            up: [942800896, 1358364672, -1369047040],
            forward: [723582976, -1662058496, -1150287872],
        });
        entity.set_position_raw([623, -830, 1920]);
        entity.set_velocity_raw([760, -875, -766]);
        let mut tasks = SpecializedActorTaskScheduler::default();
        assert_eq!(tasks.adopt_intro2_flyers(&entities), 2);
        let mut fx = WorldFx::new();
        for _ in 0..8 {
            fx.advance_frame_pacing(20_000);
        }
        Some(Self {
            session,
            entities,
            tasks,
            fx,
            static_damage: StaticDamageScheduler::new(),
            notifications: GameplayNotifications::new(),
            id,
        })
    }

    fn contact_frame(&mut self, tick: u32) -> Intro2ContactFrame<'_> {
        Intro2ContactFrame {
            entities: &mut self.entities,
            resources: &mut self.session.cache,
            actor_tasks: &mut self.tasks,
            world_fx: &mut self.fx,
            static_damage: &mut self.static_damage,
            notifications: &mut self.notifications,
            retail_tick: tick,
        }
    }

    fn run(&mut self, tick: u32) -> Intro2FlyerContactOutcome {
        let id = self.id;
        resolve_intro2_flyer_surface_contact(&mut self.contact_frame(tick), id)
    }

    fn run_complete(&mut self, tick: u32) -> NativeFlyerContactOutcome {
        let id = self.id;
        resolve_native_flyer_contacts(&mut self.contact_frame(tick), id)
    }

    fn entity(&mut self) -> &mut Entity {
        self.entities.entity_mut(self.id).unwrap()
    }

    fn particles(&self) -> Vec<WorldParticle> {
        let mut fx = self.fx.fork_for_main_base_abort_transaction();
        let mut result = Vec::new();
        fx.prepare_presentation([640, 480], 0x3000, |particle| {
            result.push(*particle);
            v2k_render::ParticleCenterProjection {
                screen: [320, 240],
                depth_raw: 256,
                clip: 0,
            }
        });
        result
    }
}

#[v2k_test_support::retail_test]
fn native_bee_captured_soft_entry_allocates_four_owned_class13_then_exits_without_replay() {
    let Some(mut f) = Fixture::new() else { return };
    let mut rng = f.fx.fork_for_main_base_abort_transaction();
    assert!(matches!(
        f.run(1774),
        Intro2FlyerContactOutcome::Applied {
            solid_contact: false,
            collision_damage_raw: 0,
            water_entry: true,
            response: Some(WholeBodySurfaceResponse::SurfaceBurst {
                response_selector: 6
            }),
            ring: None,
        }
    ));
    assert_eq!(f.entity().velocity_raw(), [760, -875, -766]);
    assert_eq!(bits(f.entity(), 0x600000).unwrap(), 0);
    let particles = f.particles();
    assert_eq!(particles.len(), 4);
    for particle in particles {
        assert_eq!(particle.source_class, 13);
        assert_eq!(particle.owner_id, Some(f.id));
        assert_eq!(particle.source_entity_type_at_birth, Some(87));
        assert_eq!(
            particle.position.map(|v| (v * 256.0) as i16),
            [623, -1024, 1920]
        );
        assert_eq!(particle.step_start_water_state, particle.water_state);
    }
    assert!(
        f.fx.take_positional_sounds().is_empty(),
        "own +88 is zero; no borrowed Type26 cue"
    );
    // Same-frame repeat, fully submerged, intersecting upward, then above:
    // only the first recorded edge may allocate a response.
    assert!(matches!(
        f.run(1774),
        Intro2FlyerContactOutcome::Applied {
            water_entry: false,
            ..
        }
    ));
    for (tick, position, expected) in [
        (1806, [1025, -1107, 1479], 0x200000),
        (1834, [1451, -852, 1209], 0),
        (1848, [1686, -451, 1097], 0x400000),
    ] {
        f.entity().set_position_raw(position);
        assert!(matches!(
            f.run(tick),
            Intro2FlyerContactOutcome::Applied {
                water_entry: false,
                ..
            }
        ));
        assert_eq!(bits(f.entity(), 0x600000).unwrap(), expected);
    }
    assert_eq!(f.fx.particle_count(), 4);
    assert_eq!(
        f.fx.next_shared_retail_random_u16(),
        rng.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn native_bee_surface_equalities_intersect_and_dry_cells_preserve_crossing_state() {
    let Some(mut f) = Fixture::new() else { return };
    let terrain = f.session.cache.level_terrain().unwrap();
    let wave = v2k_formats::terrain::wave_surface_raw(
        623,
        1920,
        1774,
        terrain.sea_level_raw(),
        i16::from(terrain.cell(2, 7).unwrap().height as i8) * 32,
    );
    for (y, expected) in [
        (wave + 199, 0x400000),
        (wave + 198, 0),
        (wave - 198, 0),
        (wave - 199, 0x200000),
    ] {
        f.entity()
            .collision
            .state_flags_at_0x08
            .overwrite(0x600000, 0);
        f.entity().set_position_raw([623, y, 1920]);
        assert!(matches!(
            f.run(1774),
            Intro2FlyerContactOutcome::Applied {
                water_entry: false,
                ..
            }
        ));
        assert_eq!(bits(f.entity(), 0x600000).unwrap(), expected);
    }
    for cell in &mut f.session.cache.level_terrain_mut().unwrap().cells {
        cell.height = 0;
    }
    f.entity().set_position_raw([623, 2000, 1920]);
    f.entity()
        .collision
        .state_flags_at_0x08
        .overwrite(0x600000, 0x200000);
    assert!(matches!(
        f.run(1774),
        Intro2FlyerContactOutcome::Applied {
            solid_contact: false,
            water_entry: false,
            ..
        }
    ));
    assert_eq!(bits(f.entity(), 0x600000).unwrap(), 0x200000);
    assert_eq!(f.fx.particle_count(), 0);
}

#[v2k_test_support::retail_test]
fn native_bee_hard_entry_uses_actual_ring_constructor_and_immediate_scheduler_custody() {
    for (velocity, severe, model) in [(-1001, false, 132), (-1750, false, 132), (-1751, true, 130)]
    {
        let Some(mut f) = Fixture::new() else { return };
        f.entity().set_velocity_raw([760, velocity, -766]);
        let mut rng = f.fx.fork_for_main_base_abort_transaction();
        rng.next_shared_retail_random_u16(); // singleton class48 selector
        let Intro2FlyerContactOutcome::Applied {
            water_entry: true,
            response: Some(WholeBodySurfaceResponse::HardImpact { severe: actual }),
            ring: Some(Type60ConstructionOutcome::ActorLinked(receipt)),
            ..
        } = f.run(1774)
        else {
            panic!("hard entry must construct a live ring")
        };
        assert_eq!(actual, severe);
        let ring_id = receipt.actor().entity_id;
        assert_eq!(
            f.tasks.family_for(ring_id),
            Some(SpecializedActorTaskFamily::Type60ExplodingRing)
        );
        let ring = f.entities.entity_mut(ring_id).unwrap();
        assert_eq!(ring.entity_type, 60);
        assert_eq!(ring.model_in_slot(0), Some(model));
        assert_eq!(ring.position_raw(), [623, -1024, 1920]);
        assert_eq!(f.entity().velocity_raw(), [760, velocity >> 1, -766]);
        // The source request is queued immediately; the audio phase consumes
        // it after the world event queue, independently of actor admission.
        assert!(f.fx.take_positional_sounds().is_empty());
        f.fx.process_pending();
        let sounds = f.fx.take_positional_sounds();
        assert_eq!(sounds.len(), 1);
        assert_eq!(sounds[0].sound_id, 17);
        assert_eq!(
            sounds[0].position.map(|v| (v * 256.0) as i16),
            [623, -830, 1920]
        );
        assert!(matches!(
            f.run(1774),
            Intro2FlyerContactOutcome::Applied {
                water_entry: false,
                ..
            }
        ));
        f.fx.process_pending();
        assert!(f.fx.take_positional_sounds().is_empty());
        assert_eq!(
            f.fx.next_shared_retail_random_u16(),
            rng.next_shared_retail_random_u16()
        );
    }
}

#[v2k_test_support::retail_test]
fn native_bee_pending_contact_is_rejected_but_source_ineligible_exits_before_custody() {
    let Some(mut f) = Fixture::new() else { return };
    f.tasks.park_intro2_flyer_contact_prefix(f.id);
    let before = f.entity().collision.clone();
    let mut rng = f.fx.fork_for_main_base_abort_transaction();
    assert_eq!(
        f.run(1774),
        Intro2FlyerContactOutcome::Blocked {
            reason: Intro2FlyerContactBlock::Runtime("current completed contact owner"),
            committed_prefix: false,
        }
    );
    assert_eq!(f.entity().collision, before);
    f.entity().collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(1);
    f.entity().physical_body_basis_q31 = RetailRuntimeValue::Unresolved;
    assert_eq!(f.run(1774), Intro2FlyerContactOutcome::Ineligible);
    assert_eq!(f.fx.particle_count(), 0);
    assert_eq!(
        f.fx.next_shared_retail_random_u16(),
        rng.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn complete_flyer_phase_preserves_surface_and_static_blocking_boundaries() {
    let Some(mut f) = Fixture::new() else {
        return;
    };
    f.tasks = SpecializedActorTaskScheduler::default();
    let mut rng = f.fx.fork_for_main_base_abort_transaction();
    let blocked = f.run_complete(1774);
    assert!(matches!(
        blocked.surface,
        Intro2FlyerContactOutcome::Blocked { .. }
    ));
    assert_eq!(
        blocked.static_contact, None,
        "surface failure leaves static unvisited"
    );
    assert!(blocked.blocks_later_contacts());
    assert_eq!(f.fx.particle_count(), 0);
    assert_eq!(
        f.fx.next_shared_retail_random_u16(),
        rng.next_shared_retail_random_u16()
    );

    // Source+70 excludes both phases before they require the missing task owner.
    f.entity().collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(1);
    let ineligible = f.run_complete(1774);
    assert_eq!(ineligible.surface, Intro2FlyerContactOutcome::Ineligible);
    assert_eq!(
        ineligible.static_contact,
        Some(NativeGroundContactOutcome::Ineligible)
    );
    assert!(!ineligible.blocks_later_contacts());

    // Disabling terrain/water does not disable the separate static scan.
    // Its required physical basis is unresolved, so that reached phase blocks
    // pairs. Task custody is checked after a real static hit, not on a miss.
    f.entity().collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
    f.entity()
        .collision
        .state_flags_at_0x08
        .overwrite(0x10000, 0);
    f.entity().physical_body_basis_q31 = RetailRuntimeValue::Unresolved;
    let static_blocked = f.run_complete(1774);
    assert_eq!(
        static_blocked.surface,
        Intro2FlyerContactOutcome::Ineligible
    );
    assert!(matches!(
        static_blocked.static_contact,
        Some(NativeGroundContactOutcome::Blocked { .. })
    ));
    assert!(static_blocked.blocks_later_contacts());
}

#[v2k_test_support::retail_test]
fn complete_flyer_phase_does_not_admit_other_native_flying_families() {
    let Some(mut f) = Fixture::new() else {
        return;
    };
    let foreign_id = f
        .entities
        .iter_all()
        .find(|entity| entity.entity_type == 13)
        .unwrap()
        .id;
    for id in [foreign_id, u32::MAX] {
        let result = resolve_native_flyer_contacts(&mut f.contact_frame(1774), id);
        assert_eq!(result.surface, Intro2FlyerContactOutcome::Ineligible);
        assert_eq!(
            result.static_contact,
            Some(NativeGroundContactOutcome::Ineligible)
        );
        assert!(!result.blocks_later_contacts());
    }
}

#[v2k_test_support::retail_test]
fn native_flyer_solid_response_precedes_water_and_lethal_damage_publishes_quiet_death() {
    for spawn in [44, 46] {
        for lethal in [false, true] {
            let Some(mut f) = Fixture::for_spawn(spawn) else {
                return;
            };
            for cell in &mut f.session.cache.level_terrain_mut().unwrap().cells {
                cell.height = 0;
            }
            f.entity().set_position_raw([623, -50, 1920]);
            f.entity()
                .set_velocity_raw([0, if lethal { -12000 } else { -200 }, 0]);
            f.entity().collision.health_raw =
                RetailRuntimeValue::Known(if lethal { 1 } else { 4000 });
            let result = f.run(1774);
            assert!(
                matches!(result, Intro2FlyerContactOutcome::Applied {
                solid_contact: true, water_entry: false, collision_damage_raw: amount, ..
            } if amount > 0),
                "{result:?}"
            );
            if lethal {
                assert_eq!(
                    f.entity().collision.health_raw,
                    RetailRuntimeValue::Known(0)
                );
                assert_ne!(
                    f.entity().velocity_raw()[1],
                    500,
                    "quiet death creates no class12 launch"
                );
                assert_eq!(
                    f.entity().collision.constructor_sound_attachment_id_at_0x8c,
                    RetailRuntimeValue::Known(None)
                );
                assert_eq!(f.tasks.family_for(f.id), None);
                assert!(
                    crate::native_flying_surface_contact::native_flyer_quiet_death_authenticates(
                        &f.entities,
                        f.id
                    )
                );
                assert!(
                    matches!(f.run(1774), Intro2FlyerContactOutcome::Applied { .. }),
                    "quiet terminal retains its handle until the deferred splice"
                );
            } else {
                assert_eq!(
                    f.entity().collision.health_raw,
                    RetailRuntimeValue::Known(4000),
                    "canonical channel1 threshold filters this response to zero"
                );
            }
            assert_eq!(bits(f.entity(), 0x800000).unwrap(), 0x800000);
            assert!(f.entity().position_raw()[1] > -50);
        }
    }
}

#[v2k_test_support::retail_test]
fn both_native_flyers_resolve_downward_ground_contacts_on_flat_and_sloped_terrain() {
    // These are controlled stress poses, not captured bee trajectories. The
    // real allocations, active collision models and completed task owners
    // exercise the late production contact phase with each family's data.
    for spawn in [44, 46] {
        let Some(mut f) = Fixture::for_spawn(spawn) else {
            return;
        };
        let mut contacts = 0;
        for slope in [[0, 0], [2, -1], [-2, 2]] {
            for cell in &mut f.session.cache.level_terrain_mut().unwrap().cells {
                cell.height = 0;
            }
            for x in 0..16 {
                for z in 0..16 {
                    let height = ((x as i32 - 2) * slope[0] + (z as i32 - 7) * slope[1]) as i8;
                    f.session.cache.level_terrain_mut().unwrap().cells[x * 256 + z].height =
                        height as u8;
                }
            }
            // Downward displacement spans the production frame-duration
            // range, without clamping to sea height or changing dive policy.
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
                    f.entity().collision.health_raw = RetailRuntimeValue::Known(4000);
                    let result = f.run(1774);
                    assert!(
                        matches!(
                            result,
                            Intro2FlyerContactOutcome::Applied {
                                solid_contact: true,
                                water_entry: false,
                                ..
                            }
                        ),
                        "spawn{spawn} slope{slope:?} dt{elapsed_us} vy{velocity}: {result:?}"
                    );
                    assert!(
                        f.entity().position_raw()[1] > y,
                        "source solid response must correct the actual model penetration"
                    );
                    assert!(f.entity().velocity_raw()[1] > velocity);
                    contacts += 1;
                }
            }
        }
        assert_eq!(contacts, 27);
    }
}

#[v2k_test_support::retail_test]
fn native_flyer_quiet_death_releases_buzz_clears_tasks_and_defers_without_g_rng_or_corpse() {
    use crate::native_flying_surface_contact::{
        native_flyer_quiet_death_authenticates, publish_native_flying_standard_death,
        register_native_flying_death, NativeFlyingSurfaceDeathPublication,
    };
    for spawn in [44, 46] {
        let Some(mut f) = Fixture::for_spawn(spawn) else {
            return;
        };
        let entity_type = f.entity().entity_type;
        let metadata = f
            .entities
            .type_runtime_metadata(entity_type)
            .unwrap()
            .clone();
        assert_eq!(
            metadata
                .initializer
                .as_ref()
                .unwrap()
                .alternate_behavior_class_ref,
            2
        );
        assert_eq!(metadata.mass_raw, 100);
        assert_eq!(
            metadata.common_mover_topology,
            RetailRuntimeValue::Known(crate::intro2_flyers_live::FLYER_COMMON_MOVER_TOPOLOGY)
        );
        assert_eq!(
            metadata.constructor_sound_attachment_id,
            RetailRuntimeValue::Known(Some(11))
        );
        f.entity().set_motion_raw([100, 10000, 200], [50, -80, 100]);
        f.entity().collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(9);
        let before_g = f.entity().sub_g_06070_runtime;
        let mut control = f.fx.fork_for_main_base_abort_transaction();
        let result =
            publish_native_flying_standard_death(&mut f.entities, f.id, &mut f.fx).unwrap();
        assert!(result.returned_nonzero);
        let Some(publication @ NativeFlyingSurfaceDeathPublication::QuietDeath(_)) =
            result.publication
        else {
            panic!()
        };
        register_native_flying_death(&mut f.tasks, publication);
        assert_eq!(f.tasks.family_for(f.id), None);
        let entity = f.entity();
        assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
        assert_eq!(entity.velocity_raw(), [50, -80, 100]);
        assert_eq!(entity.sub_g_06070_runtime, before_g);
        assert_eq!(
            entity.sub_a_propulsion_runtime,
            RetailRuntimeValue::Known(None)
        );
        assert_eq!(
            entity.sub_h_external_frame_runtime,
            RetailRuntimeValue::Known(None)
        );
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(9)
        );
        assert_eq!(
            entity.collision.constructor_sound_attachment_id_at_0x8c,
            RetailRuntimeValue::Known(None)
        );
        for slot in crate::actor_task_owner::ActorTaskSlot::IN_RETAIL_TICK_ORDER {
            assert!(entity.actor_tasks.task_in_slot(slot).is_none());
        }
        assert!(native_flyer_quiet_death_authenticates(&f.entities, f.id));
        assert_eq!(f.entities.pending_actor_deferred_destroy_ids(), [f.id]);
        assert!(f.entities.iter_all().any(|entity| entity.id == f.id));
        f.fx.process_pending();
        let sounds = f.fx.take_positional_sounds();
        assert_eq!(
            sounds
                .iter()
                .map(|sound| sound.sound_id)
                .collect::<Vec<_>>(),
            match metadata.death_sound_id {
                RetailRuntimeValue::Known(sound) =>
                    sound.into_iter().map(usize::from).collect::<Vec<_>>(),
                _ => panic!(),
            }
        );
        assert_eq!(
            f.fx.next_shared_retail_random_u16(),
            control.next_shared_retail_random_u16()
        );
        assert!(
            publish_native_flying_standard_death(&mut f.entities, f.id, &mut f.fx)
                .unwrap()
                .publication
                .is_none()
        );
        f.entities.cleanup_pending_actor_deferred_destroys();
        assert!(!f.entities.iter_all().any(|entity| entity.id == f.id));
    }
}

#[v2k_test_support::retail_test]
fn native_wasp_hive_ground_burial_pose_runs_shared_solid_response() {
    // Production before-case: native FrontendTicks333 /16667us, tick1060.
    // This route is RNG-dependent; it is not a retail trajectory oracle.
    let Some(mut f) = Fixture::for_spawn(44) else {
        return;
    };
    f.entity().set_position_raw([-14449, -1823, 31443]);
    f.entity().set_velocity_raw([501, -165, -532]);
    f.entity().physical_body_basis_q31 = RetailRuntimeValue::Known(Type9BodyBasis {
        lateral: [-1618018304, 244121600, -1389887488],
        up: [1297743872, 1087373312, -1319632896],
        forward: [553779200, -1835139072, -967049216],
    });
    let model = f.session.cache.global_model(276).unwrap();
    let RetailRuntimeValue::Known(basis) = f
        .entities
        .iter_all()
        .find(|e| e.id == f.id)
        .unwrap()
        .physical_body_basis_q31()
    else {
        unreachable!()
    };
    let before = model
        .collide_terrain_raw_oriented(
            f.session.cache.level_terrain().unwrap(),
            [-14449, -1823, 31443],
            basis
                .orientation_world_from_model()
                .map(|row| row.map(f64::from)),
            &f.entities
                .iter_all()
                .find(|e| e.id == f.id)
                .unwrap()
                .presentation_anim_vars(1060),
        )
        .unwrap()
        .unwrap();
    assert_eq!(before.penetration_raw, 880.0);
    let outcome = f.run(1060);
    assert!(
        matches!(
            outcome,
            Intro2FlyerContactOutcome::Applied {
                solid_contact: true,
                ..
            }
        ),
        "{outcome:?}"
    );
    assert!(f.entity().position_raw()[1] > -1823);
    let entity = f.entities.iter_all().find(|e| e.id == f.id).unwrap();
    let after = f
        .session
        .cache
        .global_model(276)
        .unwrap()
        .collide_terrain_raw_oriented(
            f.session.cache.level_terrain().unwrap(),
            entity.position_raw(),
            basis
                .orientation_world_from_model()
                .map(|row| row.map(f64::from)),
            &entity.presentation_anim_vars(1060),
        )
        .unwrap();
    assert!(
        after.is_none_or(|hit| hit.penetration_raw <= 3.0),
        "{after:?}"
    );
}
