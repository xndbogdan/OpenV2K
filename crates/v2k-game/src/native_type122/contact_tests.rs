//! Real model274/terrain contacts exercise D9B0 before the retained11760 plane.

use super::*;
use crate::{
    actor_task_owner::ActorTaskSlot,
    damage::DamagePacket,
    gameplay_notifications::GameplayNotifications,
    intro2_contacts::Intro2ContactFrame,
    intro2_radial::Intro2RadialTaskCustody,
    native_actor_surface_contact::{
        resolve_native_actor_surface_contact, NativeActorSurfaceContactOutcome,
    },
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    static_contact::{
        apply_contact_response_raw, scan_deepest_static_contact, StaticContactQuery,
        StaticModelContact,
    },
    static_damage::{static_damage_kind_is_admitted, StaticDamageScheduler},
    static_damage_live::resolve_current_static_damage_target,
    world_fx::WorldFx,
};

pub(crate) struct Fixture {
    pub session: crate::session::GameSession,
    pub entities: EntityManager,
    pub fx: WorldFx,
    pub tasks: SpecializedActorTaskScheduler,
    pub damage: StaticDamageScheduler,
    pub notifications: GameplayNotifications,
    pub id: u32,
}

impl Fixture {
    pub fn new(level: u32) -> Self {
        let (session, entities, fx) = super::construction_tests::native_fixture(level);
        let id = entities
            .iter_all()
            .find(|e| e.entity_type == 122)
            .unwrap()
            .id;
        let mut tasks = SpecializedActorTaskScheduler::new();
        tasks.adopt_type122(&entities);
        Self {
            session,
            entities,
            fx,
            tasks,
            damage: StaticDamageScheduler::new(),
            notifications: GameplayNotifications::new(),
            id,
        }
    }
    pub fn entity(&self) -> &Entity {
        self.entities.iter_all().find(|e| e.id == self.id).unwrap()
    }
    pub fn frame(&mut self) -> Intro2ContactFrame<'_> {
        Intro2ContactFrame {
            entities: &mut self.entities,
            resources: &mut self.session.cache,
            world_fx: &mut self.fx,
            static_damage: &mut self.damage,
            notifications: &mut self.notifications,
            retail_tick: 0,
            actor_tasks: &mut self.tasks,
        }
    }
    pub fn die(&mut self) {
        let owner = crate::native_actor_capture::publish_native_captor_standard_death(
            &mut self.entities,
            self.id,
            &mut crate::native_actor_capture::CaptureContext {
                resources: None,
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
        .unwrap();
        self.tasks.register_intro2_common_dying(owner);
    }
    pub fn enable_contact(&mut self) {
        let e = self.entities.entity_mut(self.id).unwrap();
        e.collision
            .state_flags_at_0x08
            .overwrite(0x8000 | 0x1000 | 0x8800_0000, 0x8000);
        e.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
    }
    pub fn overlap(&mut self) -> StaticModelContact {
        let entity = self.entity();
        let terrain = self.session.cache.terrain().unwrap();
        let objects = self.session.cache.terrain_objects().unwrap();
        let model = self.session.cache.global_model(MODEL).unwrap();
        let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
            panic!("native basis")
        };
        for x in 0..256 {
            for z in 0..256 {
                let cell = terrain.cell(x, z).unwrap();
                if cell.attribute == 0
                    || !static_damage_kind_is_admitted(
                        objects.records[usize::from(cell.attribute)].kind_index,
                    )
                {
                    continue;
                }
                let x_raw = ((x as u16) << 8 | 0x80) as i16;
                let z_raw = ((z as u16) << 8 | 0x80) as i16;
                for dy in [0i16, 64, 128, 256, -64] {
                    for (dx, dz) in [(0i16, 0i16), (-128, 0), (128, 0), (0, -128), (0, 128)] {
                        let position = [
                            x_raw.wrapping_add(dx),
                            terrain.bilinear_height_raw(x_raw, z_raw).wrapping_add(dy),
                            z_raw.wrapping_add(dz),
                        ];
                        let contact = scan_deepest_static_contact(StaticContactQuery {
                            terrain,
                            terrain_objects: objects,
                            model_pool: &self.session.cache,
                            tick: 0,
                            active_model: model,
                            active_model_to_world_basis: basis
                                .orientation_world_from_model()
                                .map(|r| r.map(f64::from)),
                            active_anim_vars: &entity.presentation_anim_vars(0),
                            position_raw: position,
                        })
                        .unwrap();
                        if let Some(hit) =
                            contact.filter(|hit| static_damage_kind_is_admitted(hit.kind_index))
                        {
                            self.entities
                                .entity_mut(self.id)
                                .unwrap()
                                .set_motion_raw(position, [0; 3]);
                            return hit;
                        }
                    }
                }
            }
        }
        panic!("authored static geometry overlaps model274")
    }
}

#[v2k_test_support::retail_test]
fn generic_crushing_precedes_retained_plane_and_never_reselects_living_or_class12() {
    for level in [24, 42, 46, 49, 50] {
        for dying in [false, true] {
            let mut f = Fixture::new(level);
            if dying {
                f.die();
            }
            f.enable_contact();
            let plane = f.overlap();
            let target = resolve_current_static_damage_target(&f.session.cache, plane.cell)
                .unwrap()
                .unwrap();
            let before = f.entity();
            let context = before.current_behavior_context;
            let slots =
                ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|s| before.actor_tasks.task_in_slot(s));
            let mut position = before.position_raw();
            let mut velocity = before.velocity_raw();
            let d = before.native_type122_runtime;
            let h = before.sub_h_external_frame_runtime.clone();
            let basis = before.physical_body_basis_q31();
            apply_contact_response_raw(&mut position, &mut velocity, plane);
            let mut oracle_rng = f.fx.fork_for_main_base_abort_transaction();
            // A8B0 visits the real living Primary02CA0 first. Class12 has no
            // task callback; the later D9B0 still filters the full40000 packet.
            if !dying {
                oracle_rng.next_shared_retail_random_u16();
                oracle_rng.next_shared_retail_random_u16();
            }
            let expected = StaticDamageScheduler::new().submit_hit(
                target,
                DamagePacket {
                    channels: [1, 0],
                    amounts_raw: [40_000, 0],
                },
                &mut || oracle_rng.next_shared_retail_random_u16(),
            );
            let id = f.id;
            let result = contact::resolve_type122_static_contact(&mut f.frame(), id);
            let contact::Type122ContactOutcome::Applied(applied) = result else {
                panic!("world{level} dying{dying}: {result:?}")
            };
            assert_eq!(applied.contact, plane);
            assert_eq!(applied.crushing_damage, Some(expected));
            assert_eq!(
                (applied.position_after_raw, applied.velocity_after_raw),
                (position, velocity)
            );
            assert_eq!(applied.impact_raw, 0);
            assert!(applied.static_damage.is_none() && applied.actor_damage.is_none());
            let after = f.entity();
            assert_eq!(
                after.current_behavior_context, context,
                "D9B0 is not C890/C690"
            );
            assert_eq!(
                ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|s| after.actor_tasks.task_in_slot(s)),
                slots
            );
            assert_eq!(after.native_type122_runtime, d);
            assert_eq!(after.sub_h_external_frame_runtime, h);
            assert_eq!(after.physical_body_basis_q31(), basis);
            assert!(f.tasks.prepare_native_actor_mutation(&f.entities, id));
            assert_eq!(
                f.fx.next_shared_retail_random_u16(),
                oracle_rng.next_shared_retail_random_u16()
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn static_custody_rejection_consumes_neither_task_rng_nor_crushing_packet() {
    for pending in [false, true] {
        let mut f = Fixture::new(24);
        f.enable_contact();
        f.overlap();
        if pending {
            f.tasks.park_native_contact_prefix(&f.entities, f.id);
        } else {
            f.tasks = SpecializedActorTaskScheduler::new();
        }
        let before = f.entity().position_raw();
        let mut rng = f.fx.fork_for_main_base_abort_transaction();
        let id = f.id;
        assert!(matches!(
            contact::resolve_type122_static_contact(&mut f.frame(), id),
            contact::Type122ContactOutcome::Blocked {
                committed_prefix: false,
                ..
            }
        ));
        assert_eq!(f.entity().position_raw(), before);
        assert_eq!(f.damage.active_program_count(), 0);
        assert_eq!(
            f.fx.next_shared_retail_random_u16(),
            rng.next_shared_retail_random_u16()
        );
    }
}

#[v2k_test_support::retail_test]
fn class12_water_retains_native_custody_and_plays_only_hard_entry_sound17() {
    let mut f = Fixture::new(24);
    f.die();
    f.enable_contact();
    f.fx.garbage_collect_disposable_positional_sounds();
    let terrain = f.session.cache.level_terrain_mut().unwrap();
    terrain.header[0] = 0;
    for cell in &mut terrain.cells {
        cell.height = (-128i8) as u8;
        cell.attribute = 0;
        cell.terrain_type = 0;
    }
    let entity = f.entities.entity_mut(f.id).unwrap();
    entity.set_motion_raw([1024, 0, 2048], [0, -1001, 0]);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(0x600000, 0x400000);
    let id = f.id;
    let outcome = resolve_native_actor_surface_contact(&mut f.frame(), id);
    assert!(
        matches!(
            outcome,
            NativeActorSurfaceContactOutcome::Applied {
                solid_contact: false,
                water_entry: true,
                ring: Some(_),
                ..
            }
        ),
        "{outcome:?}"
    );
    // Retail SAR preserves the negative odd word's floor: -1001 >> 1.
    assert_eq!(f.entity().velocity_raw()[1], -501);
    assert!(f.tasks.prepare_native_actor_mutation(&f.entities, id));
    f.fx.process_pending();
    assert_eq!(
        f.fx.take_positional_sounds(),
        vec![crate::world_fx::PositionalSoundEvent::fixed(
            17,
            [4.0, 0.0, 8.0]
        ),]
    );
}
