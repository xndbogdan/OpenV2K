use super::*;

fn request() -> HiveComponentDeathRequest {
    HiveComponentDeathRequest {
        entity_id: 0x0420_0001,
        controller_state: 1,
        forced_death_requested: true,
        health_raw: 1_000_000_000,
        objective_hostile_present: true,
        source_remote_owned: false,
        death_sound_id: 67,
        entity_position_raw: [100, 200, 300],
        anchor_position_raw: [-400, -500, -600],
        current_model_extent_raw: 175,
        marker_present: true,
    }
}

#[test]
fn forced_locked_death_preserves_distinct_cue_anchor_and_current_extent() {
    let plan = plan_hive_component_death(request()).unwrap();
    assert!(!plan.completion_prefix);
    assert_eq!((plan.controller_state_after, plan.health_after_raw), (0, 0));
    assert_eq!(plan.sound_position_raw, [100, 200, 300]);
    assert_eq!(plan.effects_position_raw, [-400, -500, -600]);
    assert_eq!(plan.surface_burst.source_extent_raw, 175);
    assert_eq!(plan.radial.trailing_raw, [67, 0x0420_0001]);
    assert_eq!(plan.radial.packet.channels, [1, 4]);
    assert_eq!(plan.radial.packet.amounts_raw, [10_000, 8_000]);
    assert_eq!(plan.suction_timer_after_us, Some(-6_000_000));
    assert_eq!(plan.radial_accumulator_after_us, 0);
    assert!(!plan.source_remote_owned);
}

#[test]
fn completion_prefix_has_the_source_forced_and_objective_matrix() {
    for forced in [false, true] {
        for hostile in [false, true] {
            let plan = plan_hive_component_death(HiveComponentDeathRequest {
                forced_death_requested: forced,
                objective_hostile_present: hostile,
                health_raw: 0,
                ..request()
            })
            .unwrap();
            assert_eq!(plan.completion_prefix, !forced || !hostile);
            assert_eq!(plan.objective_hostile_present, hostile);
        }
    }
    assert!(
        plan_hive_component_death(HiveComponentDeathRequest {
            source_remote_owned: true,
            ..request()
        })
        .unwrap()
        .source_remote_owned
    );
}

#[test]
fn live_positive_health_state_zero_and_missing_marker_keep_separate_paths() {
    assert!(plan_hive_component_death(HiveComponentDeathRequest {
        forced_death_requested: false,
        ..request()
    })
    .is_none());
    assert!(plan_hive_component_death(HiveComponentDeathRequest {
        controller_state: 0,
        ..request()
    })
    .is_none());
    for state in [1, 2, 3] {
        let plan = plan_hive_component_death(HiveComponentDeathRequest {
            controller_state: state,
            forced_death_requested: false,
            health_raw: -1,
            marker_present: false,
            ..request()
        })
        .unwrap();
        assert!(plan.completion_prefix);
        assert_eq!(plan.suction_timer_after_us, None);
    }
}

#[test]
fn subsequent_state_zero_callback_only_requests_a_live_nonzero_entity() {
    for flags in [None, Some(0), Some(0x4000), Some(0x8000_4000)] {
        assert!(!hive_state_zero_requests_standard_death(0, flags));
    }
    assert!(hive_state_zero_requests_standard_death(0, Some(0x8000)));
    assert!(hive_state_zero_requests_standard_death(
        0,
        Some(0x8000_0000)
    ));
    assert!(!hive_state_zero_requests_standard_death(1, Some(0x8000)));
}
