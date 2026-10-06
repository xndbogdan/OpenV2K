use v2k_formats::terrain::{TerrainCell, TerrainGrid, GRID_SIZE};
use v2k_game::gameplay_notifications::TextTypewriterCadence;
use v2k_game::infection_evolution::{infected_cell_count, INFECTION_TERRAIN_TYPE_BIT};
use v2k_game::world_complete_results::{
    fun_0042dd10_selector3_rescued, fun_0042dd10_selector5_natives_killed,
    fun_0042dd10_selector6_aliens_killed, fun_00452270_case1_minutes_and_seconds,
    fun_00452270_case_e_hides_line, landscape_virus_percent, session_music_should_play,
    Fun0042e210CapabilityCensus, WorldCompleteResultsRuntime, WorldCompleteStats,
    WorldCompleteTally, FUN_00455CC0_CONTINUE_SOUND_ID,
};

fn empty_terrain() -> TerrainGrid {
    TerrainGrid {
        header: [0, 0, 0, 0, 0],
        cells: vec![
            TerrainCell {
                height: 0,
                attribute: 0,
                terrain_type: 0,
            };
            GRID_SIZE * GRID_SIZE
        ],
    }
}

fn infect(terrain: &mut TerrainGrid, cell: [u8; 2]) {
    let index = usize::from(cell[0]) * GRID_SIZE + usize::from(cell[1]);
    terrain.cells[index].terrain_type |= INFECTION_TERRAIN_TYPE_BIT;
}

#[test]
fn overlay_51_time_trophy_blinks_on_odd_dat_004fed60_over_40() {
    assert!(fun_00452270_case_e_hides_line(0));
    assert!(fun_00452270_case_e_hides_line(39));
    assert!(!fun_00452270_case_e_hides_line(40));
    assert!(!fun_00452270_case_e_hides_line(79));
    assert!(fun_00452270_case_e_hides_line(80));
    let mut runtime = WorldCompleteResultsRuntime::default();
    runtime.record_hive_completion(WorldCompleteStats {
        ticks_0x2bc: 6_250,
        time_trophy: true,
        ..WorldCompleteStats::default()
    });
    runtime.advance(6_000_000);
    let resolve = |id| match id {
        0xbe => Some("\t     <5000,10000,*,15,17,90,30,14>Time trophy collected."),
        _ => None,
    };
    let hidden =
        runtime.presentation_with_typewriter(&mut TextTypewriterCadence::default(), 0, resolve);
    assert!(hidden.lines.is_empty());
    let shown =
        runtime.presentation_with_typewriter(&mut TextTypewriterCadence::default(), 40, resolve);
    assert_eq!(
        shown
            .lines
            .iter()
            .map(|line| (line.string_id, line.text.as_str()))
            .collect::<Vec<_>>(),
        vec![(0xbe, "Time trophy collected.")]
    );
}

#[test]
fn overlay_51_field7_empties_before_type_on_so_blink_has_no_cue() {
    let mut runtime = WorldCompleteResultsRuntime::default();
    runtime.record_hive_completion(WorldCompleteStats {
        ticks_0x2bc: 4845,
        time_trophy: true,
        ..WorldCompleteStats::default()
    });
    runtime.advance(5_150_000);
    let resolve = |id| match id {
        0xbe => Some("\t     <5000,10000,*,15,17,90,30,14>Time trophy collected."),
        _ => None,
    };
    let hidden =
        runtime.presentation_with_typewriter(&mut TextTypewriterCadence::default(), 0, resolve);
    assert!(hidden.lines.is_empty());
    assert!(!hidden.play_typewriter_sound);
    let shown =
        runtime.presentation_with_typewriter(&mut TextTypewriterCadence::default(), 40, resolve);
    assert_eq!(
        shown
            .lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>(),
        vec!["Time "]
    );
    assert!(shown.play_typewriter_sound);
}

#[test]
fn overlay_51_world_saved_time_uses_fun_00452270_case1() {
    assert_eq!(fun_00452270_case1_minutes_and_seconds(0), (0, 0));
    assert_eq!(fun_00452270_case1_minutes_and_seconds(49), (0, 0));
    assert_eq!(fun_00452270_case1_minutes_and_seconds(50), (0, 1));
    assert_eq!(fun_00452270_case1_minutes_and_seconds(2_999), (0, 59));
    assert_eq!(fun_00452270_case1_minutes_and_seconds(3_000), (1, 0));
    assert_eq!(fun_00452270_case1_minutes_and_seconds(6_250), (2, 5));
}

#[test]
fn fun_004567b0_stamps_dat_004fed60_once_when_abort_is_clear() {
    let mut tally = WorldCompleteTally::default();
    assert!(!tally.stamp_fun_004567b0(true, 6_250));
    assert_eq!(tally.ticks_0x2bc, 0);
    assert!(tally.stamp_fun_004567b0(false, 6_250));
    assert_eq!(tally.ticks_0x2bc, 6_250);
    assert!(!tally.stamp_fun_004567b0(false, 9_000));
    assert_eq!(tally.ticks_0x2bc, 6_250);
}

#[test]
fn overlay_51_stat_selectors_match_fun_0042dd10() {
    assert_eq!(fun_0042dd10_selector3_rescued(2, 7), 9);
    assert_eq!(fun_0042dd10_selector5_natives_killed(9, 2, 7), 0);
    assert_eq!(fun_0042dd10_selector5_natives_killed(9, 2, 4), 3);
    assert_eq!(fun_0042dd10_selector6_aliens_killed(12, 12), 0);
    assert_eq!(fun_0042dd10_selector6_aliens_killed(12, 5), 7);
    assert_eq!(fun_0042dd10_selector6_aliens_killed(5, 12), 0);
}

#[test]
fn overlay_51_recense_includes_sub_m_staff_in_selector3() {
    let mut tally = WorldCompleteTally::default();
    tally.snapshot_world_load_census(Fun0042e210CapabilityCensus {
        cap_0x400: 0,
        cap_0x800: 6,
        cap_0x08: 7,
    });
    tally.recense_fun_0042dd10(Fun0042e210CapabilityCensus {
        cap_0x400: 2,
        cap_0x800: 4,
        cap_0x08: 2,
    });
    assert_eq!(tally.natives_rescued, 6);
    assert_eq!(tally.natives_killed, 0);
    assert_eq!(tally.aliens_killed, 5);
}

#[test]
fn overlay_51_recense_fills_selectors_3_5_6_from_load_snapshot() {
    let mut tally = WorldCompleteTally::default();
    tally.snapshot_world_load_census(Fun0042e210CapabilityCensus {
        cap_0x400: 0,
        cap_0x800: 6,
        cap_0x08: 7,
    });
    assert_eq!(tally.load_natives_0x170, 6);
    assert_eq!(tally.load_aliens_0x16c, 7);
    tally.recense_fun_0042dd10(Fun0042e210CapabilityCensus {
        cap_0x400: 1,
        cap_0x800: 3,
        cap_0x08: 2,
    });
    assert_eq!(tally.natives_rescued, 4);
    assert_eq!(tally.natives_killed, 2);
    assert_eq!(tally.aliens_killed, 5);
}

#[test]
fn landscape_virus_percent_matches_fun_004366f0() {
    assert_eq!(landscape_virus_percent(0), 0);
    assert_eq!(landscape_virus_percent(1), 1);
    assert_eq!(landscape_virus_percent(95), 1);
    assert_eq!(landscape_virus_percent(0x1_0000), 87);
}

#[test]
fn overlay_51_virus_row_snapshots_the_live_infection_census() {
    let mut terrain = empty_terrain();
    terrain.cells[GRID_SIZE + 1].terrain_type = 0x08;
    for z in 0..95u8 {
        infect(&mut terrain, [0, z]);
    }
    assert_eq!(infected_cell_count(&terrain), 95);

    let mut tally = WorldCompleteTally::default();
    tally.stamp_fun_004567b0(false, 4845);
    let stats = tally.snapshot_stats(
        landscape_virus_percent(infected_cell_count(&terrain)),
        false,
        false,
        0,
    );
    let mut runtime = WorldCompleteResultsRuntime::default();
    runtime.record_hive_completion(stats);
    runtime.advance(3_500_000);
    let lines = runtime.presentation(|id| match id {
        0xc2 => Some(" <2500,10000,*,15,51,85,30,5>The landscape was %d%% virused."),
        _ => None,
    });
    assert_eq!(
        lines
            .iter()
            .map(|line| (line.string_id, line.text.as_str()))
            .collect::<Vec<_>>(),
        vec![(0xc2, "The landscape was 1% virused.")]
    );
}

#[test]
fn overlay_51_rows_type_on_with_field_six_interval() {
    let mut runtime = WorldCompleteResultsRuntime::default();
    runtime.record_hive_completion(
        WorldCompleteTally {
            ticks_0x2bc: 6_250,
            ..WorldCompleteTally::default()
        }
        .snapshot_stats(0, false, false, 0),
    );
    let resolve = |id| match id {
        0xbd => Some("        <   *,10000,*,10,10,90,30,1>World saved in %d:%02d."),
        _ => None,
    };
    assert!(runtime.presentation(resolve).is_empty());
    runtime.advance(150_000);
    let partial = runtime.presentation(resolve);
    assert_eq!(
        partial
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>(),
        vec!["World"]
    );
    runtime.advance(450_000);
    let complete = runtime.presentation(resolve);
    assert_eq!(
        complete
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>(),
        vec!["World saved in 2:05."]
    );
}

#[test]
fn overlay_51_type_on_uses_the_shared_fun_00452790_cue() {
    let mut runtime = WorldCompleteResultsRuntime::default();
    runtime.record_hive_completion(
        WorldCompleteTally {
            ticks_0x2bc: 6_250,
            ..WorldCompleteTally::default()
        }
        .snapshot_stats(0, false, false, 0),
    );
    let resolve = |id| match id {
        0xbd => Some("        <   *,10000,*,10,10,90,30,1>World saved in %d:%02d."),
        _ => None,
    };
    let mut cadence = TextTypewriterCadence::default();
    let at_open = runtime.presentation_with_typewriter(&mut cadence, 10, resolve);
    assert!(at_open.lines.is_empty());
    assert!(at_open.play_typewriter_sound);

    runtime.advance(600_000);
    let mut later = TextTypewriterCadence::default();
    let complete = runtime.presentation_with_typewriter(&mut later, 40, resolve);
    assert_eq!(
        complete
            .lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>(),
        vec!["World saved in 2:05."]
    );
    assert!(!complete.play_typewriter_sound);
}

fn ordinary_route() -> v2k_game::campaign_transition::CampaignWarpRoute {
    use v2k_game::campaign_transition::{CampaignWarpRoute, WarpArrival};
    CampaignWarpRoute {
        source_level_id: 13,
        marker_cell: [187, 129],
        marker_kind: 22,
        marker_subtype: 1,
        marker_args: [0, 0],
        destination_logical_level: 2,
        destination_level_id: 14,
        arrival: WarpArrival {
            position_raw: [17408, 2560, -32512],
            heading_raw: 0x4000,
        },
    }
}

#[test]
fn hive_death_keeps_hud_music_and_contact_available_until_exit() {
    let mut runtime = WorldCompleteResultsRuntime::default();
    runtime.record_hive_completion(WorldCompleteStats {
        ticks_0x2bc: 4845,
        ..WorldCompleteStats::default()
    });
    runtime.advance(8_000_000);
    assert!(runtime.has_hive_statistics());
    assert!(!runtime.is_progress_map_active());
    assert!(!runtime.replaces_ingame_hud());
    assert!(session_music_should_play(
        15,
        runtime.is_progress_map_active()
    ));
    assert!(runtime
        .continue_prompt_line(|_| Some("save prompt"))
        .is_none());
    assert_eq!(runtime.continue_progress_map(), None);
    assert!(runtime.has_hive_statistics());
}

#[test]
fn committed_route_opens_map_then_space_releases_exact_arrival_once() {
    let mut runtime = WorldCompleteResultsRuntime::default();
    runtime.record_hive_completion(WorldCompleteStats {
        ticks_0x2bc: 4845,
        ..WorldCompleteStats::default()
    });
    let route = ordinary_route();
    let transition = v2k_game::campaign_transition::CampaignTransition::AuthoredMarker(route);
    runtime.open_progress_map(transition);
    assert!(runtime.is_progress_map_active());
    assert!(runtime.replaces_ingame_hud());
    assert!(!runtime.has_hive_statistics());
    assert!(runtime.presentation(|_| Some("statistics")).is_empty());
    assert!(!session_music_should_play(
        15,
        runtime.is_progress_map_active()
    ));
    assert_eq!(FUN_00455CC0_CONTINUE_SOUND_ID, 3);
    assert_eq!(runtime.continue_progress_map(), Some(transition));
    assert!(!runtime.is_progress_map_active());
    assert_eq!(runtime.continue_progress_map(), None);
    assert!(session_music_should_play(
        15,
        runtime.is_progress_map_active()
    ));
}

#[test]
fn completion_rows_use_the_retail_saved_tick_gate() {
    for (tick, shown) in [(0, false), (500, false), (501, true)] {
        let mut runtime = WorldCompleteResultsRuntime::default();
        runtime.record_hive_completion(WorldCompleteStats {
            ticks_0x2bc: tick,
            ..WorldCompleteStats::default()
        });
        assert_eq!(runtime.has_hive_statistics(), shown);
        assert!(!runtime.replaces_ingame_hud());
    }
}

#[test]
fn progress_map_occupancy_pauses_cd_music() {
    assert!(session_music_should_play(15, false));
    assert!(session_music_should_play(1, false));
    assert!(!session_music_should_play(0, false));
    assert!(!session_music_should_play(15, true));
    assert!(!session_music_should_play(1, true));
    assert!(!session_music_should_play(0, true));
}

#[test]
fn overlay_51_backdrop_table_covers_each_tile_once() {
    use v2k_game::overlay_51_backdrop::{
        overlay_51_tile_origin, OVERLAY_51_BACKDROP_TILES, OVERLAY_51_FIRST_SPRITE_ID,
        OVERLAY_51_LAST_TILE_SPRITE_ID, OVERLAY_51_TILE_MARKER_SPRITE_ID,
    };

    assert_eq!(OVERLAY_51_BACKDROP_TILES.len(), 30);
    assert_eq!(OVERLAY_51_TILE_MARKER_SPRITE_ID, 3765);
    assert_eq!(0x3ad4 / 4, u32::from(OVERLAY_51_TILE_MARKER_SPRITE_ID));
    let mut sprites: Vec<u16> = OVERLAY_51_BACKDROP_TILES
        .iter()
        .map(|(_, sprite)| *sprite)
        .collect();
    sprites.sort_unstable();
    assert_eq!(
        sprites,
        (OVERLAY_51_FIRST_SPRITE_ID..=OVERLAY_51_LAST_TILE_SPRITE_ID).collect::<Vec<_>>()
    );
    let mut locals: Vec<usize> = OVERLAY_51_BACKDROP_TILES
        .iter()
        .map(|(local, _)| *local)
        .collect();
    locals.sort_unstable();
    assert_eq!(locals, (0..30).collect::<Vec<_>>());
    assert_eq!(overlay_51_tile_origin((19, 4), (640, 432)), (121, 17));
}

#[test]
fn overlay_51_fun_00454ff0_hides_unsaved_worlds_and_blinks_the_current_slot() {
    use v2k_game::overlay_51_backdrop::{
        overlay_51_campaign_slot, overlay_51_flag_box_visible, overlay_51_object_flags,
        overlay_51_object_visible, overlay_51_s1_local,
    };
    use v2k_game::power_up_contact::PlayerCampaignProgress;

    assert_eq!(overlay_51_campaign_slot(1), Some(1));
    assert_eq!(overlay_51_s1_local(48), Some(0));
    assert_eq!(overlay_51_campaign_slot(13), Some(0));
    assert_eq!(overlay_51_campaign_slot(14), Some(14));

    let mut progress = PlayerCampaignProgress::new();
    progress.set_current_control_slot(Some(1));
    let current = overlay_51_object_flags(1, &progress);
    assert_eq!(current, 0x11);
    assert!(!overlay_51_object_visible(current, 0));
    assert!(overlay_51_object_visible(current, 100));
    assert!(!overlay_51_object_visible(current, 200));
    assert!(!overlay_51_object_visible(
        overlay_51_object_flags(2, &progress),
        100
    ));
    assert!(overlay_51_object_visible(6, 0));
    assert!(overlay_51_object_visible(6, 100));
    assert!(overlay_51_flag_box_visible(current));
    assert!(!overlay_51_flag_box_visible(6));
}

#[v2k_test_support::retail_test]
fn overlay_51_backdrop_loads_from_selected_tier() {
    use v2k_game::overlay_51_backdrop::{
        overlay_51_flag_extra_origin, overlay_51_pair_graph_from_linkage, Overlay51Backdrop,
        OVERLAY_51_BACKDROP_TILES, OVERLAY_51_FLAG_BOX_SPRITE_ID,
        OVERLAY_51_HIDDEN_TROPHY_OFFSET_LOCAL, OVERLAY_51_PAIR_CAP_SPRITE_ID,
        OVERLAY_51_PAIR_RIBBON_SPRITE_ID, OVERLAY_51_SCALE_LOCAL_INDEX,
        OVERLAY_51_TILE_MARKER_SPRITE_ID,
    };
    use v2k_game::power_up_contact::PlayerCampaignProgress;
    use v2k_game::session::GameSession;
    use v2k_render::WorldSpriteBlend;

    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).expect("init");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("high-res overlay 3");
    session
        .load_auxiliary_ovl(51, 1)
        .expect("high-res overlay 51");
    assert_eq!(
        session
            .cache
            .system_layout_point(3, OVERLAY_51_SCALE_LOCAL_INDEX),
        Some((640, 432))
    );
    let backdrop = Overlay51Backdrop::from_cache(&session.cache).expect("backdrop");
    assert_eq!(backdrop.tiles().len(), OVERLAY_51_BACKDROP_TILES.len());
    assert_eq!(backdrop.tiles()[0].origin, (121, 17));
    // RGB555 rectangle width is 128 bytes, representing 64 authored pixels.
    assert_eq!(backdrop.tiles()[0].width, 64);
    assert_eq!(backdrop.tiles()[0].height, 48);
    let marker = backdrop.marker();
    assert_eq!(OVERLAY_51_TILE_MARKER_SPRITE_ID, 3765);
    assert_eq!((marker.width, marker.height), (66, 50));
    assert_eq!(marker.blend, WorldSpriteBlend::Masked);
    let (atlas, entry) = session
        .cache
        .global_sprite(OVERLAY_51_TILE_MARKER_SPRITE_ID)
        .expect("sprite 3765");
    assert_eq!(entry.pal_size as u8, 0x05);
    assert_eq!(atlas.entries.last().unwrap().index, 3765);
    let extra = backdrop.flag_extra().expect("sprite 532");
    assert_eq!(
        extra.hidden_offset,
        session
            .cache
            .system_layout_point(3, OVERLAY_51_HIDDEN_TROPHY_OFFSET_LOCAL)
            .expect("overlay-3 S1 31")
    );
    assert_eq!(extra.hidden_offset, (-27, -14));
    assert_eq!(extra.time_offset, (27, -14));
    assert_eq!((extra.width, extra.height), (12, 12));
    assert_eq!(extra.blend, WorldSpriteBlend::Masked);
    assert_eq!(
        overlay_51_flag_extra_origin(
            backdrop.tiles()[0].origin,
            (backdrop.tiles()[0].width, backdrop.tiles()[0].height),
            extra.hidden_offset,
            (extra.width, extra.height),
        ),
        (120, 21)
    );
    let flag_box = backdrop.flag_box().expect("sprite 583");
    assert_eq!(OVERLAY_51_FLAG_BOX_SPRITE_ID, 583);
    assert_eq!(0x91c / 4, u32::from(OVERLAY_51_FLAG_BOX_SPRITE_ID));
    assert_eq!((flag_box.width, flag_box.height), (2, 2));
    assert_eq!(flag_box.blend, WorldSpriteBlend::HalfAdditive);
    let (cap_atlas, cap_entry) = session
        .cache
        .global_sprite(OVERLAY_51_PAIR_CAP_SPRITE_ID)
        .expect("sprite 3763");
    assert_eq!(cap_entry.index, 3763);
    assert_eq!(0x3acc / 4, u32::from(OVERLAY_51_PAIR_CAP_SPRITE_ID));
    let _ = cap_atlas;
    let (_, ribbon_entry) = session
        .cache
        .global_sprite(OVERLAY_51_PAIR_RIBBON_SPRITE_ID)
        .expect("sprite 3764");
    assert_eq!(ribbon_entry.index, 3764);
    assert_eq!(0x3ad0 / 4, u32::from(OVERLAY_51_PAIR_RIBBON_SPRITE_ID));
    let graph = overlay_51_pair_graph_from_linkage(
        session
            .cache
            .system_linkage(3)
            .expect("overlay-3 Section 14"),
    )
    .expect("S14 record 3");
    assert_eq!(graph[0][0], 18);
    assert_eq!(graph[0][1], 2);
    assert!(backdrop
        .visible_pair_links(&PlayerCampaignProgress::new())
        .is_empty());
}

#[test]
fn overlay_51_continue_prompt_uses_fun_00454390_layout() {
    use v2k_game::world_complete_results::{
        RESULTS_CONTINUE_BASELINE_PERCENT, RESULTS_CONTINUE_TEXT_ID,
    };

    let mut runtime = WorldCompleteResultsRuntime::default();
    assert!(runtime
        .continue_prompt_line(|_| Some("Press S to Save, Space to Continue"))
        .is_none());

    runtime.open_progress_map(
        v2k_game::campaign_transition::CampaignTransition::AuthoredMarker(ordinary_route()),
    );
    let line = runtime
        .continue_prompt_line(|_| Some("Press S to Save, Space to Continue"))
        .expect("open card owns the continue line");
    assert_eq!(line.string_id, RESULTS_CONTINUE_TEXT_ID);
    assert_eq!(line.text, "Press S to Save, Space to Continue");
    assert_eq!(line.baseline_percent, RESULTS_CONTINUE_BASELINE_PERCENT);
    assert_eq!(RESULTS_CONTINUE_BASELINE_PERCENT, 95);
    assert!(line.center_x);

    runtime.continue_progress_map();
    assert!(runtime
        .continue_prompt_line(|_| Some("Press S to Save, Space to Continue"))
        .is_none());
}
