//! Authored survivor limits and the warning/loss pair observed in NoCD05.

use v2k_game::campaign_failure::{
    CampaignCasualtyAction, CampaignCasualtyFrame, CampaignCasualtyState, CampaignCasualtyText,
    CampaignSelectorCursor, CampaignSelectorEntry, CampaignSelectorStep,
};
use v2k_game::session::GameSession;
use v2k_game::world_complete_results::Fun0042e210CapabilityCensus;

/// A clean landscape and a live capability-`0x4000` entity: virus and census
/// rows stay quiet.
fn quiet_frame() -> CampaignCasualtyFrame {
    let mut capability_buckets = [0; 14];
    capability_buckets[1] = 1;
    CampaignCasualtyFrame {
        capability_buckets,
        virus_percent: Some(0),
        ..CampaignCasualtyFrame::default()
    }
}

#[v2k_test_support::retail_test]
fn retail_campaign_casualty_records_use_supported_signed_survivor_limits() {
    let data = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&data).expect("retail PRELOAD parses");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("high-resolution global overlay");
    let mut authored = Vec::new();
    for world in 13..=50 {
        session.load_level_by_id(world, 1).unwrap();
        let level = session.cache.level_desc().unwrap();
        let mut state = CampaignCasualtyState::default();
        for (index, record) in level.campaign_records.iter().enumerate() {
            if record.flags() & 0x84 != 0x84 {
                continue;
            }
            let threshold = record.data[0x19] as i8;
            authored.push((world, index, record.flags(), threshold));
            let mut frame = CampaignCasualtyFrame {
                census: Fun0042e210CapabilityCensus {
                    cap_0x400: 0,
                    cap_0x800: i32::from(threshold) + 1,
                    cap_0x08: 0,
                },
                ..quiet_frame()
            };
            // Saved and already-aborted entries must not consume the per-load
            // warning latch in any authored world.
            for (world_saved, abort_active) in [(true, false), (false, true)] {
                assert!(state
                    .evaluate(
                        &level.campaign_records,
                        CampaignCasualtyFrame {
                            world_saved,
                            abort_active,
                            ..frame
                        }
                    )
                    .unwrap()
                    .is_empty());
            }
            let mut cursor = CampaignSelectorCursor::new(frame, CampaignSelectorEntry::default());
            let mut cursor_state = state;
            let mut cursor_actions = Vec::new();
            loop {
                match cursor
                    .next_step(&mut cursor_state, &level.campaign_records)
                    .unwrap()
                {
                    CampaignSelectorStep::DirectText(text) => {
                        cursor_actions.push(CampaignCasualtyAction::DirectText(text))
                    }
                    CampaignSelectorStep::AbortRecord { record_index } => {
                        cursor_actions.push(CampaignCasualtyAction::AbortWorld { record_index })
                    }
                    CampaignSelectorStep::CheckRoute { .. } => (),
                    CampaignSelectorStep::Finished => break,
                    CampaignSelectorStep::AbortMainBase => panic!("no Main Base request"),
                    CampaignSelectorStep::RetryCurrentWorld => panic!("no wreck interior request"),
                    CampaignSelectorStep::VirusCriticalHint
                    | CampaignSelectorStep::VirusCriticalSound => panic!("a clean landscape"),
                }
            }
            assert_eq!(
                cursor_actions,
                [CampaignCasualtyAction::DirectText(
                    CampaignCasualtyText::OneMoreLoss
                )]
            );
            assert_eq!(
                state.evaluate(&level.campaign_records, frame).unwrap(),
                [CampaignCasualtyAction::DirectText(
                    CampaignCasualtyText::OneMoreLoss
                )],
                "world {world}: warning at threshold+1"
            );
            assert!(state
                .evaluate(&level.campaign_records, frame)
                .unwrap()
                .is_empty());
            frame.census.cap_0x800 -= 1;
            assert_eq!(
                state.evaluate(&level.campaign_records, frame).unwrap(),
                [
                    CampaignCasualtyAction::DirectText(CampaignCasualtyText::WorldLost),
                    CampaignCasualtyAction::AbortWorld {
                        record_index: index
                    },
                ],
                "world {world}: D4 precedes systemic abort at threshold"
            );
            // A fresh load re-arms the same authored warning. A skipped
            // warning count still reaches loss without relying on that latch.
            let mut reloaded = CampaignCasualtyState::default();
            assert_eq!(
                reloaded.evaluate(&level.campaign_records, frame).unwrap(),
                [
                    CampaignCasualtyAction::DirectText(CampaignCasualtyText::WorldLost),
                    CampaignCasualtyAction::AbortWorld {
                        record_index: index
                    },
                ]
            );
            frame.census.cap_0x400 = i32::from(threshold) + 1;
            frame.census.cap_0x800 = 0;
            assert_eq!(
                reloaded.evaluate(&level.campaign_records, frame).unwrap(),
                [CampaignCasualtyAction::DirectText(
                    CampaignCasualtyText::OneMoreLoss
                ),],
                "world{world}: scientists plus peasants are survivors"
            );
        }
    }
    assert_eq!(
        authored,
        [
            (13, 2, 0x86, 0),
            (14, 2, 0x84, 5),
            (15, 2, 0x84, 5),
            (16, 3, 0x84, 7),
            (17, 2, 0x84, 0),
            (18, 3, 0x84, 7),
            (19, 2, 0x84, 5),
            (21, 2, 0x84, 8),
            (24, 1, 0x84, 0),
            (26, 3, 0x84, 0),
            (31, 2, 0x84, 8),
            (32, 2, 0x84, 2),
            (34, 2, 0x84, 0),
            (35, 6, 0x84, 5),
            (36, 3, 0x84, 15),
            (37, 2, 0x84, 8),
            (39, 1, 0x84, 4),
            (40, 2, 0x84, 4),
            (42, 3, 0x84, 0),
            (43, 2, 0x84, 4),
            (46, 1, 0x84, 2),
        ]
    );
}

/// The warning cadence matches a retail world-36 lab run with its infected
/// count written: text `0xCB` strictly more than 160 ticks apart at eight
/// points, then `0xCB`, `0xD5` and the abort in the frame the limit is reached.
#[v2k_test_support::retail_test]
fn retail_virus_and_census_rows_lose_the_world_at_their_authored_limits() {
    let data = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&data).expect("retail PRELOAD parses");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("high-resolution global overlay");
    let mut virus_rows = Vec::new();
    let mut census_rows = Vec::new();
    for world in 13..=50 {
        session.load_level_by_id(world, 1).unwrap();
        let level = session.cache.level_desc().unwrap();
        // Far above every casualty limit, so only the row under test acts.
        let healthy = CampaignCasualtyFrame {
            census: Fun0042e210CapabilityCensus {
                cap_0x800: 100,
                ..Default::default()
            },
            world_clock: 1000,
            ..quiet_frame()
        };
        for (index, record) in level.campaign_records.iter().enumerate() {
            let flags = record.flags() & 0xff;
            if flags == 0x88 {
                census_rows.push((world, index, record.data[0x14]));
                let mut state = CampaignCasualtyState::default();
                let mut empty = healthy;
                empty.capability_buckets[usize::from(record.data[0x14])] = 0;
                assert_eq!(
                    state.evaluate(&level.campaign_records, empty).unwrap(),
                    [CampaignCasualtyAction::AbortWorld {
                        record_index: index
                    }],
                    "world {world}: an empty bucket loses without text"
                );
            }
            if flags != 0xa0 {
                continue;
            }
            let limit = i32::from(record.data[0x1a] as i8);
            virus_rows.push((world, index, limit));
            let at = |percent: i32, world_clock: i32| CampaignCasualtyFrame {
                virus_percent: Some(percent),
                world_clock,
                ..healthy
            };
            let mut state = CampaignCasualtyState::default();
            assert!(state
                .evaluate(&level.campaign_records, at(limit - 9, 1000))
                .unwrap()
                .is_empty());
            let warning = [
                CampaignCasualtyAction::VirusCriticalHint,
                CampaignCasualtyAction::DirectText(CampaignCasualtyText::VirusCritical),
                CampaignCasualtyAction::VirusCriticalSound,
            ];
            assert_eq!(
                state
                    .evaluate(&level.campaign_records, at(limit - 8, 1000))
                    .unwrap(),
                warning
            );
            // Eight points left: the text returns 160 ticks later.
            for clock in [1000, 1160] {
                assert_eq!(
                    state
                        .evaluate(&level.campaign_records, at(limit - 8, clock))
                        .unwrap(),
                    [CampaignCasualtyAction::VirusCriticalHint]
                );
            }
            assert_eq!(
                state
                    .evaluate(&level.campaign_records, at(limit - 8, 1161))
                    .unwrap(),
                warning
            );
            assert_eq!(
                state
                    .evaluate(&level.campaign_records, at(limit, 1161))
                    .unwrap(),
                [
                    CampaignCasualtyAction::VirusCriticalHint,
                    CampaignCasualtyAction::DirectText(CampaignCasualtyText::VirusWorldLost),
                    CampaignCasualtyAction::AbortWorld {
                        record_index: index
                    },
                ],
                "world {world}: D5 precedes the abort at the limit"
            );
            assert_eq!(
                CampaignCasualtyState::default()
                    .evaluate(&level.campaign_records, at(limit, 1000))
                    .unwrap(),
                [
                    CampaignCasualtyAction::VirusCriticalHint,
                    CampaignCasualtyAction::DirectText(CampaignCasualtyText::VirusCritical),
                    CampaignCasualtyAction::VirusCriticalSound,
                    CampaignCasualtyAction::DirectText(CampaignCasualtyText::VirusWorldLost),
                    CampaignCasualtyAction::AbortWorld {
                        record_index: index
                    },
                ]
            );
            // A stamped session silences the text but not the loss.
            let mut stamped = CampaignCasualtyState::default();
            assert_eq!(
                stamped
                    .evaluate(
                        &level.campaign_records,
                        CampaignCasualtyFrame {
                            results_stamped: true,
                            ..at(limit - 1, 1000)
                        }
                    )
                    .unwrap(),
                [CampaignCasualtyAction::VirusCriticalHint]
            );
        }
    }
    assert_eq!(
        virus_rows,
        [
            (13, 3, 20),
            (14, 3, 15),
            (15, 4, 25),
            (16, 4, 20),
            (17, 3, 40),
            (18, 2, 12),
            (19, 1, 15),
            (20, 5, 20),
            (21, 3, 25),
            (22, 1, 30),
            (24, 2, 40),
            (26, 1, 37),
            (31, 3, 28),
            (33, 2, 30),
            (34, 3, 30),
            (35, 5, 55),
            (36, 4, 50),
            (37, 6, 20),
            (38, 1, 20),
            (39, 2, 20),
            (40, 3, 20),
        ]
    );
    // Bucket 1 is capability 0x4000: each of these worlds has one Type106.
    assert_eq!(census_rows, [(37, 5, 1), (38, 2, 1), (40, 1, 1)]);
}

#[v2k_test_support::retail_test]
fn level_one_failed_interior_returns_level_one_before_real_next_world_records() {
    use v2k_game::campaign_transition::{
        authored_campaign_routes, CampaignTransition, FailedWorldRetry,
        RETAIL_CONTROLLER_DEFAULT_ARRIVAL,
    };

    let data = v2k_test_support::retail_dir();
    assert!(data.join("PRELOAD.DAT").is_file(), "required retail corpus");
    let mut session = GameSession::init(&data).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(13, 1).unwrap();
    let level = session.cache.level_desc().unwrap();
    let authored = authored_campaign_routes(
        13,
        level,
        session.cache.level_terrain().unwrap(),
        session.cache.terrain_objects().unwrap(),
    )
    .unwrap();
    assert!(!authored.is_empty());
    assert_ne!(authored[0].route.destination_level_id, 13);

    let frame = CampaignCasualtyFrame {
        abort_active: true,
        ..CampaignCasualtyFrame::default()
    };
    let mut casualty = CampaignCasualtyState::default();
    let mut cursor = CampaignSelectorCursor::new(
        frame,
        CampaignSelectorEntry {
            failed_world_interior_request: true,
            ..Default::default()
        },
    );
    assert_eq!(
        cursor.next_step(&mut casualty, &level.campaign_records),
        Ok(CampaignSelectorStep::RetryCurrentWorld)
    );
    let retry = CampaignTransition::FailedWorldRetry(
        FailedWorldRetry::new(13, RETAIL_CONTROLLER_DEFAULT_ARRIVAL.position_raw).unwrap(),
    );
    assert_eq!(retry.source_level_id(), 13);
    assert_eq!(retry.destination_level_id(), 13);
    assert_eq!(retry.destination_logical_level(), 1);
    assert_eq!(retry.arrival(), RETAIL_CONTROLLER_DEFAULT_ARRIVAL);
    assert_eq!(
        cursor.next_step(&mut casualty, &level.campaign_records),
        Ok(CampaignSelectorStep::Finished)
    );
    assert_eq!(casualty, CampaignCasualtyState::default());

    // With no inner request, the same failed-world frame retains normal
    // record visitation. Failure alone is not a blanket route prohibition.
    let mut cursor = CampaignSelectorCursor::new(frame, CampaignSelectorEntry::default());
    assert_eq!(
        cursor.next_step(&mut casualty, &level.campaign_records),
        Ok(CampaignSelectorStep::CheckRoute {
            record_index: authored[0].record_index,
        })
    );
}
