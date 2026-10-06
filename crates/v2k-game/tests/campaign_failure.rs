//! Authored survivor limits and the warning/loss pair observed in NoCD05.

use v2k_game::campaign_failure::{
    CampaignCasualtyAction, CampaignCasualtyFrame, CampaignCasualtyState, CampaignCasualtyText,
    CampaignSelectorCursor, CampaignSelectorEntry, CampaignSelectorStep,
};
use v2k_game::session::GameSession;
use v2k_game::world_complete_results::Fun0042e210CapabilityCensus;

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
                world_saved: false,
                abort_active: false,
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
                    CampaignSelectorStep::AbortCasualties { record_index } => {
                        cursor_actions.push(CampaignCasualtyAction::AbortWorld { record_index })
                    }
                    CampaignSelectorStep::CheckRoute { .. } => (),
                    CampaignSelectorStep::Finished => break,
                    CampaignSelectorStep::AbortMainBase => panic!("no Main Base request"),
                    CampaignSelectorStep::RetryCurrentWorld => panic!("no wreck interior request"),
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
        census: Fun0042e210CapabilityCensus::default(),
        world_saved: false,
        abort_active: true,
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
