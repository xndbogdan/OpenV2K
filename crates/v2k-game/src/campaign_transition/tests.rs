use super::*;

fn route(subtype: u8, args: [i8; 2], record_index: usize) -> AuthoredCampaignRoute {
    AuthoredCampaignRoute {
        record_index,
        route: CampaignWarpRoute {
            source_level_id: 13,
            marker_cell: [149, 239],
            marker_kind: marker_kind_for_subtype(subtype),
            marker_subtype: subtype,
            marker_args: args,
            destination_logical_level: 18,
            destination_level_id: 30,
            arrival: WarpArrival {
                position_raw: [7424, 5120, 9728],
                heading_raw: 0x4000,
            },
        },
    }
}

fn contact(subtype: u8, terrain_type: u8) -> StaticModelContact {
    StaticModelContact {
        cell: [149, 239],
        attribute: 1,
        terrain_type,
        model_id: 39,
        kind_index: marker_kind_for_subtype(subtype),
        normal_q12: [0, 4096, 0],
        penetration_raw: 1,
        response_raw: 1536,
    }
}

fn progress() -> PlayerCampaignProgress {
    let mut progress = PlayerCampaignProgress::new();
    progress.set_current_control_slot(Some(1));
    progress
}

#[test]
fn actual_contact_stamp_is_single_use_and_preserves_authored_column() {
    let authored = route(2, [2, 2], 3);
    let mut runtime = CampaignWarpRuntime {
        routes: vec![authored],
        ..Default::default()
    };
    let mut progress = progress();
    assert_eq!(
        runtime.poll_route(
            &mut progress,
            50,
            crate::campaign_transition::CampaignRouteVisit::FirstMatching
        ),
        None
    );
    runtime.observe_player_static_contact(contact(2, 0x18), 50);
    assert_eq!(
        runtime.poll_route(
            &mut progress,
            50,
            crate::campaign_transition::CampaignRouteVisit::FirstMatching
        ),
        Some(authored.route)
    );
    assert_eq!(progress.control_slot_bits(1), Some((1 << 12) | 0x20));
    assert_eq!(
        runtime.poll_route(
            &mut progress,
            50,
            crate::campaign_transition::CampaignRouteVisit::FirstMatching
        ),
        None
    );
}

#[test]
fn stamp_selectors_are_bit3_then_bit4_with_explicit_two_wildcard() {
    let first = route(2, [1, 0], 0);
    let second = route(2, [2, 2], 1);
    for (terrain_type, expected) in [(0, second), (8, first), (16, second), (24, second)] {
        let mut runtime = CampaignWarpRuntime {
            routes: vec![first, second],
            ..Default::default()
        };
        runtime.observe_player_static_contact(contact(2, terrain_type), 20);
        let mut progress = progress();
        assert_eq!(
            runtime.poll_route(
                &mut progress,
                20,
                crate::campaign_transition::CampaignRouteVisit::FirstMatching
            ),
            Some(expected.route)
        );
        assert_eq!(
            progress.control_slot_bits(1),
            Some((1 << (9 + expected.record_index)) | 0x20)
        );
    }
}

#[test]
fn rejected_selector_retains_stamp_and_unsigned_age_is_strict_even_at_wrap() {
    let mut runtime = CampaignWarpRuntime {
        routes: vec![route(2, [1, 0], 0)],
        ..Default::default()
    };
    let mut progress = progress();
    runtime.observe_player_static_contact(contact(2, 0), u32::MAX);
    assert_eq!(
        runtime.poll_route(
            &mut progress,
            u32::MAX,
            crate::campaign_transition::CampaignRouteVisit::FirstMatching
        ),
        None
    );
    assert!(runtime.marker_stamp.is_some());
    runtime.routes[0].route.marker_args = [2, 2];
    assert!(runtime
        .poll_route(
            &mut progress,
            0,
            crate::campaign_transition::CampaignRouteVisit::FirstMatching
        )
        .is_some());
    runtime.observe_player_static_contact(contact(2, 0), u32::MAX);
    assert_eq!(
        runtime.poll_route(
            &mut progress,
            1,
            crate::campaign_transition::CampaignRouteVisit::FirstMatching
        ),
        None
    );
}

#[test]
fn foreign_control_slot_and_unrelated_contact_cannot_commit_route() {
    let mut runtime = CampaignWarpRuntime {
        routes: vec![route(2, [2, 2], 0)],
        ..Default::default()
    };
    let mut progress = progress();
    progress.set_current_control_slot(Some(2));
    runtime.observe_player_static_contact(contact(2, 0), 20);
    let before = progress;
    assert_eq!(
        runtime.poll_route(
            &mut progress,
            20,
            crate::campaign_transition::CampaignRouteVisit::FirstMatching
        ),
        None
    );
    assert_eq!(progress, before);
    assert!(runtime.marker_stamp.is_some());
    runtime.marker_stamp = None;
    let mut fuel = contact(2, 0);
    fuel.kind_index = 4;
    runtime.observe_player_static_contact(fuel, 20);
    assert!(runtime.marker_stamp.is_none());
}

#[test]
fn marker_constructor_uses_signed_height_and_wrapped_cell_centres() {
    assert_eq!(
        marker_position_raw([149, 239], 0xaf),
        [-27264, -2080, -4224]
    );
    assert_eq!(marker_position_raw([21, 39], 0x4b), [5504, 2912, 10112]);
}

#[test]
fn ordered_record_probe_does_not_consume_an_earlier_or_later_matching_stamp() {
    let authored = route(2, [2, 2], 3);
    let mut runtime = CampaignWarpRuntime {
        routes: vec![authored],
        ..Default::default()
    };
    let mut progress = progress();
    runtime.observe_player_static_contact(contact(2, 0x18), 50);
    for index in [0, 2, 4] {
        assert_eq!(
            runtime.poll_route(&mut progress, 50, CampaignRouteVisit::Record(index)),
            None
        );
        assert!(runtime.marker_stamp.is_some());
        assert_eq!(progress.control_slot_bits(1), Some(0));
    }
    assert_eq!(
        runtime.poll_route(&mut progress, 50, CampaignRouteVisit::Record(3)),
        Some(authored.route)
    );
    assert_eq!(progress.control_slot_bits(1), Some((1 << 12) | 0x20));
    assert!(runtime.marker_stamp.is_none());
}

#[test]
fn retry_returns_the_same_interactive_world_and_exact_retained_xyz_with_fixed_heading() {
    let retained = [i16::MIN, -129, i16::MAX];
    for level in 13..=48 {
        let transition =
            CampaignTransition::FailedWorldRetry(FailedWorldRetry::new(level, retained).unwrap());
        assert_eq!(transition.source_level_id(), level);
        assert_eq!(transition.destination_level_id(), level);
        assert_eq!(transition.destination_logical_level(), level - 12);
        assert_eq!(
            transition.arrival(),
            WarpArrival {
                position_raw: retained,
                heading_raw: 0x4000,
            }
        );
    }
    for level in [0, 1, 12, 49, 50, u32::MAX] {
        assert_eq!(FailedWorldRetry::new(level, retained), None);
    }
    let authored = route(2, [2, 2], 3).route;
    let transition = CampaignTransition::AuthoredMarker(authored);
    assert_eq!(transition.source_level_id(), authored.source_level_id);
    assert_eq!(
        transition.destination_level_id(),
        authored.destination_level_id
    );
    assert_eq!(
        transition.destination_logical_level(),
        authored.destination_logical_level
    );
    assert_eq!(transition.arrival(), authored.arrival);
}

#[test]
fn failed_interior_preempts_a_real_marker_stamp_without_pair_or_exit_progress_writes() {
    use crate::campaign_failure::{
        CampaignCasualtyFrame, CampaignCasualtyState, CampaignSelectorCursor,
        CampaignSelectorEntry, CampaignSelectorStep,
    };
    use crate::power_up_contact::RETAIL_CONTROL_SLOT_COUNT;
    use crate::world_complete_results::Fun0042e210CapabilityCensus;
    use v2k_formats::levels::CampaignRecord;

    let mut record = CampaignRecord { data: [0; 32] };
    record.data[0x0c] = 0x10;
    let authored = route(2, [2, 2], 0);
    let mut runtime = CampaignWarpRuntime {
        routes: vec![authored],
        ..Default::default()
    };
    runtime.observe_player_static_contact(contact(2, 0x18), 50);
    let bits = std::array::from_fn::<_, RETAIL_CONTROL_SLOT_COUNT, _>(|index| {
        0x9000_0000 | ((index as u32) << 16) | 0x0044
    });
    let mut progress = PlayerCampaignProgress::from_native_snapshot(1, bits, 5, 7);
    let before = progress;
    let mut state = CampaignCasualtyState::default();
    let mut cursor = CampaignSelectorCursor::new(
        CampaignCasualtyFrame {
            census: Fun0042e210CapabilityCensus::default(),
            world_saved: false,
            abort_active: true,
        },
        CampaignSelectorEntry {
            failed_world_interior_request: true,
            ..Default::default()
        },
    );
    let transition = match cursor.next_step(&mut state, &[record]).unwrap() {
        CampaignSelectorStep::RetryCurrentWorld => CampaignTransition::FailedWorldRetry(
            FailedWorldRetry::new(13, [111, -222, 333]).unwrap(),
        ),
        CampaignSelectorStep::CheckRoute { record_index } => CampaignTransition::AuthoredMarker(
            runtime
                .poll_route(&mut progress, 50, CampaignRouteVisit::Record(record_index))
                .unwrap(),
        ),
        other => panic!("unexpected selector step: {other:?}"),
    };
    assert_eq!(transition.destination_level_id(), 13);
    assert_eq!(transition.arrival().position_raw, [111, -222, 333]);
    assert_ne!(transition.arrival(), authored.route.arrival);
    assert_eq!(progress, before, "retain all37 words, lives and trophies");
    assert!(
        runtime.marker_stamp.is_some(),
        "retry did not consume static contact"
    );
    assert_eq!(state, CampaignCasualtyState::default());
}
