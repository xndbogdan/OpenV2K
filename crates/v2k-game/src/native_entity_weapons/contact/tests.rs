use super::*;
use crate::{
    entity::{AuthoredPlayerArrival, AuthoredWorldConstruction, EntityConstructionResources},
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailStateWord},
    native_entity_weapons::{EntityWeaponConstructionRequest, EntityWeaponKind},
    session::GameSession,
    type60_exploding_ring::Type60ConstructionProvenance,
};
use v2k_formats::terrain::wave_surface_raw;

struct Fixture {
    session: GameSession,
    manager: EntityManager,
    fx: WorldFx,
    owner: NativeEntityWeaponOwner,
}

impl Fixture {
    fn new() -> Self {
        Self::with_kind(EntityWeaponKind::Grenade)
    }

    fn with_kind(kind: EntityWeaponKind) -> Self {
        let root = v2k_test_support::retail_dir();
        let mut session = GameSession::init(&root).expect("retail corpus required");
        session.load_auxiliary_ovl(3, 1).unwrap();
        session.load_level_by_id(17, 1).unwrap();
        let metadata: Vec<_> = session
            .cache
            .global_entity_model_table()
            .iter()
            .enumerate()
            .map(|(id, _)| {
                EntityTypeRuntimeMetadata::from_section12(
                    session.cache.global_entity_type(id).unwrap(),
                )
            })
            .collect();
        let mut fx = WorldFx::new();
        let mut manager = EntityManager::from_authored_world(
            AuthoredWorldConstruction {
                level: session.cache.level_desc().unwrap(),
                logical_world_index: 5,
                type_metadata: &metadata,
                resources: EntityConstructionResources {
                    terrain: session.cache.level_terrain(),
                    terrain_objects: session.cache.terrain_objects(),
                    model_extent_raw: Some(&|id| {
                        session.cache.global_model(id).map(|model| model.radius)
                    }),
                },
                player_arrival: Some(AuthoredPlayerArrival {
                    position_raw: [1000, 1024, 2000],
                    heading_raw: 0,
                }),
                retail_tick: 0,
            },
            &mut fx,
        )
        .unwrap();
        let owner = manager
            .construct_entity_weapon(
                EntityWeaponConstructionRequest {
                    kind,
                    source_actor_id: manager.player().unwrap().id,
                    position_raw: [1000, 500, 2000],
                    velocity_raw: [0; 3],
                    rotation_raw: [0; 3],
                },
                &session.cache,
                &mut fx,
                0,
            )
            .unwrap();
        Self {
            session,
            manager,
            fx,
            owner,
        }
    }

    fn wet_edge(&mut self, velocity_y: i16) -> [i16; 3] {
        let material = self
            .session
            .cache
            .level_desc()
            .unwrap()
            .water_response_selectors()
            .iter()
            .position(|&selector| selector < 7)
            .unwrap();
        let terrain = self.session.cache.level_terrain_mut().unwrap();
        terrain.header[0] = 0;
        for cell in &mut terrain.cells {
            cell.height = (-32i8) as u8;
            cell.attribute = 0;
            cell.terrain_type = material as u8;
        }
        let waves = self
            .session
            .cache
            .level_desc()
            .unwrap()
            .raw_u32(0x84)
            .unwrap()
            != 0;
        let y = if waves {
            wave_surface_raw(1000, 2000, 0, 0, -1024)
        } else {
            0
        };
        let entity = self.manager.entity_mut(self.owner.entity_id()).unwrap();
        entity.set_position_raw([1000, y, 2000]);
        entity.set_velocity_raw([100, velocity_y, 300]);
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(0x0060_0000, 0x0040_0000);
        [1000, y, 2000]
    }

    fn run(&mut self) -> EntityWeaponSurfaceOutcome {
        resolve_entity_weapon_surface_contact_with_terminal(
            &mut self.manager,
            self.owner,
            EntityWeaponSurfaceFrame {
                resources: &mut self.session.cache,
                world_fx: &mut self.fx,
                retail_tick: 0,
            },
            |_, _, _, _| -> Result<bool, ()> {
                panic!("nonlethal controlled contact cannot enter death")
            },
        )
        .unwrap()
    }

    fn isolate_terminal(&mut self, weapons: &[u32]) {
        for id in self.manager.retail_live_order_ids().collect::<Vec<_>>() {
            let entity = self.manager.entity_mut(id).unwrap();
            if !weapons.contains(&id) {
                entity.collision.state_flags_at_0x08.overwrite(u32::MAX, 0);
            }
        }
        let terrain = self.session.cache.level_terrain_mut().unwrap();
        for cell in &mut terrain.cells {
            cell.attribute = 0;
        }
    }
}

#[v2k_test_support::retail_test]
fn hard_entry_constructs_real_ring_before_sound_and_signed_velocity_damping() {
    for (velocity, provenance, expected_model, damped) in [
        (
            -1001,
            Type60ConstructionProvenance::HardWaterModerate,
            132,
            -501,
        ),
        (
            -1751,
            Type60ConstructionProvenance::HardWaterSevere,
            130,
            -876,
        ),
    ] {
        let mut fixture = Fixture::new();
        let body_position = fixture.wet_edge(velocity);
        let before_count = fixture.manager.iter_all().count();
        fixture.fx.take_positional_sounds();
        let outcome = fixture.run();
        let EntityWeaponSurfaceOutcome::Applied {
            water_entry: true,
            ring: Some(Type60ConstructionOutcome::ActorLinked(receipt)),
            ..
        } = outcome
        else {
            panic!("actual hard-water ring expected: {outcome:?}")
        };
        assert_eq!(receipt.provenance(), provenance);
        assert_eq!(fixture.manager.iter_all().count(), before_count + 1);
        let ring = fixture
            .manager
            .iter_all()
            .find(|entity| entity.id == receipt.actor().entity_id)
            .unwrap();
        assert_eq!(ring.entity_type, 60);
        assert_eq!(ring.model_slots, [Some(expected_model); 4]);
        assert_eq!(ring.position_raw(), body_position);
        assert_eq!(
            fixture
                .manager
                .entity_mut(fixture.owner.entity_id())
                .unwrap()
                .velocity_raw(),
            [100, damped, 300]
        );
        let sounds = fixture.fx.take_positional_sounds();
        assert!(sounds.iter().any(|sound| sound.sound_id == 17));
        // Classification consumed the edge; the same pose cannot construct a
        // second ring merely because the caller still owns the projectile.
        let next = fixture.run();
        assert!(matches!(
            next,
            EntityWeaponSurfaceOutcome::Applied {
                water_entry: false,
                ..
            }
        ));
        assert_eq!(fixture.manager.iter_all().count(), before_count + 1);
    }
}

#[v2k_test_support::retail_test]
fn ordinary_water_entry_keeps_velocity_and_does_not_allocate_a_ring() {
    let mut fixture = Fixture::new();
    fixture.wet_edge(-1000);
    let count = fixture.manager.iter_all().count();
    assert!(matches!(
        fixture.run(),
        EntityWeaponSurfaceOutcome::Applied {
            water_entry: true,
            response: Some(WholeBodySurfaceResponse::SurfaceBurst { .. }),
            ring: None,
            ..
        }
    ));
    assert_eq!(fixture.manager.iter_all().count(), count);
    assert_eq!(
        fixture
            .manager
            .entity_mut(fixture.owner.entity_id())
            .unwrap()
            .velocity_raw(),
        [100, -1000, 300]
    );
}

#[v2k_test_support::retail_test]
fn terrain_uses_authored_model_collision_program_and_shared_damage_filter() {
    let mut fixture = Fixture::new();
    let terrain = fixture.session.cache.level_terrain_mut().unwrap();
    terrain.header[0] = (-4096i32).wrapping_shl(8);
    for cell in &mut terrain.cells {
        cell.height = 0;
        cell.attribute = 0;
        cell.terrain_type = 0;
    }
    let entity = fixture
        .manager
        .entity_mut(fixture.owner.entity_id())
        .unwrap();
    entity.set_position_raw([1000, 0, 2000]);
    entity.set_velocity_raw([100, -1000, 300]);
    let before_health = entity.collision.health_raw;
    let outcome = fixture.run();
    let EntityWeaponSurfaceOutcome::Applied {
        solid_contact: true,
        collision_damage_raw,
        ..
    } = outcome
    else {
        panic!("model128 collision program must contact the plane: {outcome:?}")
    };
    assert!(collision_damage_raw > 0 && collision_damage_raw <= 2000);
    let entity = fixture
        .manager
        .entity_mut(fixture.owner.entity_id())
        .unwrap();
    assert!(entity.position_raw()[1] > 0);
    assert_eq!(entity.velocity_raw(), [100, -1, 300]);
    assert_eq!(
        entity.collision.health_raw, before_health,
        "channel1 filters below authored2000 threshold"
    );
    assert_eq!(
        entity.collision.state_flags_at_0x08.masked(0x0080_0000),
        RetailRuntimeValue::Known(0x0080_0000)
    );
}

#[v2k_test_support::retail_test]
fn ineligible_subject_skips_missing_physical_basis_before_custody() {
    let mut fixture = Fixture::new();
    let entity = fixture
        .manager
        .entity_mut(fixture.owner.entity_id())
        .unwrap();
    entity.physical_body_basis_q31 = RetailRuntimeValue::Unresolved;
    entity.collision.state_flags_at_0x08 = RetailStateWord::exact(0);
    assert_eq!(fixture.run(), EntityWeaponSurfaceOutcome::Ineligible);
}

#[v2k_test_support::retail_test]
fn actual_rocket_solid_terminal_finishes_before_shared_motion_and_water_tail() {
    use crate::{
        gameplay_notifications::GameplayNotifications,
        specialized_actor_task_production::{
            SpecializedActorTaskScheduler, SpecializedActorTaskWorld,
        },
        static_damage::StaticDamageScheduler,
    };
    let mut f = Fixture::with_kind(EntityWeaponKind::Rocket);
    f.isolate_terminal(&[f.owner.entity_id()]);
    let terrain = f.session.cache.level_terrain_mut().unwrap();
    terrain.header[0] = (-4096i32).wrapping_shl(8);
    for cell in &mut terrain.cells {
        cell.height = 0;
        cell.terrain_type = 0;
    }
    let body = f.manager.entity_mut(f.owner.entity_id()).unwrap();
    body.set_motion_raw([1000, 0, 2000], [100, -1000, 300]);
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler
        .register_native_weapon(&f.manager, f.owner)
        .unwrap();
    let mut damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    let mut hull = crate::player_hull::PlayerHull::new(
        crate::player_hull::HullDamageProfile::from_type_record(
            f.session.cache.global_entity_type(46).unwrap(),
        ),
    );
    hull.readback_entity_damage_state(&f.manager.player().unwrap().collision);
    let result = resolve_entity_weapon_contacts(
        EntityWeaponContactFrame {
            common: crate::intro2_contacts::Intro2ContactFrame {
                entities: &mut f.manager,
                resources: &mut f.session.cache,
                world_fx: &mut f.fx,
                static_damage: &mut damage,
                notifications: &mut notifications,
                retail_tick: 2,
                actor_tasks: &mut scheduler,
            },
            world: SpecializedActorTaskWorld::Playing {
                player_hull: &mut hull,
                extra_lives: RetailRuntimeValue::Known(3),
            },
        },
        f.owner,
    )
    .unwrap();
    assert!(
        matches!(
            result,
            EntityWeaponContactOutcome::Applied {
                surface: EntityWeaponSurfaceOutcome::Applied {
                    solid_contact: true,
                    ..
                },
                static_objects: static_objects::EntityWeaponStaticOutcome::Miss,
                active_pairs:
                    crate::native_actor_capture::pair::NativeCaptorPairOutcome::Resolved { .. },
            }
        ),
        "{result:?}"
    );
    assert!(crate::class49_death::finished_terminal_hit_authenticates(
        &f.manager,
        f.owner.entity_id()
    ));
    let entity = f.manager.entity_mut(f.owner.entity_id()).unwrap();
    assert!(crate::actor_task_owner::ActorTaskSlot::IN_RETAIL_TICK_ORDER
        .into_iter()
        .all(|slot| entity.actor_tasks.task_in_slot(slot).is_none()));
    assert_eq!(entity.velocity_raw(), [100, -1, 300]);
    assert!(entity.position_raw()[1] > 0);
    assert!(
        matches!(entity.current_behavior_context, RetailRuntimeValue::Known(Some(context))
        if context.active_style().style_address() == 0x4c7150)
    );
}

#[v2k_test_support::retail_test]
fn real_weapon_pair_keeps_opposite_callbacks_components_and_physical_after_terminal() {
    use crate::{
        gameplay_notifications::GameplayNotifications,
        native_actor_capture::pair::{
            resolve_native_captor_active_contacts_with_playing, CaptureFeedbackPolicy,
            NativeCaptorPairOutcome, NativeCaptorPairStage, PlayingPlayerContact,
        },
        player_active_contact::{
            active_pair_body_from_entity, classify_oriented_active_pair_contact,
            OrientedActivePairContactRequest,
        },
        specialized_actor_task_production::SpecializedActorTaskScheduler,
        static_damage::StaticDamageScheduler,
    };
    let mut f = Fixture::new();
    let rocket = f
        .manager
        .construct_entity_weapon(
            EntityWeaponConstructionRequest {
                kind: EntityWeaponKind::Rocket,
                source_actor_id: f.manager.player().unwrap().id,
                position_raw: [10_000; 3],
                velocity_raw: [-200, 0, 0],
                rotation_raw: [0; 3],
            },
            &f.session.cache,
            &mut f.fx,
            0,
        )
        .unwrap();
    f.isolate_terminal(&[f.owner.entity_id(), rocket.entity_id()]);
    for (id, velocity) in [
        (f.owner.entity_id(), [100, 0, 0]),
        (rocket.entity_id(), [-200, 0, 0]),
    ] {
        let entity = f.manager.entity_mut(id).unwrap();
        entity.set_motion_raw([10_000; 3], velocity);
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(u32::MAX, 0x0206_8005);
    }
    let mut hit = false;
    'probe: for x in [-100, -50, 0, 50, 100] {
        for y in [-100, -50, 0, 50, 100] {
            f.manager
                .entity_mut(rocket.entity_id())
                .unwrap()
                .set_position_raw([10_000 + x, 10_000 + y, 10_000]);
            let first = f
                .manager
                .iter_all()
                .find(|e| e.id == f.owner.entity_id())
                .unwrap();
            let second = f
                .manager
                .iter_all()
                .find(|e| e.id == rocket.entity_id())
                .unwrap();
            if classify_oriented_active_pair_contact(
                OrientedActivePairContactRequest {
                    subject: first,
                    candidate: second,
                    subject_entry: &active_pair_body_from_entity(first, &f.session.cache),
                    retail_tick: 2,
                },
                &f.session.cache,
            )
            .unwrap()
            .is_some()
            {
                hit = true;
                break 'probe;
            }
        }
    }
    assert!(hit, "authoredmodel128/240musthavea real oriented contact");
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler
        .register_native_weapon(&f.manager, f.owner)
        .unwrap();
    scheduler
        .register_native_weapon(&f.manager, rocket)
        .unwrap();
    let mut damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    let mut hull = crate::player_hull::PlayerHull::new(
        crate::player_hull::HullDamageProfile::from_type_record(
            f.session.cache.global_entity_type(46).unwrap(),
        ),
    );
    hull.readback_entity_damage_state(&f.manager.player().unwrap().collision);
    let outcome = resolve_native_captor_active_contacts_with_playing(
        &mut crate::intro2_contacts::Intro2ContactFrame {
            entities: &mut f.manager,
            resources: &mut f.session.cache,
            world_fx: &mut f.fx,
            static_damage: &mut damage,
            notifications: &mut notifications,
            retail_tick: 2,
            actor_tasks: &mut scheduler,
        },
        f.owner.entity_id(),
        CaptureFeedbackPolicy::Gameplay,
        Some(PlayingPlayerContact {
            hull: &mut hull,
            extra_lives: RetailRuntimeValue::Known(3),
        }),
    );
    let NativeCaptorPairOutcome::Resolved { visits } = outcome else {
        panic!("{outcome:?}");
    };
    let visit = visits
        .iter()
        .find(|v| v.candidate_id == rocket.entity_id())
        .unwrap();
    assert!(!visit.physical_suppressed);
    assert_eq!(
        visit
            .stages
            .iter()
            .filter(|s| matches!(s, NativeCaptorPairStage::Behavior { .. }))
            .count(),
        2
    );
    assert_eq!(
        visit
            .stages
            .iter()
            .filter(|s| matches!(s, NativeCaptorPairStage::Component { .. }))
            .count(),
        6
    );
    assert!(matches!(
        visit.stages.last(),
        Some(NativeCaptorPairStage::Physical { .. })
    ));
    for id in [f.owner.entity_id(), rocket.entity_id()] {
        assert!(crate::class49_death::finished_terminal_hit_authenticates(
            &f.manager, id
        ));
    }
}

#[v2k_test_support::retail_test]
fn native_static_hit_finishes_terminal_then_applies_retained_plane_to_current_body() {
    use crate::{
        gameplay_notifications::GameplayNotifications,
        specialized_actor_task_production::SpecializedActorTaskScheduler,
        static_contact::{
            apply_contact_response_raw, scan_deepest_static_contact, StaticContactQuery,
        },
        static_damage::StaticDamageScheduler,
    };
    let mut f = Fixture::new();
    f.isolate_terminal(&[f.owner.entity_id()]);
    let terrain = f.session.cache.level_terrain_mut().unwrap();
    terrain.header[0] = (-4096i32).wrapping_shl(8);
    for cell in &mut terrain.cells {
        cell.height = 0;
        cell.terrain_type = 0;
    }
    let cell = [10u8, 20u8];
    let attributes = f
        .session
        .cache
        .terrain_objects()
        .unwrap()
        .records
        .iter()
        .enumerate()
        .skip(1)
        .filter_map(|(attribute, descriptor)| {
            f.session
                .cache
                .global_model(usize::from(descriptor.model_id_for(0)))
                .filter(|model| model.collision_radius_raw != 0)
                .map(|_| attribute as u8)
        })
        .collect::<Vec<_>>();
    let mut selected = None;
    'probe: for attribute in attributes {
        f.session.cache.level_terrain_mut().unwrap().cells
            [usize::from(cell[0]) * 256 + usize::from(cell[1])]
        .attribute = attribute;
        for x in [-96, -32, 32, 96] {
            for y in (0..1024).step_by(64) {
                let position = [10 * 256 + 127 + x, y, 20 * 256 + 127];
                f.manager
                    .entity_mut(f.owner.entity_id())
                    .unwrap()
                    .set_position_raw(position);
                let entity = f
                    .manager
                    .iter_all()
                    .find(|e| e.id == f.owner.entity_id())
                    .unwrap();
                let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
                    unreachable!()
                };
                let result = scan_deepest_static_contact(StaticContactQuery {
                    terrain: f.session.cache.level_terrain().unwrap(),
                    terrain_objects: f.session.cache.terrain_objects().unwrap(),
                    model_pool: &f.session.cache,
                    tick: 2,
                    active_model: f.session.cache.global_model(128).unwrap(),
                    active_model_to_world_basis: basis
                        .orientation_world_from_model()
                        .map(|row| row.map(f64::from)),
                    active_anim_vars: &entity.presentation_anim_vars(2),
                    position_raw: position,
                });
                if let Ok(Some(contact)) = result {
                    selected = Some(contact);
                    break 'probe;
                }
            }
        }
    }
    let contact = selected.expect("canonicalAlpinestaticmodel and model128 orientedcontact");
    let velocity = contact
        .normal_q12
        .map(|normal| (i32::from(normal) * -2000 >> 12) as i16);
    f.manager
        .entity_mut(f.owner.entity_id())
        .unwrap()
        .set_velocity_raw(velocity);
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler
        .register_native_weapon(&f.manager, f.owner)
        .unwrap();
    let mut damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    let mut hull = crate::player_hull::PlayerHull::new(
        crate::player_hull::HullDamageProfile::from_type_record(
            f.session.cache.global_entity_type(46).unwrap(),
        ),
    );
    hull.readback_entity_damage_state(&f.manager.player().unwrap().collision);
    let outcome = static_objects::resolve_entity_weapon_static_contact(
        Class49TerminalFrame {
            entities: &mut f.manager,
            resources: &mut f.session.cache,
            world_fx: &mut f.fx,
            static_damage: &mut damage,
            notifications: &mut notifications,
            retail_tick: 2,
            world: Class49WorldContext::Playing {
                scheduler: &mut scheduler,
                player_hull: &mut hull,
                extra_lives: RetailRuntimeValue::Known(3),
                active_terminal_calls: Vec::new(),
            },
        },
        f.owner,
    )
    .unwrap();
    let static_objects::EntityWeaponStaticOutcome::Applied {
        contact: retained,
        position_before_raw,
        position_after_raw,
        velocity_before_raw,
        velocity_after_raw,
        impact_raw,
        actor_damage,
        ..
    } = outcome
    else {
        panic!("{outcome:?}")
    };
    assert_eq!(retained, contact);
    let mut expected_position = position_before_raw;
    let mut expected_velocity = velocity_before_raw;
    apply_contact_response_raw(&mut expected_position, &mut expected_velocity, retained);
    assert_eq!(
        (position_after_raw, velocity_after_raw),
        (expected_position, expected_velocity)
    );
    assert_eq!(
        impact_raw,
        velocity_delta_impact_raw(velocity_before_raw, velocity_after_raw, 10)
    );
    assert!(impact_raw > 0);
    assert!(
        actor_damage.is_some(),
        "11760 actor delivery follows fresh static-cell delivery"
    );
    assert!(crate::class49_death::finished_terminal_hit_authenticates(
        &f.manager,
        f.owner.entity_id()
    ));
    let entity = f.manager.entity_mut(f.owner.entity_id()).unwrap();
    assert_eq!(
        (entity.position_raw(), entity.velocity_raw()),
        (position_after_raw, velocity_after_raw)
    );
}

#[v2k_test_support::retail_test]
fn full_water_pass_transfers_real_type60_primary_to_scheduler_before_later_contact() {
    use crate::{
        gameplay_notifications::GameplayNotifications,
        specialized_actor_task_production::{
            SpecializedActorTaskFamily, SpecializedActorTaskScheduler, SpecializedActorTaskWorld,
        },
        static_damage::StaticDamageScheduler,
    };
    let mut f = Fixture::new();
    f.isolate_terminal(&[f.owner.entity_id()]);
    f.wet_edge(-1751);
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler
        .register_native_weapon(&f.manager, f.owner)
        .unwrap();
    let mut damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    let mut hull = crate::player_hull::PlayerHull::new(
        crate::player_hull::HullDamageProfile::from_type_record(
            f.session.cache.global_entity_type(46).unwrap(),
        ),
    );
    hull.readback_entity_damage_state(&f.manager.player().unwrap().collision);
    let outcome = resolve_entity_weapon_contacts(
        EntityWeaponContactFrame {
            common: crate::intro2_contacts::Intro2ContactFrame {
                entities: &mut f.manager,
                resources: &mut f.session.cache,
                world_fx: &mut f.fx,
                static_damage: &mut damage,
                notifications: &mut notifications,
                retail_tick: 0,
                actor_tasks: &mut scheduler,
            },
            world: SpecializedActorTaskWorld::Playing {
                player_hull: &mut hull,
                extra_lives: RetailRuntimeValue::Known(3),
            },
        },
        f.owner,
    )
    .unwrap();
    let EntityWeaponContactOutcome::Applied {
        surface:
            EntityWeaponSurfaceOutcome::Applied {
                ring: Some(Type60ConstructionOutcome::ActorLinked(receipt)),
                ..
            },
        active_pairs: crate::native_actor_capture::pair::NativeCaptorPairOutcome::Resolved { .. },
        ..
    } = outcome
    else {
        panic!("{outcome:?}")
    };
    assert_eq!(
        scheduler.family_for(receipt.actor().entity_id),
        Some(SpecializedActorTaskFamily::Type60ExplodingRing)
    );
    assert_eq!(
        receipt.provenance(),
        Type60ConstructionProvenance::HardWaterSevere
    );
}

#[v2k_test_support::retail_test]
fn player_first_weapon_pair_uses_strict_source_grace_and_real_playing_hull() {
    use crate::{
        gameplay_notifications::GameplayNotifications,
        native_actor_capture::pair::{
            resolve_native_captor_active_contacts_with_playing, CaptureFeedbackPolicy,
            NativeCaptorPairOutcome, NativeCaptorPairStage, PlayingPlayerContact,
        },
        player_active_contact::{
            active_pair_body_from_entity, classify_oriented_active_pair_contact,
            OrientedActivePairContactRequest,
        },
        specialized_actor_task_production::SpecializedActorTaskScheduler,
        static_damage::StaticDamageScheduler,
    };
    let mut f = Fixture::new();
    let player_id = f.manager.player().unwrap().id;
    f.isolate_terminal(&[f.owner.entity_id(), player_id]);
    for (id, flags) in [(f.owner.entity_id(), 0x0206_8005), (player_id, 0x0006_8005)] {
        let entity = f.manager.entity_mut(id).unwrap();
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(u32::MAX, flags);
        entity.set_motion_raw([10_000; 3], [0; 3]);
        entity.apply_d720_euler_body_basis();
    }
    f.manager
        .entity_mut(f.owner.entity_id())
        .unwrap()
        .collision
        .recent_relation_elapsed_us_at_0x68 = RetailRuntimeValue::Known(750_000);
    let mut found = false;
    'probe: for x in [-160, -80, 0, 80, 160] {
        for y in [-160, -80, 0, 80, 160] {
            f.manager
                .entity_mut(f.owner.entity_id())
                .unwrap()
                .set_position_raw([10_000 + x, 10_000 + y, 10_000]);
            let subject = f.manager.player().unwrap();
            let candidate = f
                .manager
                .iter_all()
                .find(|e| e.id == f.owner.entity_id())
                .unwrap();
            if classify_oriented_active_pair_contact(
                OrientedActivePairContactRequest {
                    subject,
                    candidate,
                    subject_entry: &active_pair_body_from_entity(subject, &f.session.cache),
                    retail_tick: 2,
                },
                &f.session.cache,
            )
            .unwrap()
            .is_some()
            {
                found = true;
                break 'probe;
            }
        }
    }
    assert!(
        found,
        "actual model41/model128contact at expired source grace"
    );
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler
        .register_native_weapon(&f.manager, f.owner)
        .unwrap();
    let mut damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    let mut hull = crate::player_hull::PlayerHull::default();
    f.manager.sync_player_hull_collision_state(&hull);
    f.manager
        .entity_mut(f.owner.entity_id())
        .unwrap()
        .collision
        .recent_relation_elapsed_us_at_0x68 = RetailRuntimeValue::Known(749_999);
    let suppressed = resolve_native_captor_active_contacts_with_playing(
        &mut crate::intro2_contacts::Intro2ContactFrame {
            entities: &mut f.manager,
            resources: &mut f.session.cache,
            world_fx: &mut f.fx,
            static_damage: &mut damage,
            notifications: &mut notifications,
            retail_tick: 2,
            actor_tasks: &mut scheduler,
        },
        player_id,
        CaptureFeedbackPolicy::Gameplay,
        Some(PlayingPlayerContact {
            hull: &mut hull,
            extra_lives: RetailRuntimeValue::Known(3),
        }),
    );
    assert!(
        matches!(suppressed, NativeCaptorPairOutcome::Resolved { visits } if visits.is_empty())
    );
    assert!(!crate::class49_death::finished_terminal_hit_authenticates(
        &f.manager,
        f.owner.entity_id()
    ));
    f.manager
        .entity_mut(f.owner.entity_id())
        .unwrap()
        .collision
        .recent_relation_elapsed_us_at_0x68 = RetailRuntimeValue::Known(750_000);
    let health_before = hull.health_raw;
    let result = resolve_native_captor_active_contacts_with_playing(
        &mut crate::intro2_contacts::Intro2ContactFrame {
            entities: &mut f.manager,
            resources: &mut f.session.cache,
            world_fx: &mut f.fx,
            static_damage: &mut damage,
            notifications: &mut notifications,
            retail_tick: 2,
            actor_tasks: &mut scheduler,
        },
        player_id,
        CaptureFeedbackPolicy::Gameplay,
        Some(PlayingPlayerContact {
            hull: &mut hull,
            extra_lives: RetailRuntimeValue::Known(3),
        }),
    );
    let NativeCaptorPairOutcome::Resolved { visits } = result else {
        panic!("{result:?}")
    };
    let visit = visits
        .iter()
        .find(|v| v.candidate_id == f.owner.entity_id())
        .unwrap();
    assert_eq!(
        visit
            .stages
            .iter()
            .filter(|s| matches!(s, NativeCaptorPairStage::Behavior { .. }))
            .count(),
        2
    );
    assert!(matches!(
        visit.stages.last(),
        Some(NativeCaptorPairStage::Physical { .. })
    ));
    assert!(crate::class49_death::finished_terminal_hit_authenticates(
        &f.manager,
        f.owner.entity_id()
    ));
    assert!(
        hull.health_raw < health_before,
        "synchronousgrenadeblastreachestheactualplayerhull"
    );
    assert_eq!(
        f.manager.player().unwrap().collision.health_raw,
        RetailRuntimeValue::Known(hull.health_raw)
    );
}
