//! Native Type87 late-11AD0 terrain/water contact custody.
//!
//! The airborne bee applies an empty contact (no solid overlap, no water
//! entry) with zero RNG or mutation, and a shallow forced overlap bounces
//! through the solid response without changing health. The buried-bee oracle
//! pins retail's averaged cell plane, which the former triangle test missed.

use v2k_game::{
    actor_task_owner::ActorTaskSlot,
    entity::{EntityConstructionResources, EntityManager},
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    gameplay_notifications::GameplayNotifications,
    intro2_commands::Intro2Commands,
    intro2_contacts::{resolve_intro2_contacts, Intro2ContactFrame, Intro2ContactReport},
    intro2_flyer_contacts::Intro2FlyerContactOutcome,
    intro2_meteors::plan_meteor_terrain_response,
    session::GameSession,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    static_damage::StaticDamageScheduler,
    terrain_contact::TerrainModelContact,
    world_fx::WorldFx,
};

fn fixture() -> (GameSession, EntityManager, WorldFx) {
    let data = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&data).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(50, 1).unwrap();
    let metadata: Vec<EntityTypeRuntimeMetadata> = session
        .cache
        .global_entity_model_table()
        .iter()
        .copied()
        .enumerate()
        .map(|(id, model_slots)| {
            session
                .cache
                .global_entity_type(id)
                .map(EntityTypeRuntimeMetadata::from_section12)
                .unwrap_or(EntityTypeRuntimeMetadata {
                    model_slots,
                    ..Default::default()
                })
        })
        .collect();
    let mut fx = WorldFx::new();
    let model_extent = |id| session.cache.global_model(id).map(|model| model.radius);
    let mut manager = EntityManager::from_native_intro2_frontend(
        session.cache.level_desc().unwrap(),
        &metadata,
        EntityConstructionResources {
            terrain: session.cache.terrain(),
            terrain_objects: session.cache.terrain_objects(),
            model_extent_raw: Some(&model_extent),
        },
        4793,
        &mut fx,
    )
    .unwrap();
    manager.disable_authored_behavior_components();
    let mut commands = Intro2Commands::from_cache(&session.cache).unwrap();
    for tick in 0..=325u32 {
        commands.present(&mut manager, tick);
    }
    let mut scheduler = SpecializedActorTaskScheduler::default();
    assert_eq!(scheduler.adopt_intro2_flyers(&manager), 2);
    (session, manager, fx)
}

fn bee_id(manager: &EntityManager) -> u32 {
    manager
        .iter_all()
        .find(|e| e.authored_spawn_index == Some(46))
        .unwrap()
        .id
}

fn resolve(
    session: &mut GameSession,
    manager: &mut EntityManager,
    fx: &mut WorldFx,
    damage: &mut StaticDamageScheduler,
    notifications: &mut GameplayNotifications,
    scheduler: &mut SpecializedActorTaskScheduler,
    tick: u32,
    id: u32,
) -> Intro2FlyerContactOutcome {
    let mut outcome = None;
    for report in resolve_intro2_contacts(Intro2ContactFrame {
        entities: manager,
        resources: &mut session.cache,
        world_fx: fx,
        static_damage: damage,
        notifications,
        retail_tick: tick,
        actor_tasks: scheduler,
    }) {
        if let Intro2ContactReport::Flyer { entity_id, result } = report {
            if entity_id == id {
                outcome = Some(result);
            }
        }
    }
    outcome.expect("flyer report")
}

#[v2k_test_support::retail_test]
fn airborne_bee_applies_empty_contact_without_mutation_or_rng() {
    let (mut session, mut manager, mut fx) = fixture();
    let id = bee_id(&manager);
    let mut scheduler = SpecializedActorTaskScheduler::default();
    scheduler.adopt_intro2_flyers(&manager);
    let mut damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    let entity = manager.iter_all().find(|e| e.id == id).unwrap();
    let before_position = entity.position_raw();
    let before_tasks =
        ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_task_state(slot).cloned());
    let [x, y, z] = before_position;
    let floor = session.cache.terrain().unwrap().bilinear_height_raw(x, z);
    assert!(y > floor, "keeper setup needs the airborne bee");
    let pending_before = fx.pending_event_count();
    let (_, _, mut unchanged_fx) = fixture();
    let outcome = resolve(
        &mut session,
        &mut manager,
        &mut fx,
        &mut damage,
        &mut notifications,
        &mut scheduler,
        325,
        id,
    );
    assert!(
        matches!(
            outcome,
            Intro2FlyerContactOutcome::Applied {
                solid_contact: false,
                water_entry: false,
                ..
            }
        ),
        "{outcome:?}"
    );
    assert_eq!(
        fx.pending_event_count(),
        pending_before,
        "a clean miss queues no sounds, particles or rings"
    );
    let entity = manager.iter_all().find(|e| e.id == id).unwrap();
    assert_eq!(entity.position_raw(), before_position);
    assert_eq!(
        ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_task_state(slot).cloned()),
        before_tasks
    );
    assert_eq!(
        std::array::from_fn::<_, 8, _>(|_| fx.next_shared_retail_random_u16()),
        std::array::from_fn::<_, 8, _>(|_| unchanged_fx.next_shared_retail_random_u16()),
        "an empty contact preserves the shared random stream",
    );
}

#[v2k_test_support::retail_test]
fn shallow_overlap_bounces_through_solid_response() {
    let (mut session, mut manager, mut fx) = fixture();
    let id = bee_id(&manager);
    let mut scheduler = SpecializedActorTaskScheduler::default();
    scheduler.adopt_intro2_flyers(&manager);
    let mut damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    let entity = manager.iter_all().find(|e| e.id == id).unwrap();
    let [x, _, z] = entity.position_raw();
    let floor = session.cache.terrain().unwrap().bilinear_height_raw(x, z);
    let before_health = entity.collision.health_raw;
    let before_tasks =
        ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_task_state(slot).cloned());
    let position = [x, floor + 100, z];
    let velocity = [0, -1500, 0];
    let model = session
        .cache
        .global_model(entity.model_index.unwrap())
        .unwrap();
    let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
        panic!("native bee body basis");
    };
    let hit = model
        .collide_terrain_raw_oriented(
            session.cache.terrain().unwrap(),
            position,
            basis
                .orientation_world_from_model()
                .map(|row| row.map(f64::from)),
            &entity.presentation_anim_vars(325),
        )
        .unwrap()
        .expect("shallow terrain fixture overlaps the real bee model");
    let separated = plan_meteor_terrain_response(
        position,
        velocity,
        entity.mass_raw,
        TerrainModelContact {
            normal_q12: hit.normal.map(|n| (n * 4096.0).round() as i16),
            penetration_raw: hit.penetration_raw as i32,
        },
    )
    .position_raw;
    // This fixture tests the solid phase. The restored G static continuation
    // also reaches authored buildings at this forced low pose and correctly
    // invokes task+20. Clear only its exact local Section10 scan attributes;
    // height, material, sea level and the full production dispatch remain.
    let scan_radius = model.collision_radius_raw.wrapping_add(0x0200) as i16;
    assert!(scan_radius > 0);
    let count = (((i32::from(scan_radius) * 2 + 0x01ff) as u32 & 0xff00) as u16) / 0x0100;
    for dx in 0..count {
        for dz in 0..count {
            let cell = [
                ((separated[0]
                    .wrapping_sub(scan_radius)
                    .wrapping_add((dx * 0x0100) as i16) as u16)
                    >> 8) as u8,
                ((separated[2]
                    .wrapping_sub(scan_radius)
                    .wrapping_add((dz * 0x0100) as i16) as u16)
                    >> 8) as u8,
            ];
            session
                .cache
                .take_level_terrain_object_attribute(cell)
                .unwrap();
        }
    }
    manager
        .entity_mut(id)
        .unwrap()
        .set_motion_raw(position, velocity);
    let outcome = resolve(
        &mut session,
        &mut manager,
        &mut fx,
        &mut damage,
        &mut notifications,
        &mut scheduler,
        325,
        id,
    );
    let Intro2FlyerContactOutcome::Applied {
        solid_contact,
        water_entry,
        ..
    } = outcome
    else {
        panic!("shallow overlap must respond: {outcome:?}");
    };
    assert!(solid_contact);
    assert!(!water_entry);
    let entity = manager.iter_all().find(|e| e.id == id).unwrap();
    let [_, y, _] = entity.position_raw();
    assert!(
        y > floor,
        "solid response must separate the bee from the floor"
    );
    assert_eq!(entity.collision.health_raw, before_health);
    assert_eq!(
        ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_task_state(slot).cloned()),
        before_tasks
    );
}

#[v2k_test_support::retail_test]
fn buried_bee_uses_retail_cell_plane_instead_of_missing_triangle_projections() {
    let (mut session, mut manager, mut fx) = fixture();
    let position = [623, -3559, 1920];
    let model = session.cache.global_model(270).unwrap();
    let hit = model
        .collide_terrain_raw_oriented(
            session.cache.terrain().unwrap(),
            position,
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            &v2k_formats::models::AnimVars::default(),
        )
        .unwrap()
        .expect("retail 415160 hits the real bee model in this buried pose");
    // Executing retail 415160 with model270's gate and detail sphere centers
    // gives this same result for both spheres. The old implementation missed.
    assert_eq!(
        hit.normal,
        [247.0 / 4096.0, 3903.0 / 4096.0, -1216.0 / 4096.0]
    );
    assert_eq!(hit.penetration_raw, 572.0);

    let id = bee_id(&manager);
    let health_before = manager.entity_mut(id).unwrap().collision.health_raw;
    let mut scheduler = SpecializedActorTaskScheduler::default();
    scheduler.adopt_intro2_flyers(&manager);
    manager
        .entity_mut(id)
        .unwrap()
        .set_motion_raw(position, [0; 3]);
    let outcome = resolve(
        &mut session,
        &mut manager,
        &mut fx,
        &mut StaticDamageScheduler::new(),
        &mut GameplayNotifications::new(),
        &mut scheduler,
        325,
        id,
    );
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
    let bee = manager.entity_mut(id).unwrap();
    assert!(bee.position_raw()[1] > position[1]);
    assert_eq!(bee.collision.health_raw, health_before);
}
