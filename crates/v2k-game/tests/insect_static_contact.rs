//! Real Intro2 beetles and ordinary stag births against authored static geometry.
use v2k_game::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    entity::{
        AuthoredWorldConstruction, EntityConstructionResources, EntityManager, Intro2BirthSelection,
    },
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    gameplay_notifications::GameplayNotifications,
    intro2_contacts::Intro2ContactFrame,
    native_ground_actor::contact::{
        resolve_flying_static_contact, resolve_insect_static_contact, NativeGroundContactOutcome,
    },
    session::GameSession,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    static_contact::{
        apply_contact_response_raw, scan_deepest_static_contact, StaticContactQuery,
        StaticModelContact,
    },
    static_damage::StaticDamageScheduler,
    static_kind_catalog::static_kind_descriptor,
    world_fx::WorldFx,
};

fn session(level: u32) -> (GameSession, Vec<EntityTypeRuntimeMetadata>) {
    let data = v2k_test_support::retail_dir();
    assert!(data.join("PRELOAD.DAT").is_file(), "retail corpus required");
    let mut session = GameSession::init(&data).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(level, 1).unwrap();
    let metadata = session
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
    (session, metadata)
}

fn overlap(
    session: &GameSession,
    manager: &EntityManager,
    id: u32,
    wanted_kind: Option<u32>,
) -> ([i16; 3], StaticModelContact, u32, u16) {
    let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
    let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
        panic!("native body basis");
    };
    let model = session
        .cache
        .global_model(entity.model_index.unwrap())
        .unwrap();
    let terrain = session.cache.terrain().unwrap();
    let objects = session.cache.terrain_objects().unwrap();
    // Authored Section10 props. Live huts/factories are covered independently
    // by the active-pair regression; these cells exercise the shared static lane.
    for x in 0..256usize {
        for z in 0..256usize {
            let cell = terrain.cell(x, z).unwrap();
            if cell.attribute == 0 || cell.terrain_type & 8 != 0 {
                continue;
            }
            let object = &objects.records[usize::from(cell.attribute)];
            if !matches!(object.kind_index, 0 | 1 | 2 | 3 | 4 | 10) {
                continue;
            }
            if wanted_kind.is_some_and(|kind| kind != object.kind_index) {
                continue;
            }
            let cx = ((x as u16) << 8).wrapping_add(128) as i16;
            let cz = ((z as u16) << 8).wrapping_add(128) as i16;
            for dx in (-256..=256).step_by(64) {
                for dz in (-256..=256).step_by(64) {
                    let px = cx.wrapping_add(dx as i16);
                    let pz = cz.wrapping_add(dz as i16);
                    let position = [px, terrain.bilinear_height_raw(px, pz).wrapping_add(75), pz];
                    let contact = scan_deepest_static_contact(StaticContactQuery {
                        terrain,
                        terrain_objects: objects,
                        model_pool: &session.cache,
                        tick: 1300,
                        active_model: model,
                        active_model_to_world_basis: basis
                            .orientation_world_from_model()
                            .map(|row| row.map(f64::from)),
                        active_anim_vars: &entity.presentation_anim_vars(1300),
                        position_raw: position,
                    })
                    .unwrap();
                    if let Some(contact) = contact
                        .filter(|contact| contact.response_raw < 0 && contact.penetration_raw > 0)
                    {
                        let current = terrain
                            .cell(usize::from(contact.cell[0]), usize::from(contact.cell[1]))
                            .unwrap();
                        let selected = &objects.records[usize::from(current.attribute)];
                        if wanted_kind.is_some_and(|kind| kind != selected.kind_index) {
                            continue;
                        }
                        return (
                            position,
                            contact,
                            selected.kind_index,
                            selected.model_id_for(current.terrain_type),
                        );
                    }
                }
            }
        }
    }
    panic!(
        "no authored static overlap for type{} model{} kind{wanted_kind:?}",
        entity.entity_type,
        entity.model_index.unwrap()
    );
}

fn resolve(
    session: &mut GameSession,
    manager: &mut EntityManager,
    id: u32,
    scheduler: &mut SpecializedActorTaskScheduler,
    fx: &mut WorldFx,
    damage: &mut StaticDamageScheduler,
    notifications: &mut GameplayNotifications,
) -> NativeGroundContactOutcome {
    let kind = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .unwrap()
        .entity_type;
    let mut frame = Intro2ContactFrame {
        entities: manager,
        resources: &mut session.cache,
        world_fx: fx,
        static_damage: damage,
        notifications,
        retail_tick: 1300,
        actor_tasks: scheduler,
    };
    if matches!(kind, 16 | 26 | 94) {
        resolve_insect_static_contact(&mut frame, id)
    } else {
        resolve_flying_static_contact(&mut frame, id)
    }
}

fn exercise(session: GameSession, manager: EntityManager, id: u32, fx: WorldFx) {
    exercise_kind(session, manager, id, fx, None);
}

fn exercise_kind(
    mut session: GameSession,
    mut manager: EntityManager,
    id: u32,
    mut fx: WorldFx,
    wanted_kind: Option<u32>,
) {
    let (position, contact, kind, static_model) = overlap(&session, &manager, id, wanted_kind);
    let static_name = session
        .cache
        .global_model(usize::from(static_model))
        .unwrap()
        .name
        .clone();
    let entity = manager.entity_mut(id).unwrap();
    // Preserve native constructor, graph and incoming orientation. Controlled
    // velocity drives the native separating plane rather than a guessed push.
    let velocity = contact
        .normal_q12
        .map(|n| (-i32::from(n) * 256 >> 12) as i16);
    entity.set_motion_raw(position, velocity);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(0x8000, 0x8000);
    entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
    let actor_type = entity.entity_type;
    let basis = entity.physical_body_basis_q31();
    let sub_g_before = entity.sub_g_06070_runtime;
    let sub_h_before = entity.sub_h_external_frame_runtime.clone();
    let RetailRuntimeValue::Known(default_flags) = entity.collision.default_state_flags_at_0xc8
    else {
        panic!("native policy");
    };
    let heading = entity.rotation_heading_pitch_roll_raw()[0];
    let other_tasks = [ActorTaskSlot::Secondary, ActorTaskSlot::Tertiary]
        .map(|slot| entity.actor_task_state(slot).copied());
    let primary_before = entity.actor_task_state(ActorTaskSlot::Primary).copied();
    let mut scheduler = SpecializedActorTaskScheduler::default();
    let mut damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    let before = manager.entity_mut(id).unwrap().collision.clone();
    let rejected = resolve(
        &mut session,
        &mut manager,
        id,
        &mut scheduler,
        &mut fx,
        &mut damage,
        &mut notifications,
    );
    assert!(
        matches!(
            rejected,
            NativeGroundContactOutcome::Blocked {
                committed_prefix: false,
                ..
            }
        ),
        "{rejected:?}"
    );
    assert_eq!(manager.entity_mut(id).unwrap().position_raw(), position);
    assert_eq!(manager.entity_mut(id).unwrap().collision, before);
    assert!(
        match actor_type {
            16 => scheduler.adopt_intro2_type16(&manager),
            26 => scheduler.adopt_intro2_type26(&manager),
            94 => scheduler.adopt_intro2_type94(&manager),
            13 => scheduler.adopt_intro2_type13_search_attack(&manager),
            10 => scheduler.adopt_intro2_type10(&manager),
            57 => scheduler.adopt_intro2_type57(&manager),
            15 | 87 => scheduler.adopt_intro2_flyers(&manager),
            _ => unreachable!(),
        } > 0
    );
    let result = resolve(
        &mut session,
        &mut manager,
        id,
        &mut scheduler,
        &mut fx,
        &mut damage,
        &mut notifications,
    );
    let NativeGroundContactOutcome::Applied(applied) = result else {
        panic!("type{actor_type} static kind{kind}/model{static_model}: {result:?}");
    };
    assert_eq!(applied.contact, contact);
    let mut expected_position = position;
    let mut expected_velocity = velocity;
    apply_contact_response_raw(&mut expected_position, &mut expected_velocity, contact);
    assert_eq!(applied.position_after_raw, expected_position);
    assert_eq!(applied.velocity_after_raw, expected_velocity);
    assert_ne!(
        expected_position, position,
        "native separation must be observable"
    );
    assert_eq!(
        applied.crushing_damage.is_some(),
        default_flags & 0x400 != 0,
        "each native policy retains its own D9B0 eligibility"
    );
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(entity.physical_body_basis_q31(), basis);
    assert_eq!(entity.rotation_heading_pitch_roll_raw()[0], heading);
    assert!(applied.impact_raw > 0);
    if actor_type == 94 {
        let RetailRuntimeValue::Known(Some(before_h)) = &sub_h_before else {
            panic!("native Type94 H");
        };
        let RetailRuntimeValue::Known(Some(after_h)) = &entity.sub_h_external_frame_runtime else {
            panic!("retained Type94 H");
        };
        assert_eq!(before_h.records().len(), 6);
        assert_eq!(
            after_h.records(),
            before_h.records(),
            "static contact does not advance the six feet"
        );
        assert_eq!(after_h.cursor(), before_h.cursor());
        assert_eq!(
            after_h.surface_policy(),
            v2k_game::sub_h_external_frame::SubHSurfacePolicy::TerrainAndWater
        );
        if applied
            .actor_damage
            .as_ref()
            .is_none_or(|damage| damage.death_publication.is_none())
        {
            assert_eq!(
                entity.sub_h_external_frame_runtime, sub_h_before,
                "living static contact retains enabled water H"
            );
        } else {
            assert!(
                !after_h.is_enabled(),
                "genuine Class12 disables H while retaining water policy"
            );
        }
    }
    // Static02CA0 writes the private direction/timer, not task age or Sub-D.
    match (
        primary_before,
        entity.actor_task_state(ActorTaskSlot::Primary),
    ) {
        (
            Some(ActorTaskRuntime::DefecateVirusWander(before)),
            Some(ActorTaskRuntime::DefecateVirusWander(after)),
        ) => {
            assert_eq!(after.elapsed_ms(), before.elapsed_ms());
            assert_eq!(
                after.private_state().direction,
                -before.private_state().direction
            );
            assert_eq!(after.private_state().reversal_timer_ms, 2500);
        }
        (
            Some(ActorTaskRuntime::SharedRetarget(before)),
            Some(ActorTaskRuntime::SharedRetarget(after)),
        ) => {
            assert_eq!(after.elapsed_ms(), before.elapsed_ms());
            assert_eq!(
                after.private_state().direction,
                -before.private_state().direction
            );
            assert_eq!(after.private_state().reversal_timer_ms, 2500);
            if let (
                RetailRuntimeValue::Known(Some(before_g)),
                RetailRuntimeValue::Known(Some(after_g)),
            ) = (sub_g_before, entity.sub_g_06070_runtime)
            {
                assert_eq!(
                    after_g.invert_target_at_0x3c(),
                    RetailRuntimeValue::Known(u8::from(after.private_state().direction == -1))
                );
                assert_eq!(
                    after_g.randomized_target_raw_at_0x38(),
                    before_g.randomized_target_raw_at_0x38()
                );
                assert_eq!(after_g.rate_raw_at_0x34(), before_g.rate_raw_at_0x34());
                assert_eq!(after_g.mode_at_0x3f(), before_g.mode_at_0x3f());
                assert_eq!(after_g.mode_at_0x40(), before_g.mode_at_0x40());
                assert_eq!(
                    after_g.animation_outputs_raw(),
                    before_g.animation_outputs_raw()
                );
            }
        }
        (Some(ActorTaskRuntime::TrashFurniture(_)), _) => assert!(
            applied.furniture_damage.is_some(),
            "C890 must survive generic crush/filter results"
        ),
        _ => {}
    }
    if applied
        .actor_damage
        .as_ref()
        .is_some_and(|damage| damage.death_publication.is_some())
    {
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::CommonDying(_))
        ));
        assert_eq!(
            [ActorTaskSlot::Secondary, ActorTaskSlot::Tertiary]
                .map(|slot| entity.actor_task_state(slot).copied()),
            [None, None]
        );
    } else if applied.furniture_damage.is_none() {
        assert_eq!(
            [ActorTaskSlot::Secondary, ActorTaskSlot::Tertiary]
                .map(|slot| entity.actor_task_state(slot).copied()),
            other_tasks
        );
    }
    if !matches!(actor_type, 15 | 87) {
        assert!(scheduler.park_native_contact_prefix(&manager, id));
        let stopped = resolve(
            &mut session,
            &mut manager,
            id,
            &mut scheduler,
            &mut fx,
            &mut damage,
            &mut notifications,
        );
        assert!(
            matches!(
                stopped,
                NativeGroundContactOutcome::Blocked {
                    committed_prefix: false,
                    ..
                } | NativeGroundContactOutcome::Miss
            ),
            "{stopped:?}"
        );
    }
    eprintln!("type{actor_type} static kind{kind} model{static_model}/{static_name:?} cell{:?}: {position:?}->{expected_position:?}", contact.cell);
}

#[v2k_test_support::retail_test]
fn intro2_authored_beetles_and_stags_use_native_static_contact() {
    for spawn in [5, 42, 10, 25] {
        let (session, metadata) = session(50);
        let manager = EntityManager::from_captured_intro2_frontend_with_type_metadata(
            session.cache.level_desc().unwrap(),
            &metadata,
            EntityConstructionResources {
                terrain: session.cache.terrain(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&|id| {
                    session.cache.global_model(id).map(|model| model.radius)
                }),
            },
            Intro2BirthSelection::default(),
            &mut || 1,
        )
        .unwrap();
        let id = manager
            .iter_all()
            .find(|e| e.authored_spawn_index == Some(spawn))
            .unwrap()
            .id;
        exercise(session, manager, id, WorldFx::new());
    }
}

#[v2k_test_support::retail_test]
fn intro2_native_water_actor_static_contact_preserves_six_feet_and_water_policy() {
    let (session, metadata) = session(50);
    let manager = EntityManager::from_captured_intro2_frontend_with_type_metadata(
        session.cache.level_desc().unwrap(),
        &metadata,
        EntityConstructionResources {
            terrain: session.cache.terrain(),
            terrain_objects: session.cache.terrain_objects(),
            model_extent_raw: Some(&|id| session.cache.global_model(id).map(|model| model.radius)),
        },
        Intro2BirthSelection::default(),
        &mut || 1,
    )
    .unwrap();
    let id = manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(43))
        .unwrap()
        .id;
    let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
    assert!(entity.intro2_type94_runtime.is_some());
    let RetailRuntimeValue::Known(Some(c)) = metadata[94].sub_c_lift_descriptor else {
        panic!("water Sub-C");
    };
    assert_eq!(c.surface_mode_raw, 1);
    assert_eq!(c.base_clearance_raw, 50);
    exercise(session, manager, id, WorldFx::new());
}

#[v2k_test_support::retail_test]
fn type94_model_and_metadata_cannot_replace_its_native_allocation_receipt() {
    let (mut session, metadata) = session(50);
    let mut manager = EntityManager::from_captured_intro2_frontend_with_type_metadata(
        session.cache.level_desc().unwrap(),
        &metadata,
        EntityConstructionResources {
            terrain: session.cache.terrain(),
            terrain_objects: session.cache.terrain_objects(),
            model_extent_raw: Some(&|id| session.cache.global_model(id).map(|model| model.radius)),
        },
        Intro2BirthSelection::default(),
        &mut || 1,
    )
    .unwrap();
    let id = manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(43))
        .unwrap()
        .id;
    let entity = manager.entity_mut(id).unwrap();
    let position = entity.position_raw();
    let h = entity.sub_h_external_frame_runtime.clone();
    entity.intro2_type94_runtime = None;
    let mut scheduler = SpecializedActorTaskScheduler::new();
    let mut fx = WorldFx::new();
    let mut damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    let result = resolve(
        &mut session,
        &mut manager,
        id,
        &mut scheduler,
        &mut fx,
        &mut damage,
        &mut notifications,
    );
    assert_eq!(result, NativeGroundContactOutcome::Ineligible);
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(entity.position_raw(), position);
    assert_eq!(entity.sub_h_external_frame_runtime, h);
}

#[v2k_test_support::retail_test]
fn intro2_native_flying_insects_share_static_geometry_with_distinct_policy() {
    for spawn in [0, 1, 44, 46, 55, 56] {
        let (session, metadata) = session(50);
        let manager = EntityManager::from_captured_intro2_frontend_with_type_metadata(
            session.cache.level_desc().unwrap(),
            &metadata,
            EntityConstructionResources {
                terrain: session.cache.terrain(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&|id| {
                    session.cache.global_model(id).map(|model| model.radius)
                }),
            },
            Intro2BirthSelection::default(),
            &mut || 1,
        )
        .unwrap();
        let id = manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(spawn))
            .unwrap()
            .id;
        exercise(session, manager, id, WorldFx::new());
    }
}

#[v2k_test_support::retail_test]
fn authored_additional_static_kinds_share_the_same_geometry_and_callback_path() {
    let (catalog_session, _) = session(50);
    let terrain = catalog_session.cache.terrain().unwrap();
    let objects = catalog_session.cache.terrain_objects().unwrap();
    let mut kinds = std::collections::BTreeSet::new();
    for x in 0..256usize {
        for z in 0..256usize {
            let cell = terrain.cell(x, z).unwrap();
            if cell.attribute == 0 || cell.terrain_type & 8 != 0 {
                continue;
            }
            let kind = objects.records[usize::from(cell.attribute)].kind_index;
            // Kind1's retail response is +1792: it cannot supply the separating
            // contact this test asks for. Select from the exact response table.
            if matches!(kind, 1 | 2 | 3 | 4 | 10)
                && static_kind_descriptor(kind).unwrap().contact_response_raw() < 0
            {
                if kinds.insert(kind) {
                    let model_id = objects.records[usize::from(cell.attribute)]
                        .model_id_for(cell.terrain_type);
                    let model = catalog_session
                        .cache
                        .global_model(usize::from(model_id))
                        .unwrap();
                    eprintln!(
                        "additional authored kind{kind} model{model_id}/{:?} radius{}",
                        model.name, model.radius
                    );
                }
            }
        }
    }
    // The authored windmill and castle are buildings. The shield-up record
    // also has solid geometry, but it does not establish building coverage.
    for kind in [3, 10] {
        assert!(
            kinds.contains(&kind),
            "authored building kind{kind} required, got{kinds:?}"
        );
        for spawn in [10, 46] {
            let (session, metadata) = session(50);
            let manager = EntityManager::from_captured_intro2_frontend_with_type_metadata(
                session.cache.level_desc().unwrap(),
                &metadata,
                EntityConstructionResources {
                    terrain: session.cache.terrain(),
                    terrain_objects: session.cache.terrain_objects(),
                    model_extent_raw: Some(&|id| {
                        session.cache.global_model(id).map(|model| model.radius)
                    }),
                },
                Intro2BirthSelection::default(),
                &mut || 1,
            )
            .unwrap();
            let id = manager
                .iter_all()
                .find(|entity| entity.authored_spawn_index == Some(spawn))
                .unwrap()
                .id;
            exercise_kind(session, manager, id, WorldFx::new(), Some(kind));
        }
    }
}

#[v2k_test_support::retail_test]
fn ordinary_authored_stags_share_the_static_rule_across_worlds() {
    let mut checked = 0;
    for level in 13..=49 {
        let (session, metadata) = session(level);
        if !session
            .cache
            .level_desc()
            .unwrap()
            .entities
            .iter()
            .any(|e| e.entity_type == 26)
        {
            continue;
        }
        let mut fx = WorldFx::new();
        let manager = EntityManager::from_authored_world(
            AuthoredWorldConstruction {
                level: session.cache.level_desc().unwrap(),
                logical_world_index: level as i32 - 12,
                type_metadata: &metadata,
                resources: EntityConstructionResources {
                    terrain: session.cache.terrain(),
                    terrain_objects: session.cache.terrain_objects(),
                    model_extent_raw: Some(&|id| session.cache.global_model(id).map(|m| m.radius)),
                },
                player_arrival: None,
                retail_tick: 1300,
            },
            &mut fx,
        )
        .unwrap();
        let id = manager.iter_all().find(|e| e.entity_type == 26).unwrap().id;
        exercise(session, manager, id, fx);
        checked += 1;
        if checked == 2 {
            break;
        }
    }
    assert_eq!(checked, 2, "two authored ordinary worlds required");
}

#[v2k_test_support::retail_test]
fn living_ground_insects_keep_native_surface_exclusion_but_common12_separates() {
    use v2k_game::intro2_common_dying::publish_intro2_common_standard_death;
    use v2k_game::native_actor_surface_contact::{
        resolve_native_actor_surface_contact, NativeActorSurfaceContactOutcome,
    };
    for spawn in [5, 10] {
        let (mut session, metadata) = session(50);
        let mut manager = EntityManager::from_captured_intro2_frontend_with_type_metadata(
            session.cache.level_desc().unwrap(),
            &metadata,
            EntityConstructionResources {
                terrain: session.cache.terrain(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&|id| {
                    session.cache.global_model(id).map(|model| model.radius)
                }),
            },
            Intro2BirthSelection::default(),
            &mut || 1,
        )
        .unwrap();
        let id = manager
            .iter_all()
            .find(|e| e.authored_spawn_index == Some(spawn))
            .unwrap()
            .id;
        let entity = manager.entity_mut(id).unwrap();
        let [x, _, z] = entity.position_raw();
        let y = session
            .cache
            .terrain()
            .unwrap()
            .bilinear_height_raw(x, z)
            .wrapping_sub(1000);
        entity.set_motion_raw([x, y, z], [0, -512, 0]);
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(0x8000, 0x8000);
        entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
        assert_eq!(
            entity.collision.state_flags_at_0x08.masked(0x10000),
            RetailRuntimeValue::Known(0)
        );
        let mut fx = WorldFx::new();
        let mut scheduler = SpecializedActorTaskScheduler::new();
        let mut damage = StaticDamageScheduler::new();
        let mut notifications = GameplayNotifications::new();
        let living = resolve_native_actor_surface_contact(
            &mut Intro2ContactFrame {
                entities: &mut manager,
                resources: &mut session.cache,
                world_fx: &mut fx,
                static_damage: &mut damage,
                notifications: &mut notifications,
                retail_tick: 1300,
                actor_tasks: &mut scheduler,
            },
            id,
        );
        assert_eq!(living, NativeActorSurfaceContactOutcome::Ineligible);
        assert_eq!(manager.entity_mut(id).unwrap().position_raw(), [x, y, z]);
        // Actual10C10/C620 publication, not a fabricated style or task receipt.
        let owner = publish_intro2_common_standard_death(&mut manager, id, &mut fx)
            .unwrap()
            .unwrap();
        scheduler.register_intro2_common_dying(owner);
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(
            entity.collision.state_flags_at_0x08.masked(0x10000),
            RetailRuntimeValue::Known(0x10000)
        );
        entity.set_motion_raw([x, y, z], [0, -512, 0]);
        let dead = resolve_native_actor_surface_contact(
            &mut Intro2ContactFrame {
                entities: &mut manager,
                resources: &mut session.cache,
                world_fx: &mut fx,
                static_damage: &mut damage,
                notifications: &mut notifications,
                retail_tick: 1300,
                actor_tasks: &mut scheduler,
            },
            id,
        );
        assert!(
            matches!(
                dead,
                NativeActorSurfaceContactOutcome::Applied {
                    solid_contact: true,
                    ..
                }
            ),
            "spawn{spawn}: {dead:?}"
        );
        assert!(manager.entity_mut(id).unwrap().position_raw()[1] > y);
    }
}

#[v2k_test_support::retail_test]
fn lethal_type13_static_contact_finishes_its_real_class1_explosion() {
    let (mut session, metadata) = session(50);
    let mut manager = EntityManager::from_captured_intro2_frontend_with_type_metadata(
        session.cache.level_desc().unwrap(),
        &metadata,
        EntityConstructionResources {
            terrain: session.cache.terrain(),
            terrain_objects: session.cache.terrain_objects(),
            model_extent_raw: Some(&|id| session.cache.global_model(id).map(|model| model.radius)),
        },
        Intro2BirthSelection::default(),
        &mut || 1,
    )
    .unwrap();
    let id = manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(0))
        .unwrap()
        .id;
    let (position, contact, _, _) = overlap(&session, &manager, id, Some(0));
    let ids: Vec<_> = manager.iter_all().map(|entity| entity.id).collect();
    for other in ids {
        manager
            .entity_mut(other)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .overwrite(0x8000, if other == id { 0x8000 } else { 0 });
    }
    let velocity = contact
        .normal_q12
        .map(|n| (-i32::from(n) * 12_000 >> 12) as i16);
    let entity = manager.entity_mut(id).unwrap();
    entity.set_motion_raw(position, velocity);
    entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
    let mut predicted_position = position;
    let mut predicted_velocity = velocity;
    apply_contact_response_raw(&mut predicted_position, &mut predicted_velocity, contact);
    let predicted_impact =
        v2k_game::damage::velocity_delta_impact_raw(velocity, predicted_velocity, entity.mass_raw);
    let RetailRuntimeValue::Known(profile) = entity.collision.damage_profile else {
        panic!("authored damage profile");
    };
    let predicted_damage =
        profile.filter(v2k_game::damage::DamagePacket::collision(predicted_impact));
    assert!(predicted_damage > 0, "selected inward collision must pass the actual channel-1 filter: impact{predicted_impact}, profile{profile:?}");
    // A controlled injury exercises native collision damage and the real death
    // owner; no alternate task or terminal receipt is fabricated.
    entity.collision.health_raw = RetailRuntimeValue::Known(1);
    entity.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(0);
    let ring_count = manager
        .iter_all()
        .filter(|entity| entity.entity_type == 60)
        .count();
    let mut scheduler = SpecializedActorTaskScheduler::new();
    assert!(scheduler.adopt_intro2_type13_search_attack(&manager) > 0);
    let mut fx = WorldFx::new();
    let mut damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    let result = resolve(
        &mut session,
        &mut manager,
        id,
        &mut scheduler,
        &mut fx,
        &mut damage,
        &mut notifications,
    );
    let NativeGroundContactOutcome::Applied(applied) = result else {
        panic!("real Class1 collision: {result:?}");
    };
    assert!(applied.actor_damage.as_ref().unwrap().damage_after_buffer_raw > 0, "native11760 failed to deliver predicted impact{predicted_impact}/damage{predicted_damage}: {applied:?}");
    assert!(
        applied
            .actor_damage
            .as_ref()
            .unwrap()
            .death_publication
            .is_none(),
        "Class1 leaves no fabricated CommonDying owner"
    );
    let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
    assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        panic!("Class1 style");
    };
    assert_eq!(context.active_style().style_address(), 0x4c7150);
    assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
        .into_iter()
        .all(|slot| entity.actor_task_state(slot).is_none()));
    let finished = entity
        .class49_death_runtime
        .expect("actual shared Class1 terminal receipt");
    assert_eq!(manager.pending_actor_deferred_destroy_ids(), [id]);
    assert_eq!(
        manager
            .iter_all()
            .filter(|entity| entity.entity_type == 60)
            .count(),
        ring_count,
        "BAC0 does not construct BD20's ring"
    );
    assert!(
        fx.particle_count() + fx.pending_event_count() > 0,
        "BAF0 must create its authored effects"
    );
    manager
        .entity_mut(id)
        .unwrap()
        .set_motion_raw(position, velocity);
    let repeated = resolve(
        &mut session,
        &mut manager,
        id,
        &mut scheduler,
        &mut fx,
        &mut damage,
        &mut notifications,
    );
    assert!(
        matches!(
            repeated,
            NativeGroundContactOutcome::Applied(_) | NativeGroundContactOutcome::Miss
        ),
        "{repeated:?}"
    );
    assert_eq!(
        manager
            .iter_all()
            .find(|entity| entity.id == id)
            .unwrap()
            .class49_death_runtime,
        Some(finished)
    );
    assert_eq!(
        manager.pending_actor_deferred_destroy_ids(),
        [id],
        "terminal publication cannot replay"
    );
}
