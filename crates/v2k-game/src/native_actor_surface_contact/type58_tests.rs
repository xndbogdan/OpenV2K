//! Native Type58 allocations retain Class12 custody through late 11AD0.

use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{
        ActorTaskId, ActorTaskSlot, ActorTaskVisit, ActorTaskWrapperFlags, PreparedActorTask,
    },
    common_mover::{type9_attitude::Type9BodyBasis, SubAPropulsionRuntime},
    entity::{Entity, EntityManager},
    entity_behavior::BehaviorContextRuntime,
    entity_collision_state::EntityCollisionRuntimeState,
    gameplay_notifications::GameplayNotifications,
    intro2_common_dying::publish_intro2_common_standard_death,
    intro2_type53::authored_tests::native_fixture,
    intro2_type58::{
        intro2_type58_allocation_authenticates, type58_manager_allocation_authenticates,
        Intro2Type58Owner, Intro2Type58Runtime, MODEL,
    },
    session::GameSession,
    specialized_actor_task_production::{
        SpecializedActorTaskFamily, SpecializedActorTaskScheduler,
    },
    static_damage::StaticDamageScheduler,
    sub_h_external_frame::SubHRuntimeState,
    type60_exploding_ring::Type60ConstructionProvenance,
    world_fx::{PositionalSoundEvent, WorldFx},
};

pub(crate) struct Fixture {
    pub(crate) session: GameSession,
    pub(crate) entities: EntityManager,
    pub(crate) tasks: SpecializedActorTaskScheduler,
    pub(crate) fx: WorldFx,
    pub(crate) static_damage: StaticDamageScheduler,
    pub(crate) notifications: GameplayNotifications,
    pub(crate) id: u32,
}

impl Fixture {
    pub(crate) fn new(level: u32) -> Self {
        let (session, entities, fx) = native_fixture(level);
        let count = entities
            .iter_all()
            .filter(|entity| entity.entity_type == 58)
            .count();
        assert_eq!(
            count,
            match level {
                14 => 4,
                24 => 2,
                31 | 50 => 1,
                _ => unreachable!(),
            }
        );
        let id = entities
            .iter_all()
            .find(|entity| entity.entity_type == 58)
            .unwrap()
            .id;
        assert!(type58_manager_allocation_authenticates(&entities, id));
        let mut tasks = SpecializedActorTaskScheduler::default();
        tasks.register_intro2_type58(Intro2Type58Owner::adopt(&entities, id).unwrap());
        Self {
            session,
            entities,
            tasks,
            fx,
            static_damage: StaticDamageScheduler::new(),
            notifications: GameplayNotifications::new(),
            id,
        }
    }

    pub(crate) fn entity(&self) -> &Entity {
        self.entities
            .iter_all()
            .find(|entity| entity.id == self.id)
            .unwrap()
    }

    pub(crate) fn die(&mut self) {
        let owner = publish_intro2_common_standard_death(&mut self.entities, self.id, &mut self.fx)
            .unwrap()
            .expect("native C850 Class12 publication");
        self.tasks.register_intro2_common_dying(owner);
        self.fx.garbage_collect_disposable_positional_sounds();
        let entity = self.entities.entity_mut(self.id).unwrap();
        // Frontend activation is distinct from C850. Enable the controlled
        // local late-contact visit without changing Class12's surface gate.
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(0x8000, 0x8000);
        entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
        assert_eq!(
            entity.collision.state_flags_at_0x08.masked(0x10000),
            RetailRuntimeValue::Known(0x10000)
        );
        assert_eq!(entity.model_slots, [Some(MODEL); 4]);
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::CommonDying(_))
        ));
        assert!(self
            .tasks
            .prepare_native_actor_mutation(&self.entities, self.id));
    }

    pub(crate) fn frame(&mut self, retail_tick: u32) -> Intro2ContactFrame<'_> {
        Intro2ContactFrame {
            entities: &mut self.entities,
            resources: &mut self.session.cache,
            actor_tasks: &mut self.tasks,
            world_fx: &mut self.fx,
            static_damage: &mut self.static_damage,
            notifications: &mut self.notifications,
            retail_tick,
        }
    }

    pub(crate) fn run(&mut self, retail_tick: u32) -> NativeActorSurfaceContactOutcome {
        let id = self.id;
        resolve_native_actor_surface_contact(&mut self.frame(retail_tick), id)
    }

    pub(crate) fn snapshot(&self) -> Snapshot {
        Snapshot::from_entity(self.entity())
    }

    fn flat_terrain(&mut self, wet: bool) {
        let context =
            TerrainCollisionContext::from_current_level_cache(&self.session.cache).unwrap();
        let material = context
            .water_response_selectors
            .iter()
            .position(|selector| *selector <= 6)
            .unwrap() as u8;
        let terrain = self.session.cache.level_terrain_mut().unwrap();
        terrain.header[0] = if wet { 0 } else { (-4096i32).wrapping_shl(8) };
        for cell in &mut terrain.cells {
            cell.height = if wet { (-128i8) as u8 } else { 0 };
            cell.attribute = 0;
            cell.terrain_type = material;
        }
    }

    fn solid_overlap(&mut self, floor: i16) -> crate::intro2_meteors::MeteorTerrainResponse {
        let entity = self.entity();
        let model = self.session.cache.global_model(MODEL).unwrap();
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
        let (position, response) = selected.expect("actual model273 solid hit with nonzero damage");
        self.entities
            .entity_mut(self.id)
            .unwrap()
            .set_motion_raw(position, velocity);
        response
    }

    fn water_entry(&mut self, velocity_y: i16) {
        self.flat_terrain(true);
        let entity = self.entities.entity_mut(self.id).unwrap();
        entity.set_motion_raw([1024, 0, 2048], [21, velocity_y, 31]);
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(0x600000, 0x400000);
    }

    pub(crate) fn invalidate_custody(&mut self, case: u8) {
        match case {
            0 => self.tasks = SpecializedActorTaskScheduler::default(),
            1 => {
                self.tasks
                    .park_native_contact_prefix(&self.entities, self.id);
            }
            2 => {
                let entity = self.entities.entity_mut(self.id).unwrap();
                let task_id = entity
                    .actor_tasks
                    .task_in_slot(ActorTaskSlot::Primary)
                    .unwrap();
                entity
                    .actor_tasks
                    .begin_exact_visit_with(
                        ActorTaskVisit {
                            slot: ActorTaskSlot::Primary,
                            task_id,
                        },
                        |_| (),
                    )
                    .unwrap();
            }
            3 => {
                let entity = self.entities.entity_mut(self.id).unwrap();
                let state = *entity.actor_task_state(ActorTaskSlot::Primary).unwrap();
                entity
                    .actor_tasks
                    .replace_prepared(ActorTaskSlot::Primary, PreparedActorTask::new(state));
            }
            4 => {
                let mut other = Fixture::new(14);
                other.die();
                std::mem::swap(
                    self.entities.entity_mut(self.id).unwrap(),
                    other.entities.entity_mut(other.id).unwrap(),
                );
                assert!(intro2_type58_allocation_authenticates(self.entity()));
                assert!(!type58_manager_allocation_authenticates(
                    &self.entities,
                    self.id
                ));
            }
            5 => {
                self.entities
                    .entity_mut(self.id)
                    .unwrap()
                    .collision
                    .subject_scan_gate_at_0x70 = RetailRuntimeValue::Unresolved
            }
            6 => {
                self.entities
                    .entity_mut(self.id)
                    .unwrap()
                    .physical_body_basis_q31 = RetailRuntimeValue::Unresolved
            }
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Snapshot {
    pub(crate) position: [i16; 3],
    pub(crate) velocity: [i16; 3],
    pub(crate) rotation: [i16; 3],
    pub(crate) basis: RetailRuntimeValue<Type9BodyBasis>,
    pub(crate) collision: EntityCollisionRuntimeState,
    pub(crate) runtime: Option<Intro2Type58Runtime>,
    pub(crate) sub_a: RetailRuntimeValue<Option<SubAPropulsionRuntime>>,
    pub(crate) sub_h: RetailRuntimeValue<Option<SubHRuntimeState>>,
    pub(crate) context: RetailRuntimeValue<Option<BehaviorContextRuntime>>,
    pub(crate) models: [Option<usize>; 4],
    pub(crate) tasks: [Option<(ActorTaskId, ActorTaskRuntime, ActorTaskWrapperFlags)>; 3],
}

impl Snapshot {
    fn from_entity(entity: &Entity) -> Self {
        Self {
            position: entity.position_raw(),
            velocity: entity.velocity_raw(),
            rotation: entity.rotation_heading_pitch_roll_raw(),
            basis: entity.physical_body_basis_q31(),
            collision: entity.collision.clone(),
            runtime: entity.intro2_type58_runtime,
            sub_a: entity.sub_a_propulsion_runtime,
            sub_h: entity.sub_h_external_frame_runtime.clone(),
            context: entity.current_behavior_context,
            models: entity.model_slots,
            tasks: ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| {
                entity.actor_tasks.task_in_slot(slot).map(|id| {
                    (
                        id,
                        *entity.actor_tasks.task_state(id).unwrap(),
                        entity.actor_tasks.wrapper_flags(id).unwrap(),
                    )
                })
            }),
        }
    }

    pub(crate) fn assert_graph_and_components_retained(&self, before: &Self) {
        assert_eq!(self.tasks, before.tasks);
        assert_eq!(
            (self.rotation, self.basis, self.context, self.models),
            (before.rotation, before.basis, before.context, before.models)
        );
        assert_eq!(
            (self.runtime, self.sub_a, &self.sub_h),
            (before.runtime, before.sub_a, &before.sub_h)
        );
    }
}

#[v2k_test_support::retail_test]
fn native_type58_living_surface_gate_precedes_owner_and_basis_admission() {
    for level in [14, 24, 31, 50] {
        let mut f = Fixture::new(level);
        assert_eq!(
            f.entity().collision.state_flags_at_0x08.masked(0x10000),
            RetailRuntimeValue::Known(0)
        );
        f.tasks = SpecializedActorTaskScheduler::default();
        let entity = f.entities.entity_mut(f.id).unwrap();
        entity.physical_body_basis_q31 = RetailRuntimeValue::Unresolved;
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(0x8000, 0x8000);
        entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
        let before = f.snapshot();
        let mut oracle = f.fx.fork_for_main_base_abort_transaction();
        assert_eq!(
            f.run(1774),
            NativeActorSurfaceContactOutcome::Ineligible,
            "world{level}"
        );
        assert_eq!(f.snapshot(), before);
        assert_eq!(f.fx.particle_count(), 0);
        assert_eq!(
            f.fx.next_shared_retail_random_u16(),
            oracle.next_shared_retail_random_u16()
        );
    }
}

#[v2k_test_support::retail_test]
fn native_type58_class12_solid_contact_preserves_model_and_death_clock() {
    for level in [14, 24, 31, 50] {
        let mut f = Fixture::new(level);
        f.die();
        f.flat_terrain(false);
        let expected = f.solid_overlap(0);
        let before = f.snapshot();
        assert_eq!(
            &f.session.cache.global_entity_type(58).unwrap().raw_header[0x86..0x88],
            &[0, 0]
        );
        let result = f.run(0);
        assert!(
            matches!(result, NativeActorSurfaceContactOutcome::Applied { solid_contact: true, collision_damage_raw, water_entry: false, response: None, ring: None } if collision_damage_raw == expected.collision_damage_raw),
            "world{level}: {result:?}"
        );
        let after = f.snapshot();
        assert_eq!(
            (after.position, after.velocity),
            (expected.position_raw, expected.velocity_raw)
        );
        after.assert_graph_and_components_retained(&before);
        assert!(f.fx.particle_count() > 0, "141D0 solid scatter");
        assert_eq!(
            f.tasks.family_for(f.id),
            Some(SpecializedActorTaskFamily::Intro2CommonDying)
        );
        assert!(f.tasks.prepare_native_actor_mutation(&f.entities, f.id));
    }
}

#[v2k_test_support::retail_test]
fn native_type58_class12_classifies_water_after_the_solid_velocity_response() {
    for level in [14, 24, 31, 50] {
        let mut f = Fixture::new(level);
        f.die();
        f.flat_terrain(true);
        let solid = f.solid_overlap(-4096);
        f.entities
            .entity_mut(f.id)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .overwrite(0x600000, 0x400000);
        let before = f.snapshot();
        let mut classification_state = before.collision.state_flags_at_0x08;
        classification_state.overwrite(0x800000, 0x800000);
        let classification = classify_whole_body_surface(WholeBodySurfaceClassificationRequest {
            state_before: classification_state,
            terrain: f.session.cache.terrain().unwrap(),
            position_raw: solid.position_raw,
            collision_radius_raw: f
                .session
                .cache
                .global_model(MODEL)
                .unwrap()
                .collision_radius_raw as u16,
            retail_tick: 0,
            waves_enabled: f.session.cache.level_desc().unwrap().raw_u32(0x84).unwrap() != 0,
            static_sea_level_raw: Some(0),
        });
        let context = TerrainCollisionContext::from_current_level_cache(&f.session.cache).unwrap();
        let expected =
            plan_fallback_whole_body_surface_response(WholeBodySurfaceFallbackResponseRequest {
                contact: classification
                    .entry_contact
                    .expect("solid-surviving wet sphere"),
                vertical_velocity_raw: solid.velocity_raw[1],
                water_response_selectors: context.water_response_selectors,
            });
        assert!(
            matches!(
                expected.response,
                Some(WholeBodySurfaceResponse::SurfaceBurst { .. })
            ),
            "solid response removes incoming hard-entry speed"
        );
        let result = f.run(0);
        assert!(
            matches!(result, NativeActorSurfaceContactOutcome::Applied { solid_contact: true, water_entry: true, response, ring: None, .. } if response == expected.response),
            "world{level}: {result:?}"
        );
        let after = f.snapshot();
        assert_eq!(
            (after.position, after.velocity),
            (solid.position_raw, solid.velocity_raw)
        );
        assert_eq!(
            after.collision.state_flags_at_0x08,
            classification.state_after
        );
        after.assert_graph_and_components_retained(&before);
        assert!(f.tasks.prepare_native_actor_mutation(&f.entities, f.id));
    }
}

#[v2k_test_support::retail_test]
fn native_type58_class12_water_thresholds_link_native_rings_and_preserve_caller_suffix() {
    for level in [14, 24, 31, 50] {
        for velocity_y in [-1000, -1001, -1750, -1751] {
            let mut f = Fixture::new(level);
            f.die();
            f.water_entry(velocity_y);
            let before = f.snapshot();
            let result = f.run(0);
            let NativeActorSurfaceContactOutcome::Applied {
                solid_contact: false,
                water_entry: true,
                response,
                ring,
                ..
            } = result
            else {
                panic!("world{level} y{velocity_y}: {result:?}")
            };
            if velocity_y < -1000 {
                let severe = velocity_y < -1750;
                assert_eq!(
                    response,
                    Some(WholeBodySurfaceResponse::HardImpact { severe })
                );
                let lease = ring
                    .unwrap()
                    .primary_task_lease()
                    .expect("actual class48 ring task");
                let ring_entity = f
                    .entities
                    .iter_all()
                    .find(|entity| entity.id == lease.actor().entity_id)
                    .unwrap();
                assert_eq!(
                    ring_entity.type60_construction_provenance(),
                    Some(if severe {
                        Type60ConstructionProvenance::HardWaterSevere
                    } else {
                        Type60ConstructionProvenance::HardWaterModerate
                    })
                );
                assert_eq!(
                    ring_entity.model_slots,
                    [Some(if severe { 130 } else { 132 }); 4]
                );
                assert_eq!(
                    f.tasks.family_for(ring_entity.id),
                    Some(SpecializedActorTaskFamily::Type60ExplodingRing)
                );
                assert_eq!(f.entity().velocity_raw(), [21, velocity_y >> 1, 31]);
            } else {
                assert!(matches!(
                    response,
                    Some(WholeBodySurfaceResponse::SurfaceBurst { .. })
                ));
                assert!(ring.is_none());
                assert_eq!(f.entity().velocity_raw(), [21, velocity_y, 31]);
            }
            assert!(f.fx.take_positional_sounds().is_empty());
            f.fx.process_pending();
            let header = &f.session.cache.global_entity_type(58).unwrap().raw_header;
            let cue = u16::from_le_bytes([header[0x88], header[0x89]]);
            let mut sounds = Vec::new();
            if cue != 0 {
                sounds.push(PositionalSoundEvent::fixed(
                    usize::from(cue),
                    [4.0, 0.0, 8.0],
                ));
            }
            if velocity_y < -1000 {
                sounds.push(PositionalSoundEvent::fixed(17, [4.0, 0.0, 8.0]));
            }
            assert_eq!(
                f.fx.take_positional_sounds(),
                sounds,
                "type+88 before hard sound17"
            );
            let particles = f.fx.particle_count();
            let count = f.entities.iter_all().count();
            let mut rng = f.fx.fork_for_main_base_abort_transaction();
            assert!(matches!(
                f.run(0),
                NativeActorSurfaceContactOutcome::Applied {
                    water_entry: false,
                    ..
                }
            ));
            f.fx.process_pending();
            assert!(f.fx.take_positional_sounds().is_empty());
            assert_eq!(f.fx.particle_count(), particles);
            assert_eq!(f.entities.iter_all().count(), count);
            assert_eq!(
                f.fx.next_shared_retail_random_u16(),
                rng.next_shared_retail_random_u16()
            );
            f.snapshot().assert_graph_and_components_retained(&before);
        }
    }
}

#[v2k_test_support::retail_test]
fn native_type58_class12_water_sphere_uses_retained_model273_radius_at_strict_edge() {
    for level in [14, 24, 31, 50] {
        for delta in [-1i16, 0, 1] {
            let mut f = Fixture::new(level);
            f.die();
            f.water_entry(0);
            let model = f.session.cache.global_model(MODEL).unwrap();
            assert_eq!(
                model.collision_radius_raw, 250,
                "model273 differs from Type17's300"
            );
            let waves = f.session.cache.level_desc().unwrap().raw_u32(0x84).unwrap() != 0;
            let surface = if waves {
                v2k_formats::terrain::wave_surface_raw(1024, 2048, 0, 0, -4096)
            } else {
                0
            };
            let position = [1024, surface.wrapping_add(250).wrapping_add(delta), 2048];
            f.entities
                .entity_mut(f.id)
                .unwrap()
                .set_position_raw(position);
            let before = f.snapshot();
            let result = f.run(0);
            assert!(
                matches!(result, NativeActorSurfaceContactOutcome::Applied {
                solid_contact: false, water_entry, ring: None, ..
            } if water_entry == (delta <= 0)),
                "world{level} delta{delta}: {result:?}"
            );
            assert_eq!(f.entity().position_raw(), position);
            assert_eq!(
                f.entity().collision.state_flags_at_0x08.masked(0x600000),
                RetailRuntimeValue::Known(if delta > 0 { 0x400000 } else { 0 })
            );
            f.snapshot().assert_graph_and_components_retained(&before);
        }
    }
}

#[v2k_test_support::retail_test]
fn native_type58_rejected_ring_admission_still_sounds_and_halves_velocity_without_rng() {
    for velocity_y in [-1001, -1751] {
        let mut f = Fixture::new(14);
        f.die();
        f.water_entry(velocity_y);
        f.entities
            .type_runtime_metadata_mut_for_test(60)
            .unwrap()
            .capability_flags = 0;
        let before = f.snapshot();
        let count = f.entities.iter_all().count();
        let mut rng = f.fx.fork_for_main_base_abort_transaction();
        let result = f.run(0);
        assert!(
            matches!(
                result,
                NativeActorSurfaceContactOutcome::Applied {
                    water_entry: true,
                    ring: Some(Type60ConstructionOutcome::RejectedBeforeSelector),
                    ..
                }
            ),
            "{result:?}"
        );
        assert_eq!(f.entities.iter_all().count(), count);
        assert_eq!(f.entity().velocity_raw(), [21, velocity_y >> 1, 31]);
        f.fx.process_pending();
        let sounds = f.fx.take_positional_sounds();
        assert_eq!(
            sounds.last(),
            Some(&PositionalSoundEvent::fixed(17, [4.0, 0.0, 8.0]))
        );
        assert_eq!(
            f.fx.next_shared_retail_random_u16(),
            rng.next_shared_retail_random_u16()
        );
        f.snapshot().assert_graph_and_components_retained(&before);
        assert!(f.tasks.prepare_native_actor_mutation(&f.entities, f.id));
    }
}

#[v2k_test_support::retail_test]
fn native_type58_class12_surface_rejects_unknown_executing_stale_and_foreign_custody() {
    for case in 0..7 {
        let mut f = Fixture::new(14);
        f.die();
        f.flat_terrain(false);
        f.solid_overlap(0);
        f.invalidate_custody(case);
        let before = f.snapshot();
        let mut rng = f.fx.fork_for_main_base_abort_transaction();
        let result = f.run(0);
        assert!(
            matches!(
                result,
                NativeActorSurfaceContactOutcome::Blocked {
                    committed_prefix: false,
                    ..
                }
            ),
            "case{case}: {result:?}"
        );
        assert_eq!(f.snapshot(), before);
        assert_eq!(f.fx.particle_count(), 0);
        assert_eq!(f.static_damage.active_program_count(), 0);
        assert_eq!(
            f.fx.next_shared_retail_random_u16(),
            rng.next_shared_retail_random_u16()
        );
    }
}

#[v2k_test_support::retail_test]
fn native_type58_class12_late_water_failure_parks_exact_solid_prefix() {
    let mut f = Fixture::new(14);
    f.die();
    f.flat_terrain(true);
    let response = f.solid_overlap(-4096);
    f.entities
        .entity_mut(f.id)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .invalidate(0x400000);
    let before = f.snapshot();
    let result = f.run(40);
    assert!(
        matches!(
            result,
            NativeActorSurfaceContactOutcome::Blocked {
                committed_prefix: true,
                ..
            }
        ),
        "{result:?}"
    );
    let after = f.snapshot();
    assert_eq!(
        (after.position, after.velocity),
        (response.position_raw, response.velocity_raw)
    );
    after.assert_graph_and_components_retained(&before);
    assert!(f.fx.particle_count() > 0);
    assert!(!f.tasks.prepare_native_actor_mutation(&f.entities, f.id));
    f.tasks
        .register_intro2_common_dying(Intro2CommonDyingOwner::adopt(&f.entities, f.id).unwrap());
    assert!(
        !f.tasks.prepare_native_actor_mutation(&f.entities, f.id),
        "readoption cannot unpark committed work"
    );
    let mut rng = f.fx.fork_for_main_base_abort_transaction();
    let particles = f.fx.particle_count();
    assert!(matches!(
        f.run(41),
        NativeActorSurfaceContactOutcome::Blocked {
            committed_prefix: false,
            ..
        }
    ));
    assert_eq!(f.snapshot(), after);
    assert_eq!(f.fx.particle_count(), particles);
    assert_eq!(
        f.fx.next_shared_retail_random_u16(),
        rng.next_shared_retail_random_u16()
    );
}
