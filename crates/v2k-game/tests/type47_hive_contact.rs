//! Level-1 gunner/hive separation: native Type47 versus bound Type67.
//!
//! Authored spawn 11 rests ~810 raw units from the hive with a +-1024 Guard
//! retarget, so wanderers can enter the fixed hive body. Retail `FUN_00411AD0`
//! pushes them back out with mass-weighted separation; without that lane a
//! wanderer inside the hive can no longer be hit, blocking the seven-hostile
//! victory gate. These corpus-backed tests pin the lane's admission,
//! eject-inside behavior, and hive-untouched guarantees.

use v2k_game::entity::{
    AuthoredPlayerArrival, AuthoredWorldConstruction, EntityConstructionResources, EntityManager,
};
use v2k_game::entity_behavior::PairContactCallbackPolicy;
use v2k_game::entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue};
use v2k_game::gameplay_notifications::GameplayNotifications;
use v2k_game::hive_controller::objective_hostile_present;
use v2k_game::intro2_contacts::Intro2ContactFrame;
use v2k_game::intro2_type17::pair::{resolve_type17_active_contacts, CaptureFeedbackPolicy};
use v2k_game::native_actor_capture::pair::{NativeCaptorPairOutcome, NativeCaptorPairStage};
use v2k_game::session::GameSession;
use v2k_game::specialized_actor_task_production::SpecializedActorTaskScheduler;
use v2k_game::static_damage::StaticDamageScheduler;
use v2k_game::world_fx::WorldFx;

const ANT_SPAWN: usize = 11;
const HIVE_SPAWN: usize = 24;
/// Hive narrow solid (model 341 sphere `r=512`) plus newant radius (`140`).
const NARROW_SEPARATION_RAW: f64 = 652.0;

/// Live-gameplay construction (`NativeOrdinary`): the same path `main.rs`
/// uses, where Type47 carries its native receipt (the fresh-post-Intro
/// harness uses the legacy production track without one).
fn live_level_one() -> Option<(GameSession, EntityManager)> {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).expect("retail fixture must load");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("retail fixture must load");
    session
        .load_level_by_id(13, 1)
        .expect("retail fixture must load");
    let type_models = session.cache.global_entity_model_table();
    let type_metadata = type_models
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
        .collect::<Vec<_>>();
    let mut world_fx = WorldFx::new();
    let manager = EntityManager::from_authored_world(
        AuthoredWorldConstruction {
            logical_world_index: 1,
            level: session.cache.level_desc()?,
            type_metadata: &type_metadata,
            resources: EntityConstructionResources {
                terrain: session.cache.terrain(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&|id| {
                    session.cache.global_model(id).map(|model| model.radius)
                }),
            },
            player_arrival: Some(AuthoredPlayerArrival {
                position_raw: [19_712, -500, 14_848],
                heading_raw: 0x4000,
            }),
            retail_tick: 4793,
        },
        &mut world_fx,
    )
    .expect("retail fixture must load");
    Some((session, manager))
}

fn entity_id(manager: &EntityManager, spawn: usize) -> u32 {
    manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(spawn))
        .unwrap_or_else(|| panic!("spawn {spawn}"))
        .id
}

fn toroidal_delta(first: i16, second: i16) -> i32 {
    let delta = i32::from(first.wrapping_sub(second));
    if delta > 32767 {
        delta - 65536
    } else if delta < -32768 {
        delta + 65536
    } else {
        delta
    }
}

fn toroidal_distance(first: [i16; 3], second: [i16; 3]) -> f64 {
    let dx = toroidal_delta(first[0], second[0]) as f64;
    let dy = i32::from(first[1].wrapping_sub(second[1])) as f64;
    let dz = toroidal_delta(first[2], second[2]) as f64;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

#[v2k_test_support::retail_test]
fn production_load_admits_the_gunner_hive_pair() {
    let (_session, manager) = live_level_one().expect("retail fixture");
    let ant = manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(ANT_SPAWN))
        .expect("ant spawn 11");
    assert_eq!(ant.entity_type, 47);
    assert!(
        ant.native_type47_construction.is_some(),
        "native receipt selects the separation lane"
    );
    let hive = manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(HIVE_SPAWN))
        .expect("hive spawn 24");
    assert_eq!(hive.entity_type, 67);
    assert!(
        hive.authored_radial_emitter.is_some(),
        "bound emitter selects the separation lane"
    );
    assert_eq!(
        hive.collision.state_flags_at_0x08.masked(0x0800_0000),
        RetailRuntimeValue::Known(0x0800_0000),
        "live hive body is fixed for pair response"
    );
    let RetailRuntimeValue::Known(Some(context)) = hive.current_behavior_context else {
        panic!("bound hive retains its Alien-Hive context");
    };
    assert_eq!(
        context.active_style().pair_contact_callback_policy(),
        PairContactCallbackPolicy::Hive,
        "hive contact hook is null against capability 8"
    );
    let distance = toroidal_distance(ant.position_raw(), hive.position_raw());
    let planar = (toroidal_delta(ant.position_raw()[0], hive.position_raw()[0]) as f64)
        .hypot(toroidal_delta(ant.position_raw()[2], hive.position_raw()[2]) as f64);
    assert!(
        (planar - 809.5).abs() < 2.0,
        "authored spawn 11 hugs the hive broad gate: planar={planar}"
    );
    assert!(
        distance > NARROW_SEPARATION_RAW,
        "birth itself is outside the narrow solid: dist={distance}"
    );
}

#[v2k_test_support::retail_test]
fn birth_overlap_runs_no_visit_and_moves_nothing() {
    let (mut session, mut manager) = live_level_one().expect("retail fixture");
    let ant_id = entity_id(&manager, ANT_SPAWN);
    let hive_id = entity_id(&manager, HIVE_SPAWN);
    let mut tasks = SpecializedActorTaskScheduler::new();
    assert_eq!(tasks.adopt_intro2_type47_guards(&manager), 3);
    let ant_before = manager
        .iter_all()
        .find(|entity| entity.id == ant_id)
        .expect("ant")
        .position_raw();
    let hive_before = manager
        .iter_all()
        .find(|entity| entity.id == hive_id)
        .expect("hive")
        .position_raw();
    let mut world_fx = WorldFx::new();
    let mut static_damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    let outcome = resolve_type17_active_contacts(
        &mut Intro2ContactFrame {
            entities: &mut manager,
            resources: &mut session.cache,
            world_fx: &mut world_fx,
            static_damage: &mut static_damage,
            notifications: &mut notifications,
            retail_tick: 0,
            actor_tasks: &mut tasks,
        },
        ant_id,
        CaptureFeedbackPolicy::Gameplay,
    );
    let NativeCaptorPairOutcome::Resolved { visits } = outcome else {
        panic!("birth pair scan must resolve: {outcome:?}");
    };
    assert!(
        visits.iter().all(|visit| visit.candidate_id != hive_id),
        "narrow miss at birth records no hive visit: {visits:?}"
    );
    assert_eq!(
        manager
            .iter_all()
            .find(|entity| entity.id == ant_id)
            .expect("ant")
            .position_raw(),
        ant_before
    );
    assert_eq!(
        manager
            .iter_all()
            .find(|entity| entity.id == hive_id)
            .expect("hive")
            .position_raw(),
        hive_before
    );
}

#[v2k_test_support::retail_test]
fn wanderer_inside_the_hive_is_ejected_untouched_hive() {
    let (mut session, mut manager) = live_level_one().expect("retail fixture");
    let ant_id = entity_id(&manager, ANT_SPAWN);
    let hive_id = entity_id(&manager, HIVE_SPAWN);
    let hive_pos = manager
        .iter_all()
        .find(|entity| entity.id == hive_id)
        .expect("hive")
        .position_raw();
    let hive_vel = manager
        .iter_all()
        .find(|entity| entity.id == hive_id)
        .expect("hive")
        .velocity_raw();
    let hive_health = manager
        .iter_all()
        .find(|entity| entity.id == hive_id)
        .expect("hive")
        .collision
        .health_raw;
    // Drive the wanderer just off the hive origin: deep inside the narrow
    // solid (dist 100 < 652) with a nonzero delta for the descriptor gate.
    manager.entity_mut(ant_id).expect("ant").set_motion_raw(
        [hive_pos[0].wrapping_add(100), hive_pos[1], hive_pos[2]],
        [0, 0, 0],
    );
    let mut tasks = SpecializedActorTaskScheduler::new();
    assert_eq!(tasks.adopt_intro2_type47_guards(&manager), 3);
    let ant_health_before = manager
        .iter_all()
        .find(|entity| entity.id == ant_id)
        .expect("ant")
        .collision
        .health_raw;
    // Retail separates by current penetration every frame. The shared
    // oriented interpreter is authoritative for the rest position (rough
    // sphere math misplaces the offset collision slots), so pin what retail
    // guarantees: the overlapping visit runs physical separation, the fixed
    // hive is untouched, nobody is damaged, the ant stays a live hostile,
    // and an immediate re-run is clean (converged, not oscillating).
    let mut world_fx = WorldFx::new();
    let mut static_damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    let outcome = resolve_type17_active_contacts(
        &mut Intro2ContactFrame {
            entities: &mut manager,
            resources: &mut session.cache,
            world_fx: &mut world_fx,
            static_damage: &mut static_damage,
            notifications: &mut notifications,
            retail_tick: 0,
            actor_tasks: &mut tasks,
        },
        ant_id,
        CaptureFeedbackPolicy::Gameplay,
    );
    let NativeCaptorPairOutcome::Resolved { visits } = outcome else {
        panic!("overlapping pair must resolve, not block: {outcome:?}");
    };
    let visit = visits
        .iter()
        .find(|visit| visit.candidate_id == hive_id)
        .expect("overlapping hive records a visit");
    assert!(
        !visit.physical_suppressed,
        "null behavior hooks keep retail separation: {visit:?}"
    );
    assert!(
        visit
            .stages
            .iter()
            .any(|stage| matches!(stage, NativeCaptorPairStage::Physical { .. })),
        "separation stage runs: {visit:?}"
    );
    let ant = manager
        .iter_all()
        .find(|entity| entity.id == ant_id)
        .expect("ant");
    let distance = toroidal_distance(ant.position_raw(), hive_pos);
    assert!(
        distance > 100.0,
        "separation displaces the wanderer along the retail normal: dist={distance}"
    );
    // Separation preserves the objective hostile: the ant is pushed out,
    // never destroyed by contact.
    assert_eq!(ant.collision.health_raw, ant_health_before);
    assert_eq!(objective_hostile_present(manager.iter_all()), Ok(true));
    let hive = manager
        .iter_all()
        .find(|entity| entity.id == hive_id)
        .expect("hive");
    assert_eq!(hive.position_raw(), hive_pos, "fixed hive never moves");
    assert_eq!(hive.velocity_raw(), hive_vel);
    assert_eq!(hive.collision.health_raw, hive_health);
    // An immediate re-run is clean: the first pass converged the overlap
    // instead of oscillating around it.
    let mut world_fx = WorldFx::new();
    let mut static_damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    let outcome = resolve_type17_active_contacts(
        &mut Intro2ContactFrame {
            entities: &mut manager,
            resources: &mut session.cache,
            world_fx: &mut world_fx,
            static_damage: &mut static_damage,
            notifications: &mut notifications,
            retail_tick: 0,
            actor_tasks: &mut tasks,
        },
        ant_id,
        CaptureFeedbackPolicy::Gameplay,
    );
    let NativeCaptorPairOutcome::Resolved { visits } = outcome else {
        panic!("settled pair must resolve: {outcome:?}");
    };
    assert!(
        visits.iter().all(|visit| visit.candidate_id != hive_id),
        "settled overlap records no further hive visit: {visits:?}"
    );
}
