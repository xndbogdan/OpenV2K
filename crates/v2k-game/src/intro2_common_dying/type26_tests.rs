use super::*;
use crate::entity_scheduler::SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT;
use crate::intro2_type47_live::world::native_intro2_fixture;

#[v2k_test_support::retail_test]
fn intro2_class12_type26_retains_authored_mass_model_and_absent_j_through_actual_world_tick() {
    for spawn in [10, 25] {
        let Some((session, mut manager, _)) = native_intro2_fixture() else {
            return;
        };
        let id = manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(spawn))
            .unwrap()
            .id;
        let metadata = manager.type_runtime_metadata(26).unwrap().clone();
        assert_eq!(metadata.mass_raw, 400);
        assert_eq!(metadata.model_slots, [267; 4]);
        assert_eq!(
            metadata.sub_j_attachment_descriptor,
            RetailRuntimeValue::Known(None)
        );
        let entity = manager.entity_mut(id).unwrap();
        let sub_d_before = entity.intro2_type26_sub_d_frame_owner;
        entity.set_motion_raw([0, 10_000, 0], [50, -80, 100]);
        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(9);
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(u32::MAX, 0x68000);
        let mut fx = WorldFx::new();
        let owner = publish_intro2_common_standard_death(&mut manager, id, &mut fx)
            .unwrap()
            .unwrap();
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(9)
        );
        assert_eq!(
            entity.collision.default_state_flags_at_0xc8,
            RetailRuntimeValue::Known(0x439)
        );
        assert_eq!(
            entity.sub_j_attachment_runtime,
            RetailRuntimeValue::Known(None)
        );
        entity.collision.state_flags_at_0x08.overwrite(
            SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
            SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
        );
        let tick = tick_intro2_common_dying(
            &mut manager,
            owner,
            Intro2CommonDyingFrame {
                resources: &session.cache,
                world_fx: &mut fx,
                elapsed_micros: 20_000,
                retail_tick: 1,
            },
        );
        assert!(
            matches!(
                tick.outcome,
                Intro2CommonDyingOutcome::Advanced {
                    detailed: true,
                    terminal: false,
                    callback_elapsed_micros: 20_000,
                    ..
                }
            ),
            "{:?}",
            tick.outcome
        );
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(entity.mass_raw, 409, "callback retains Type26 mass plus B2");
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(entity.model_index, Some(267));
        assert_eq!(entity.model_slots, [Some(267); 4]);
        assert_eq!(
            entity.sub_j_attachment_runtime,
            RetailRuntimeValue::Known(None)
        );
        assert_eq!(
            entity.intro2_type26_sub_d_frame_owner, sub_d_before,
            "null class12 target must not consume or replace the birth's classifier owner"
        );
        assert!(tick.retained_owner.is_some());
    }
}

#[v2k_test_support::retail_test]
fn intro2_class12_type26_surface_uses_model267_extent_after_coarse_terminal_callback() {
    let Some((session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let id = manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(25))
        .unwrap()
        .id;
    let radius = session.cache.global_model(267).unwrap().radius;
    let other_radius = session.cache.global_model(302).unwrap().radius;
    assert_ne!(
        radius >> 2,
        other_radius >> 2,
        "fixture must distinguish the two models"
    );
    let sea = session.cache.level_terrain().unwrap().sea_level_raw();
    // Pick a Y that the two model headers put on opposite sides of the deep
    // threshold. The callback and E100 change velocity; integration is later.
    let y = sea - ((radius >> 2).min(other_radius >> 2) + 1) as i16;
    let expected_deep = i32::from(y) < i32::from(sea) - i32::from(radius >> 2);
    let entity = manager.entity_mut(id).unwrap();
    entity.set_motion_raw([0, y, 0], [0; 3]);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(u32::MAX, 0x60000);
    entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(100);
    let mut fx = WorldFx::new();
    let owner = publish_intro2_common_standard_death(&mut manager, id, &mut fx)
        .unwrap()
        .unwrap();
    let entity = manager.entity_mut(id).unwrap();
    entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(250_000);
    entity.collision.callback_scheduler_accumulator_us_at_0x6c = RetailRuntimeValue::Known(125_001);
    let tick = tick_intro2_common_dying(
        &mut manager,
        owner,
        Intro2CommonDyingFrame {
            resources: &session.cache,
            world_fx: &mut fx,
            elapsed_micros: 0,
            retail_tick: 1,
        },
    );
    let Intro2CommonDyingOutcome::Advanced {
        terminal: true,
        detailed: false,
        callback_elapsed_micros,
        ..
    } = tick.outcome
    else {
        panic!("{:?}", tick.outcome)
    };
    let elapsed_ms = callback_elapsed_micros / 1000;
    assert!(elapsed_ms > 0);
    let expected_timer = if expected_deep {
        100 + elapsed_ms
    } else {
        100u32.saturating_sub(elapsed_ms)
    };
    assert_eq!(
        manager
            .entity_mut(id)
            .unwrap()
            .surface_lifetime_timer_ms_at_0x48,
        RetailRuntimeValue::Known(expected_timer),
        "cached Type26 model survives the terminal callback"
    );
    assert_eq!(manager.pending_actor_deferred_destroy_ids(), [id]);
    assert!(tick.retained_owner.is_none());
}
