use super::*;
use crate::{
    actor_task_owner::ActorTaskSlot,
    entity::{AuthoredWorldConstruction, EntityConstructionResources, EntityManager},
    gameplay_notifications::GameplayNotifications,
    intro2_type17::{
        capture::{publish_type17_standard_death, CaptureContext},
        Intro2Type17Owner,
    },
    session::GameSession,
    specialized_actor_task_production::{
        SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
        SpecializedActorTaskScheduler,
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
    fn new(level: u16) -> Option<Self> {
        let (mut session, metadata) = crate::intro2_type17::tests::fixture()?;
        session.load_level_by_id(u32::from(level), 1).unwrap();
        let mut fx = WorldFx::new();
        let model_extent = |id| session.cache.global_model(id).map(|model| model.radius);
        let resources = EntityConstructionResources {
            terrain: session.cache.terrain(),
            terrain_objects: session.cache.terrain_objects(),
            model_extent_raw: Some(&model_extent),
        };
        let entities = if level == 50 {
            EntityManager::from_native_intro2_frontend(
                session.cache.level_desc().unwrap(),
                &metadata,
                resources,
                0,
                &mut fx,
            )
            .unwrap()
        } else {
            EntityManager::from_authored_world(
                AuthoredWorldConstruction {
                    level: session.cache.level_desc().unwrap(),
                    logical_world_index: i32::from(level) - 12,
                    type_metadata: &metadata,
                    resources,
                    player_arrival: None,
                    retail_tick: 0,
                },
                &mut fx,
            )
            .unwrap()
        };
        let id = entities
            .iter_all()
            .find(|entity| entity.entity_type == 17)
            .unwrap()
            .id;
        let mut tasks = SpecializedActorTaskScheduler::default();
        tasks.register_intro2_type17(Intro2Type17Owner::adopt(&entities, id).unwrap());
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

    fn die(&mut self) {
        let owner = publish_type17_standard_death(
            &mut self.entities,
            self.id,
            &mut CaptureContext {
                tasks: &mut self.tasks,
                world_fx: &mut self.fx,
                notifications: &mut self.notifications,
                retail_tick: 0,
                result_screen:
                    crate::main_base_type9_abort::MainBaseType9ResultScreenState::NotShown,
                hive_dying: Default::default(),
            },
        )
        .unwrap()
        .expect("genuine native Class12 publication");
        self.tasks.register_intro2_common_dying(owner);
        self.fx.garbage_collect_disposable_positional_sounds();
        // Intro2 post-load activation is separate from the death callback.
        // This controlled contact represents an enabled local actor.
        self.entities
            .entity_mut(self.id)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .overwrite(0x8000, 0x8000);
        assert_eq!(
            self.entity().collision.state_flags_at_0x08.masked(0x10000),
            RetailRuntimeValue::Known(0x10000)
        );
    }

    fn entity(&self) -> &Entity {
        self.entities
            .iter_all()
            .find(|entity| entity.id == self.id)
            .unwrap()
    }

    fn run(&mut self, retail_tick: u32) -> NativeActorSurfaceContactOutcome {
        resolve_native_actor_surface_contact(
            &mut Intro2ContactFrame {
                entities: &mut self.entities,
                resources: &mut self.session.cache,
                actor_tasks: &mut self.tasks,
                world_fx: &mut self.fx,
                static_damage: &mut self.static_damage,
                notifications: &mut self.notifications,
                retail_tick,
            },
            self.id,
        )
    }

    fn flat_contact_terrain(&mut self, wet: bool) {
        let context =
            TerrainCollisionContext::from_current_level_cache(&self.session.cache).unwrap();
        let material = context
            .water_response_selectors
            .iter()
            .position(|value| *value <= 6)
            .unwrap() as u8;
        let terrain = self.session.cache.level_terrain_mut().unwrap();
        terrain.header[0] = if wet { 0 } else { (-4096i32).wrapping_shl(8) };
        for cell in &mut terrain.cells {
            cell.height = if wet { (-128i8) as u8 } else { 0 };
            cell.attribute = 0;
            cell.terrain_type = material;
        }
    }

    fn place_solid_overlap(&mut self, floor: i16) -> crate::intro2_meteors::MeteorTerrainResponse {
        let entity = self.entity();
        let model = self
            .session
            .cache
            .global_model(entity.model_index.unwrap())
            .unwrap();
        let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
            panic!("native basis")
        };
        let velocity = [120, -1800, -70];
        let mut selected = None;
        for dy in (0i16..=600).step_by(20) {
            let position = [1024, floor.wrapping_sub(dy), 2048];
            if let Some(hit) = model
                .collide_terrain_raw_oriented(
                    self.session.cache.terrain().unwrap(),
                    position,
                    basis
                        .orientation_world_from_model()
                        .map(|row| row.map(f64::from)),
                    &entity.presentation_anim_vars(0),
                )
                .unwrap()
            {
                let response = plan_meteor_terrain_response(
                    position,
                    velocity,
                    entity.mass_raw,
                    TerrainModelContact {
                        normal_q12: hit.normal.map(|n| (n * 4096.0).round() as i16),
                        penetration_raw: hit.penetration_raw as i32,
                    },
                );
                if response.particle_scale_raw != 0 && response.collision_damage_raw != 0 {
                    selected = Some((position, response));
                    break;
                }
            }
        }
        let (position, response) =
            selected.expect("actual model256 solid response with nonzero damage");
        let entity = self.entities.entity_mut(self.id).unwrap();
        entity.set_position_raw(position);
        entity.set_velocity_raw(velocity);
        response
    }
}

#[v2k_test_support::retail_test]
fn native_living_spiders_skip_surface_before_scheduler_and_basis_admission() {
    for level in [14, 20, 35, 50] {
        let Some(mut f) = Fixture::new(level) else {
            return;
        };
        assert_eq!(
            f.entity().collision.state_flags_at_0x08.masked(0x10000),
            RetailRuntimeValue::Known(0)
        );
        f.tasks = SpecializedActorTaskScheduler::default();
        f.entities.entity_mut(f.id).unwrap().physical_body_basis_q31 =
            RetailRuntimeValue::Unresolved;
        let before = f.entity().position_raw();
        let mut rng = f.fx.fork_for_main_base_abort_transaction();
        assert_eq!(f.run(1774), NativeActorSurfaceContactOutcome::Ineligible);
        assert_eq!(f.entity().position_raw(), before);
        assert_eq!(
            f.fx.next_shared_retail_random_u16(),
            rng.next_shared_retail_random_u16()
        );
    }
}

#[v2k_test_support::retail_test]
fn native_class12_solid_contact_retains_task_basis_and_uses_shared_checked_damage() {
    use crate::gameplay_notifications::GameplayNotificationPhase;
    for (level, notification_phase) in [
        (14, GameplayNotificationPhase::Playing),
        (20, GameplayNotificationPhase::Playing),
        (35, GameplayNotificationPhase::Playing),
        (50, GameplayNotificationPhase::NonGameplay),
    ] {
        let Some(mut f) = Fixture::new(level) else {
            return;
        };
        f.die();
        f.flat_contact_terrain(false);
        let response = f.place_solid_overlap(0);
        let task = f.entity().actor_task_state(ActorTaskSlot::Primary).cloned();
        let basis = f.entity().physical_body_basis_q31();
        assert_eq!(
            &f.session.cache.global_entity_type(17).unwrap().raw_header[0x86..0x88],
            &[0, 0],
            "canonical Type17 solid cue is null"
        );
        let result = f.run(0);
        assert!(
            matches!(result, NativeActorSurfaceContactOutcome::Applied {
            solid_contact: true, collision_damage_raw, water_entry: false, ..
        } if collision_damage_raw == response.collision_damage_raw),
            "{result:?}"
        );
        assert_eq!(f.entity().position_raw(), response.position_raw);
        assert_eq!(f.entity().velocity_raw(), response.velocity_raw);
        assert_eq!(f.entity().physical_body_basis_q31(), basis);
        assert_eq!(
            f.entity().actor_task_state(ActorTaskSlot::Primary),
            task.as_ref()
        );
        assert!(f.tasks.prepare_native_actor_mutation(&f.entities, f.id));
        let pass = f.tasks.tick(
            &mut f.entities,
            SpecializedActorTaskProductionFrame {
                world:
                    crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                hive_components: None,
                notification_phase,
                resources: &mut f.session.cache,
                world_fx: &mut f.fx,
                static_damage: &mut f.static_damage,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: 1,
                main_base_abort_active: false,
            },
            &mut f.notifications,
        );
        assert!(
            pass.outcomes.iter().any(|outcome| matches!(outcome,
                SpecializedActorTaskProductionOutcome::Intro2CommonDying(
                    crate::intro2_common_dying::Intro2CommonDyingOutcome::Advanced { entity_id, .. }
                    | crate::intro2_common_dying::Intro2CommonDyingOutcome::Waiting { entity_id }
                ) if *entity_id == f.id
            )),
            "{pass:?}"
        );
    }
}

#[v2k_test_support::retail_test]
fn native_class12_water_edges_preserve_clock_and_register_hard_entry_ring() {
    for level in [14, 50] {
        for vertical in [-1000, -1001, -1750, -1751] {
            let Some(mut f) = Fixture::new(level) else {
                return;
            };
            f.die();
            f.flat_contact_terrain(true);
            let entity = f.entities.entity_mut(f.id).unwrap();
            entity.set_position_raw([1024, 0, 2048]);
            entity.set_velocity_raw([21, vertical, 31]);
            entity
                .collision
                .state_flags_at_0x08
                .overwrite(0x600000, 0x400000);
            let task = f.entity().actor_task_state(ActorTaskSlot::Primary).cloned();
            let result = f.run(0);
            let NativeActorSurfaceContactOutcome::Applied {
                solid_contact: false,
                water_entry: true,
                response,
                ring,
                ..
            } = result
            else {
                panic!("{level} {vertical}: {result:?}")
            };
            if vertical < -1000 {
                assert_eq!(
                    response,
                    Some(WholeBodySurfaceResponse::HardImpact {
                        severe: vertical < -1750
                    })
                );
                assert!(ring.unwrap().primary_task_lease().is_some());
                assert_eq!(f.entity().velocity_raw(), [21, vertical >> 1, 31]);
            } else {
                assert!(matches!(
                    response,
                    Some(WholeBodySurfaceResponse::SurfaceBurst { .. })
                ));
                assert_eq!(f.entity().velocity_raw(), [21, vertical, 31]);
                assert!(ring.is_none());
            }
            // 4F450 queues the request; the later world-event/audio phase
            // makes it available to the mixer. The type+88 cue precedes the
            // hard-entry sound17, both at the original contact position.
            assert!(f.fx.take_positional_sounds().is_empty());
            f.fx.process_pending();
            let header = &f.session.cache.global_entity_type(17).unwrap().raw_header;
            let water_cue = u16::from_le_bytes([header[0x88], header[0x89]]);
            let mut expected_sounds = Vec::new();
            if water_cue != 0 {
                expected_sounds.push(crate::world_fx::PositionalSoundEvent::fixed(
                    usize::from(water_cue),
                    [4.0, 0.0, 8.0],
                ));
            }
            if vertical < -1000 {
                expected_sounds.push(crate::world_fx::PositionalSoundEvent::fixed(
                    17,
                    [4.0, 0.0, 8.0],
                ));
            }
            assert_eq!(
                f.fx.take_positional_sounds(),
                expected_sounds,
                "world{level} velocity{vertical}: source-ordered water sounds"
            );
            let particles = f.fx.particle_count();
            assert!(matches!(
                f.run(0),
                NativeActorSurfaceContactOutcome::Applied {
                    water_entry: false,
                    ..
                }
            ));
            f.fx.process_pending();
            assert!(
                f.fx.take_positional_sounds().is_empty(),
                "entry sounds are one-shot"
            );
            assert_eq!(
                f.fx.particle_count(),
                particles,
                "classification edge is one-shot"
            );
            assert_eq!(
                f.entity().actor_task_state(ActorTaskSlot::Primary),
                task.as_ref()
            );
            assert!(f.tasks.prepare_native_actor_mutation(&f.entities, f.id));
        }
    }
}

#[v2k_test_support::retail_test]
fn native_class12_late_water_block_parks_the_committed_solid_prefix() {
    let Some(mut f) = Fixture::new(20) else {
        return;
    };
    f.die();
    f.flat_contact_terrain(true);
    let response = f.place_solid_overlap(-4096);
    f.entities
        .entity_mut(f.id)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .invalidate(0x400000);
    let result = f.run(40);
    assert!(
        matches!(
            result,
            NativeActorSurfaceContactOutcome::Blocked {
                reason: NativeActorSurfaceContactBlock::Runtime("contact state"),
                committed_prefix: true,
            }
        ),
        "{result:?}"
    );
    assert_eq!(f.entity().position_raw(), response.position_raw);
    assert_eq!(f.entity().velocity_raw(), response.velocity_raw);
    assert!(!f.tasks.prepare_native_actor_mutation(&f.entities, f.id));
    let particles = f.fx.particle_count();
    let position = f.entity().position_raw();
    assert!(matches!(
        f.run(41),
        NativeActorSurfaceContactOutcome::Blocked {
            committed_prefix: false,
            ..
        }
    ));
    assert_eq!(f.entity().position_raw(), position);
    assert_eq!(f.fx.particle_count(), particles);
}

#[v2k_test_support::retail_test]
fn native_class12_missing_surface_owner_rejects_before_any_physical_write() {
    let Some(mut f) = Fixture::new(14) else {
        return;
    };
    f.die();
    f.flat_contact_terrain(false);
    f.place_solid_overlap(0);
    f.tasks = SpecializedActorTaskScheduler::default();
    let before = (f.entity().position_raw(), f.entity().velocity_raw());
    assert_eq!(
        f.run(0),
        NativeActorSurfaceContactOutcome::Blocked {
            reason: NativeActorSurfaceContactBlock::Runtime("current completed contact owner"),
            committed_prefix: false,
        }
    );
    assert_eq!(
        (f.entity().position_raw(), f.entity().velocity_raw()),
        before
    );
    assert_eq!(f.fx.particle_count(), 0);
}
