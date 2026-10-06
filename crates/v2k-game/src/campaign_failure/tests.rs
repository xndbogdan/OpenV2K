use super::*;
use crate::entity_collision_state::{RetailStateWord, DYING_STATE_BIT};
use crate::world_complete_results::{fun_0042e210_capability_census, Fun0042e210EntitySample};

fn record(flags: u32, minimum_peasants: i8, threshold: i8) -> CampaignRecord {
    let mut data = [0; 32];
    data[0x0c..0x10].copy_from_slice(&flags.to_le_bytes());
    data[0x18] = minimum_peasants as u8;
    data[0x19] = threshold as u8;
    CampaignRecord { data }
}

fn frame(scientists: i32, peasants: i32) -> CampaignCasualtyFrame {
    CampaignCasualtyFrame {
        census: Fun0042e210CapabilityCensus {
            cap_0x400: scientists,
            cap_0x800: peasants,
            cap_0x08: 0,
        },
        world_saved: false,
        abort_active: false,
    }
}

fn warning() -> Vec<CampaignCasualtyAction> {
    vec![CampaignCasualtyAction::DirectText(
        CampaignCasualtyText::OneMoreLoss,
    )]
}

fn loss(record_index: usize) -> Vec<CampaignCasualtyAction> {
    vec![
        CampaignCasualtyAction::DirectText(CampaignCasualtyText::WorldLost),
        CampaignCasualtyAction::AbortWorld { record_index },
    ]
}

#[test]
fn warning_boundary_uses_survivors_and_is_shared_until_world_reload() {
    let records = [record(0x84, 0, 5), record(0x84, 0, 5)];
    let mut state = CampaignCasualtyState::default();
    assert!(state.evaluate(&records, frame(2, 5)).unwrap().is_empty());
    assert_eq!(state.evaluate(&records, frame(2, 4)).unwrap(), warning());
    assert!(state.evaluate(&records, frame(2, 4)).unwrap().is_empty());
    assert!(state.evaluate(&records, frame(3, 4)).unwrap().is_empty());
    assert!(state.evaluate(&records, frame(2, 4)).unwrap().is_empty());
    assert_eq!(
        CampaignCasualtyState::default()
            .evaluate(&records, frame(2, 4))
            .unwrap(),
        warning()
    );
}

#[test]
fn casualty_text_precedes_abort_and_skipped_warning_does_not_prevent_loss() {
    let records = [record(0x10, 0, 0), record(0x84, 0, 5)];
    let mut state = CampaignCasualtyState::default();
    assert_eq!(state.evaluate(&records, frame(1, 4)).unwrap(), loss(1));
    assert_eq!(state.evaluate(&records, frame(0, 2)).unwrap(), loss(1));
}

#[test]
fn completed_or_already_aborted_world_suppresses_predicates_without_latching() {
    let records = [record(0x84, 0, 0)];
    let mut state = CampaignCasualtyState::default();
    for suppressed in [
        CampaignCasualtyFrame {
            world_saved: true,
            ..frame(0, 1)
        },
        CampaignCasualtyFrame {
            abort_active: true,
            ..frame(0, 1)
        },
        CampaignCasualtyFrame {
            world_saved: true,
            ..frame(0, 0)
        },
        CampaignCasualtyFrame {
            abort_active: true,
            ..frame(0, 0)
        },
    ] {
        assert!(state.evaluate(&records, suppressed).unwrap().is_empty());
    }
    assert_eq!(state.evaluate(&records, frame(0, 1)).unwrap(), warning());
}

#[test]
fn minimum_peasant_condition_precedes_warning_and_loss_and_uses_signed_byte() {
    let mut state = CampaignCasualtyState::default();
    let records = [record(0x86, 2, 3)];
    assert!(state.evaluate(&records, frame(3, 1)).unwrap().is_empty());
    assert!(state.evaluate(&records, frame(2, 1)).unwrap().is_empty());
    assert_eq!(state.evaluate(&records, frame(2, 2)).unwrap(), warning());
    assert_eq!(state.evaluate(&records, frame(1, 2)).unwrap(), loss(0));
    assert_eq!(
        state.evaluate(&[record(0x86, -1, 0)], frame(0, 0)).unwrap(),
        loss(0)
    );
}

#[test]
fn casualty_threshold_is_signed_and_census_addition_wraps_as_x86_dwords() {
    let mut state = CampaignCasualtyState::default();
    assert_eq!(
        state.evaluate(&[record(0x84, 0, -1)], frame(0, 0)).unwrap(),
        warning()
    );
    assert_eq!(
        state
            .evaluate(&[record(0x84, 0, -1)], frame(0, -1))
            .unwrap(),
        loss(0)
    );
    assert_eq!(
        state
            .evaluate(&[record(0x84, 0, 0)], frame(i32::MAX, 1))
            .unwrap(),
        loss(0)
    );
}

#[test]
fn mixed_unsupported_predicates_do_not_commit_population_only_effects() {
    let mut state = CampaignCasualtyState::default();
    assert_eq!(
        state.evaluate(&[record(0x84, 0, 0), record(0x85, 0, 0)], frame(0, 1)),
        Err(UnsupportedCasualtyRecord {
            record_index: 1,
            flags: 0x85
        })
    );
    assert_eq!(
        state.evaluate(&[record(0x84, 0, 0)], frame(0, 1)).unwrap(),
        warning()
    );
    assert!(state
        .evaluate(&[record(0x10, 0, 0), record(0xa0, 0, 0)], frame(0, 0))
        .unwrap()
        .is_empty());
}

#[test]
fn census_includes_sub_m_occupants_and_excludes_dying_people_before_deletion() {
    let census = fun_0042e210_capability_census([
        Fun0042e210EntitySample {
            capability: 0x800,
            state: RetailStateWord::exact(1 | DYING_STATE_BIT),
            sub_m_native_count_raw: None,
        },
        Fun0042e210EntitySample {
            capability: 0,
            state: RetailStateWord::exact(1),
            sub_m_native_count_raw: Some(2),
        },
        Fun0042e210EntitySample {
            capability: 0x800,
            state: RetailStateWord::exact(1),
            sub_m_native_count_raw: None,
        },
    ]);
    let mut state = CampaignCasualtyState::default();
    let records = [record(0x84, 0, 2)];
    assert_eq!(
        state
            .evaluate(
                &records,
                CampaignCasualtyFrame {
                    census,
                    ..frame(0, 0)
                }
            )
            .unwrap(),
        warning()
    );
    assert_eq!(state.evaluate(&records, frame(2, 0)).unwrap(), loss(0));
}

#[test]
fn ordered_selector_returns_at_an_earlier_route_without_latching_later_warning() {
    let records = [record(0x10, 0, 0), record(0x84, 0, 5)];
    let mut state = CampaignCasualtyState::default();
    let mut cursor = CampaignSelectorCursor::new(frame(0, 6), CampaignSelectorEntry::default());
    assert_eq!(
        cursor.next_step(&mut state, &records).unwrap(),
        CampaignSelectorStep::CheckRoute { record_index: 0 }
    );
    // A successful route returns from42DD10 here. The next world's load owns
    // reset, but no warning may be issued or latched by the departed record.
    assert_eq!(state, CampaignCasualtyState::default());
    // If contact does not match, scanning reaches the warning on this visit.
    assert_eq!(
        cursor.next_step(&mut state, &records).unwrap(),
        CampaignSelectorStep::DirectText(CampaignCasualtyText::OneMoreLoss)
    );
    assert_eq!(
        cursor.next_step(&mut state, &records).unwrap(),
        CampaignSelectorStep::Finished
    );
}

#[test]
fn ordered_selector_resumes_after_abort_before_a_later_route_using_entry_snapshot() {
    let records = [record(0x84, 0, 5), record(0x10, 0, 0), record(0x84, 0, 5)];
    let mut state = CampaignCasualtyState::default();
    let mut cursor = CampaignSelectorCursor::new(frame(0, 5), CampaignSelectorEntry::default());
    let steps: Vec<_> = (0..6)
        .map(|_| cursor.next_step(&mut state, &records).unwrap())
        .collect();
    assert_eq!(
        steps,
        [
            CampaignSelectorStep::DirectText(CampaignCasualtyText::WorldLost),
            CampaignSelectorStep::AbortCasualties { record_index: 0 },
            CampaignSelectorStep::CheckRoute { record_index: 1 },
            CampaignSelectorStep::DirectText(CampaignCasualtyText::WorldLost),
            CampaignSelectorStep::AbortCasualties { record_index: 2 },
            CampaignSelectorStep::Finished,
        ]
    );
    //456960 itself suppresses duplicate execution;42DD10's entry abort snapshot
    // must not be silently replaced with the flag written by the first call.
}

#[test]
fn saved_world_consumes_main_base_request_and_keeps_routes_but_skips_casualties() {
    let records = [record(0x84, 0, 5), record(0x10, 0, 0)];
    let mut state = CampaignCasualtyState::default();
    for frame in [
        CampaignCasualtyFrame {
            world_saved: true,
            ..frame(0, 0)
        },
        CampaignCasualtyFrame {
            abort_active: true,
            ..frame(0, 0)
        },
    ] {
        let mut cursor = CampaignSelectorCursor::new(
            frame,
            CampaignSelectorEntry {
                main_base_request: true,
                ..Default::default()
            },
        );
        assert_eq!(
            cursor.next_step(&mut state, &records).unwrap(),
            CampaignSelectorStep::CheckRoute { record_index: 1 }
        );
        assert_eq!(
            cursor.next_step(&mut state, &records).unwrap(),
            CampaignSelectorStep::Finished
        );
    }
    assert_eq!(state, CampaignCasualtyState::default());
    let mut cursor = CampaignSelectorCursor::new(
        frame(0, 0),
        CampaignSelectorEntry {
            main_base_request: true,
            ..Default::default()
        },
    );
    assert_eq!(
        cursor.next_step(&mut state, &records).unwrap(),
        CampaignSelectorStep::AbortMainBase
    );
    assert_eq!(
        cursor.next_step(&mut state, &records).unwrap(),
        CampaignSelectorStep::Finished
    );
    assert_eq!(
        state,
        CampaignCasualtyState::default(),
        "Main Base early return precedes warning/loss records"
    );
}

#[test]
fn ordered_selector_rejects_reached_mixed_casualty_before_committing_its_text() {
    let records = [record(0x10, 0, 0), record(0x85, 0, 5)];
    let mut state = CampaignCasualtyState::default();
    let mut cursor = CampaignSelectorCursor::new(frame(0, 6), CampaignSelectorEntry::default());
    assert_eq!(
        cursor.next_step(&mut state, &records).unwrap(),
        CampaignSelectorStep::CheckRoute { record_index: 0 }
    );
    assert_eq!(
        cursor.next_step(&mut state, &records),
        Err(UnsupportedCasualtyRecord {
            record_index: 1,
            flags: 0x85
        })
    );
    assert_eq!(state, CampaignCasualtyState::default());
    assert_eq!(
        cursor.next_step(&mut state, &records).unwrap(),
        CampaignSelectorStep::Finished
    );
}

#[test]
fn aborted_interior_returns_current_world_before_every_record_and_main_base_request() {
    // A reached85 row would block; a10 row would yield a marker visit. The
    // retry must terminate before either, even if the saved flag is also set.
    let records = [record(0x85, 0, 0), record(0x10, 0, 0)];
    for world_saved in [false, true] {
        let mut state = CampaignCasualtyState::default();
        let mut cursor = CampaignSelectorCursor::new(
            CampaignCasualtyFrame {
                abort_active: true,
                world_saved,
                ..frame(0, 0)
            },
            CampaignSelectorEntry {
                main_base_request: true,
                failed_world_interior_request: true,
            },
        );
        assert_eq!(
            cursor.next_step(&mut state, &records),
            Ok(CampaignSelectorStep::RetryCurrentWorld)
        );
        assert_eq!(
            cursor.next_step(&mut state, &records),
            Ok(CampaignSelectorStep::Finished)
        );
        assert_eq!(state, CampaignCasualtyState::default());
    }
}

#[test]
fn interior_request_without_abort_keeps_existing_main_base_and_marker_decisions() {
    let records = [record(0x10, 0, 0)];
    for (world_saved, main_base_request, expected) in [
        (
            false,
            false,
            CampaignSelectorStep::CheckRoute { record_index: 0 },
        ),
        (false, true, CampaignSelectorStep::AbortMainBase),
        (
            true,
            true,
            CampaignSelectorStep::CheckRoute { record_index: 0 },
        ),
    ] {
        let mut state = CampaignCasualtyState::default();
        let mut cursor = CampaignSelectorCursor::new(
            CampaignCasualtyFrame {
                world_saved,
                ..frame(0, 0)
            },
            CampaignSelectorEntry {
                main_base_request,
                failed_world_interior_request: true,
            },
        );
        assert_eq!(cursor.next_step(&mut state, &records), Ok(expected));
        assert_eq!(state, CampaignCasualtyState::default());
    }
}

#[test]
fn aborted_world_without_interior_request_still_visits_normal_routes() {
    let records = [record(0x85, 0, 0), record(0x10, 0, 0)];
    let mut state = CampaignCasualtyState::default();
    let mut cursor = CampaignSelectorCursor::new(
        CampaignCasualtyFrame {
            abort_active: true,
            ..frame(0, 0)
        },
        CampaignSelectorEntry::default(),
    );
    assert_eq!(
        cursor.next_step(&mut state, &records),
        Ok(CampaignSelectorStep::CheckRoute { record_index: 1 })
    );
    assert_eq!(state, CampaignCasualtyState::default());
}
