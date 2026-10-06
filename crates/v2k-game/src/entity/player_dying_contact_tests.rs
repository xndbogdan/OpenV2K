//! Native Level1 constructor/contact ordering, with actual retained geometry.
use super::player_environment_tests::{native, request, session};
use super::*;
use crate::gameplay_notifications::GameplayNotifications;
use crate::player_contact_style::*;
use crate::player_surface_contact::{PlayerSurfaceContactFrame, PlayerSurfaceContactRequest};
use crate::terrain_contact::{
    begin_player_terrain_contact, PlayerTerrainStyleCallback, PlayerTerrainStyleCallbackOutcome,
};
use v2k_formats::terrain::{TerrainCell, GRID_SIZE};

fn rng(fx: &mut WorldFx) -> u32 {
    *fx.entity_construction_state().0
}

fn terrain(height: i8) -> TerrainGrid {
    TerrainGrid {
        header: [0; 5],
        cells: vec![
            TerrainCell {
                height: height as u8,
                attribute: 0,
                terrain_type: 0
            };
            GRID_SIZE * GRID_SIZE
        ],
    }
}

fn die(
    manager: &mut EntityManager,
    hull: &mut PlayerHull,
    fx: &mut WorldFx,
    cache: &crate::resource_cache::ResourceCache,
    tick: u32,
) {
    hull.health_raw = 0;
    hull.dying = true;
    manager.sync_player_hull_collision_state(hull);
    manager
        .begin_player_dying(PlayerCheckedDamageFrame {
            hull,
            resources: cache,
            world_fx: fx,
            retail_tick: tick,
            notifications: &mut GameplayNotifications::new(),
            extra_lives: 3,
        })
        .unwrap_or_else(|block| {
            panic!(
                "native dying {block:?}, context={:?}",
                manager.player().unwrap().current_behavior_context
            )
        });
}

fn style(manager: &EntityManager) -> PlayerContactStyleRequest {
    let player = manager.player().unwrap();
    PlayerContactStyleRequest {
        behavior_context: player.current_behavior_context,
        entity_handle: player.id,
        controlled_entity_handle: Some(player.id),
        haptic_scale_raw: 15,
    }
}

#[v2k_test_support::retail_test]
fn level1_dying_terrain_uses_real_entry_model_and_hook_velocity_for_the_physical_tail() {
    let session = session(13);
    let mut fx = WorldFx::new();
    let mut manager = native(&session, 13, &mut fx);
    let entry_model_id = manager.player().unwrap().model_index.unwrap();
    let entry_model = session.cache.global_model(entry_model_id).unwrap();
    let mut hull = PlayerHull::default();
    die(&mut manager, &mut hull, &mut fx, &session.cache, 100);
    assert_ne!(manager.player().unwrap().model_index, Some(entry_model_id));
    let ground = terrain(0);
    let mut position = [100, 0, 100];
    let mut velocity = [321, -2048, -456];
    let mut runtime = manager.player_dying_contact_runtime();
    let tick = runtime.unwrap().next_burst_tick() as u32;
    let rng_before = rng(&mut fx);
    let continuation = begin_player_terrain_contact(
        &ground,
        entry_model,
        [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        &Default::default(),
        PlayerTerrainStyleCallback::DyingBounce(PlayerDyingSurfaceContactFrame {
            request: style(&manager),
            runtime: &mut runtime,
            world_fx: &mut fx,
            source_extent_raw: session
                .cache
                .global_model(manager.player().unwrap().model_index.unwrap())
                .unwrap()
                .radius
                >> 1,
            sea_level_raw: Some(ground.sea_level_raw()),
            retail_tick: tick,
        }),
        &mut position,
        &mut velocity,
        &mut hull,
    )
    .unwrap()
    .expect("native player spheres meet bare ground");
    assert_eq!(velocity, [321, 1000, -456]);
    let PlayerTerrainStyleCallbackOutcome::DyingBounce(callback) = continuation.style_callback()
    else {
        panic!("class25 +10 owner")
    };
    assert!(
        !callback.wreck_burst_emitted,
        "equal signed latch does not emit"
    );
    assert_eq!(callback.unsupported_haptic.unwrap().strength_raw, 255);
    let outcome = continuation.finish(&mut position, &mut velocity, &mut hull);
    assert_eq!(outcome.velocity_before_raw, [321, -2048, -456]);
    assert_eq!(outcome.velocity_after_raw, [321, 1000, -456]);
    assert_eq!(
        outcome.collision_impact_raw, 0,
        "141D0 reads VY after AF0, not pre-hook downward speed"
    );
    assert_eq!(rng_before, rng(&mut fx));
}

#[v2k_test_support::retail_test]
fn dying_surface_strict_latch_reseed_controller_mismatch_and_blocked_prefix() {
    let session = session(13);
    let mut fx = WorldFx::new();
    let mut manager = native(&session, 13, &mut fx);
    let mut hull = PlayerHull::default();
    die(&mut manager, &mut hull, &mut fx, &session.cache, 100);
    let mut runtime = manager.player_dying_contact_runtime();
    let latch = runtime.unwrap().next_burst_tick();
    let extent = session
        .cache
        .global_model(manager.player().unwrap().model_index.unwrap())
        .unwrap()
        .radius;
    let request = style(&manager);
    let mut velocity = [7, -513, 9];
    let before = rng(&mut fx);
    let first = apply_player_dying_surface_contact(
        PlayerDyingSurfaceContactFrame {
            request,
            runtime: &mut runtime,
            world_fx: &mut fx,
            source_extent_raw: extent,
            sea_level_raw: Some(-100),
            retail_tick: latch.wrapping_add(1) as u32,
        },
        [11, 12, 13],
        [0, 4096, 0],
        &mut velocity,
    )
    .unwrap();
    assert!(first.wreck_burst_emitted);
    assert_eq!(first.unsupported_haptic.unwrap().strength_raw, 1);
    assert_eq!(velocity, [7, 1000, 9]);
    assert_ne!(before, rng(&mut fx));
    let after = rng(&mut fx);
    let second = apply_player_dying_surface_contact(
        PlayerDyingSurfaceContactFrame {
            request,
            runtime: &mut runtime,
            world_fx: &mut fx,
            source_extent_raw: extent,
            sea_level_raw: Some(-100),
            retail_tick: latch.wrapping_add(1) as u32,
        },
        [11, 12, 13],
        [0, 4096, 0],
        &mut velocity,
    )
    .unwrap();
    assert!(!second.wreck_burst_emitted);
    assert_eq!(after, rng(&mut fx));
    let mut mismatch = request;
    mismatch.controlled_entity_handle = None;
    velocity = [7, -1000, 9];
    let mismatch = apply_player_dying_surface_contact(
        PlayerDyingSurfaceContactFrame {
            request: mismatch,
            runtime: &mut None,
            world_fx: &mut fx,
            source_extent_raw: extent,
            sea_level_raw: None,
            retail_tick: 12345,
        },
        [0; 3],
        [0, 4096, 0],
        &mut velocity,
    )
    .unwrap();
    assert!(!mismatch.controller_matched);
    assert_eq!(mismatch.unsupported_haptic, None);
    assert_eq!(velocity, [7, 1000, 9]);
    assert_eq!(after, rng(&mut fx));
    let mut missing = None;
    velocity = [7, -1000, 9];
    assert_eq!(
        apply_player_dying_surface_contact(
            PlayerDyingSurfaceContactFrame {
                request,
                runtime: &mut missing,
                world_fx: &mut fx,
                source_extent_raw: extent,
                sea_level_raw: None,
                retail_tick: 12345
            },
            [0; 3],
            [0, 4096, 0],
            &mut velocity
        ),
        Err(PlayerContactStyleBlock::MissingDyingControllerRuntime)
    );
    assert_eq!(velocity, [7, -1000, 9]);
    assert_eq!(after, rng(&mut fx));
}

#[v2k_test_support::retail_test]
fn lethal_material_constructor_is_visible_before_retained_terrain_tail_and_water_hook() {
    let session = session(13);
    let mut fx = WorldFx::new();
    let mut manager = native(&session, 13, &mut fx);
    let mut notifications = GameplayNotifications::new();
    let mut hull = PlayerHull::default();
    hull.health_raw = 1;
    manager.sync_player_hull_collision_state(&hull);
    let model = manager.player().unwrap().model_index.unwrap();
    let entry = session.cache.global_model(model).unwrap();
    let ground = terrain(-128);
    manager
        .player_mut()
        .unwrap()
        .set_motion_raw([0, -4096, 0], [0, -2000, 0]);
    manager
        .player_mut()
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(SURFACE_STATE_MASK, FULLY_ABOVE_SURFACE_STATE_BIT);
    let vars = Default::default();
    let player_id = manager.player().unwrap().id;
    let mut expected_constructor = fx.fork_for_main_base_abort_transaction();
    expected_constructor.emit_player_wreck_burst_raw(
        [0, -4096, 0],
        session.cache.global_model(67).unwrap().radius >> 1,
        Some(ground.sea_level_raw()),
        player_id,
    );
    let expected_latch =
        4793 + i32::from(expected_constructor.next_shared_retail_random_u16() >> 13);
    let mut contacts = PlayerSurfaceContactFrame {
        request: PlayerSurfaceContactRequest {
            resources: &session.cache,
            terrain: &ground,
            entry_model: Some(entry),
            entry_anim_vars: &vars,
            vehicle_mode: crate::player::VehicleMode::Hover,
            controlled_entity_handle: Some(player_id),
            ground_response_selectors: [7; 8],
            haptic_scale_raw: 15,
            retail_tick: 4793,
            extra_lives: 3,
        },
        hull: &mut hull,
        world_fx: &mut fx,
        notifications: &mut notifications,
        dying_runtime: None,
        water_style_outcome: None,
    };
    let mut order = Vec::new();
    let (outcome, result) = manager.update_with_player_surface_contacts(
        PlayerUpdateRequest {
            elapsed_micros: 0,
            active_model_collision_radius_raw: entry.collision_radius_raw,
            frame: VehicleFrameForces::DyingCommon,
            sea_level: Some(0.0),
            ..request(&ground, false)
        },
        true,
        |manager, phase| {
            match phase {
                PlayerSurfaceContactPhase::Terrain => order.push("terrain"),
                PlayerSurfaceContactPhase::Water { .. } => {
                    order.push("water");
                    assert!(
                        manager.player_dying_contact_runtime().is_some(),
                        "constructor precedes water dispatch"
                    );
                    assert_ne!(manager.player().unwrap().model_index, Some(model));
                    assert!(is_player_style(
                        match manager.player().unwrap().current_behavior_context {
                            RetailRuntimeValue::Known(Some(context)) => context,
                            _ => panic!(),
                        },
                        25
                    ));
                    assert!(
                        contacts.world_fx.particle_count() > 0,
                        "synchronous constructor burst precedes water"
                    );
                }
            }
            contacts.dispatch(manager, phase)
        },
    );
    assert_eq!(order, ["terrain", "water"]);
    let completed = result.unwrap().unwrap().unwrap().unwrap();
    let PlayerTerrainStyleCallbackOutcome::PlayerControl(style) = completed.style_callback else {
        panic!("retained living style packet")
    };
    assert!(style.hull_damage.destroyed_now);
    assert_eq!(completed.velocity_before_raw, [0, -2000, 0]);
    assert!(
        completed.position_after_raw[1] > completed.position_before_raw[1],
        "retained real sphere response follows constructor"
    );
    assert_eq!(
        contacts
            .water_style_outcome
            .unwrap()
            .unwrap()
            .normal_velocity_raw,
        (i32::from(i16::MAX) * i32::from(completed.velocity_after_raw[1])) >> 12
    );
    assert!(
        !contacts
            .water_style_outcome
            .unwrap()
            .unwrap()
            .wreck_burst_emitted,
        "constructor seeds the same-frame AF0 latch"
    );
    assert!(contacts.commit_runtime(&mut manager));
    assert_eq!(
        contacts.world_fx.test_particles_in_virgin_birth_order(),
        expected_constructor.test_particles_in_virgin_birth_order(),
        "447280 emits from the unseparated terrain pose before141D0"
    );
    assert_eq!(
        manager
            .player_dying_contact_runtime()
            .unwrap()
            .next_burst_tick(),
        expected_latch
    );
    assert!(outcome.hard_water_entry_commit.is_none());
    assert_eq!(
        manager.player().unwrap().velocity_raw()[1],
        1000,
        "AF0 VY1000 selects the common surface-burst branch, which preserves outward velocity"
    );
    assert_eq!(
        contacts.world_fx.particle_count(),
        expected_constructor.particle_count(),
        "same-frame water AF0 does not replay constructor475F0"
    );
    assert_eq!(
        rng(contacts.world_fx),
        rng(&mut expected_constructor),
        "same-frame AF0 consumes no extra wreck or latch RNG"
    );
}

#[v2k_test_support::retail_test]
fn class25_common_mover_skips_both_living_force_packets_and_preserves_body_attitude() {
    let session = session(13);
    let mut fx = WorldFx::new();
    let mut manager = native(&session, 13, &mut fx);
    let mut hull = PlayerHull::default();
    die(&mut manager, &mut hull, &mut fx, &session.cache, 100);
    manager
        .player_mut()
        .unwrap()
        .set_motion_raw([0, 1000, 0], [123, 0, -456]);
    let ground = terrain(-128);
    let (outcome, _) = manager.update_with_player_surface_contacts(
        PlayerUpdateRequest {
            frame: VehicleFrameForces::DyingCommon,
            body_pitch_roll_raw: [1234, -2222],
            ..request(&ground, false)
        },
        false,
        |_, phase| match phase {
            PlayerSurfaceContactPhase::Terrain => PlayerSurfaceContactDispatch::Terrain(()),
            PlayerSurfaceContactPhase::Water { .. } => PlayerSurfaceContactDispatch::WaterComplete,
        },
    );
    assert_eq!(outcome.surface_effect, None);
    assert_eq!(outcome.vtol_diagnostics, None);
    assert_eq!(outcome.fuel_burn_raw, 0);
    assert_eq!(
        outcome.body_angle_words_after_environment,
        Some([1234, -2222])
    );
}

#[v2k_test_support::retail_test]
fn native_dying_water_hook_precedes_selector7_suppression_and_common_surface_burst() {
    for selector in [6, 7] {
        let session = session(13);
        let mut fx = WorldFx::new();
        let mut manager = native(&session, 13, &mut fx);
        let mut hull = PlayerHull::default();
        die(&mut manager, &mut hull, &mut fx, &session.cache, 100);
        manager
            .player_mut()
            .unwrap()
            .set_motion_raw([0, -100, 0], [0, -65, 0]);
        manager
            .player_mut()
            .unwrap()
            .collision
            .state_flags_at_0x08
            .overwrite(SURFACE_STATE_MASK, FULLY_ABOVE_SURFACE_STATE_BIT);
        let id = manager.player().unwrap().id;
        let tick = manager
            .player_dying_contact_runtime()
            .unwrap()
            .next_burst_tick() as u32;
        let ground = terrain(-128);
        let vars = Default::default();
        let mut notifications = GameplayNotifications::new();
        let before = rng(&mut fx);
        let mut contacts = PlayerSurfaceContactFrame {
            request: PlayerSurfaceContactRequest {
                resources: &session.cache,
                terrain: &ground,
                entry_model: None,
                entry_anim_vars: &vars,
                vehicle_mode: crate::player::VehicleMode::Hover,
                controlled_entity_handle: Some(id),
                ground_response_selectors: [0; 8],
                haptic_scale_raw: 15,
                retail_tick: tick,
                extra_lives: 3,
            },
            hull: &mut hull,
            world_fx: &mut fx,
            notifications: &mut notifications,
            dying_runtime: manager.player_dying_contact_runtime(),
            water_style_outcome: None,
        };
        let (outcome, _) = manager.update_with_player_surface_contacts(
            PlayerUpdateRequest {
                elapsed_micros: 0,
                retail_tick: tick,
                frame: VehicleFrameForces::DyingCommon,
                active_model_collision_radius_raw: 10,
                water_response_selectors: [selector; 8],
                ..request(&ground, false)
            },
            true,
            |manager, phase| contacts.dispatch(manager, phase),
        );
        let callback = contacts.water_style_outcome.unwrap().unwrap();
        assert_eq!(
            callback.normal_velocity_raw, -520,
            "water retains129B0 normal0x7fff throughAF0's >>12"
        );
        assert_eq!(callback.unsupported_haptic.unwrap().strength_raw, 8);
        assert!(!callback.wreck_burst_emitted);
        assert_eq!(*contacts.world_fx.entity_construction_state().0, before);
        assert!(contacts.commit_runtime(&mut manager));
        assert_eq!(manager.player().unwrap().velocity_raw()[1], 1000);
        assert_eq!(outcome.water_entry.is_none(), selector == 7);
    }
}
