//! Integration tests: global resource pools + data-driven menu against the
//! real game data (ignored when no retail install is configured).

use std::collections::HashSet;

use v2k_formats::models::{face_material, AnimVars, ModelEdge, ModelEdgeStyle};
use v2k_formats::palette::BRIGHTEST_SHADE;
use v2k_game::entity::EntityManager;
use v2k_game::game_state::{MenuCtx, MenuLayout, MenuShell};
use v2k_game::menu_data::{SettingId, DISPLAY, SOUNDS};
use v2k_game::menu_text::{MenuFonts, GLYPH_BAR_FILLED};
use v2k_game::session::GameSession;
use v2k_render::GameConfig;

fn session_with_pools() -> GameSession {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).expect("retail fixture must load");
    session
        .load_auxiliary_ovl(3, 0)
        .expect("retail fixture must load");
    session
}

fn session_with_high_menu_pools() -> GameSession {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).expect("retail fixture must load");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("retail fixture must load");
    session
        .load_auxiliary_ovl(2, 1)
        .expect("retail fixture must load");
    session
        .load_auxiliary_ovl(5, 1)
        .expect("retail fixture must load");
    session
}

#[v2k_test_support::retail_test]
fn selected_menu_tiers_decode_raw_copyright_and_level_thumbnails() {
    let dir = v2k_test_support::retail_dir();
    for tier in 0..4 {
        let mut session = GameSession::init(&dir).expect("load PRELOAD");
        for level in [3, 5, 51] {
            session
                .load_auxiliary_ovl(level, tier)
                .expect("load selected tier");
        }
        let menu = v2k_game::menu::MenuResources::from_menu_ovl(
            session.cache.menu_graphics_ovl().expect("menu graphics"),
        );
        let banner = menu.copyright_banner.expect("copyright banner");
        let expected = if tier == 0 { (155, 14) } else { (309, 28) };
        assert_eq!((banner.width, banner.height), expected, "tier {tier}");
        assert!(banner.rgba.chunks_exact(4).any(|pixel| pixel[3] == 0));
        assert!(banner.rgba.chunks_exact(4).any(|pixel| pixel[3] == 255));

        let backdrop = v2k_game::overlay_51_backdrop::Overlay51Backdrop::from_cache(&session.cache)
            .expect("level selection backdrop");
        assert_eq!(backdrop.tiles().len(), 30);
        let expected = if tier == 0 { (32, 24) } else { (64, 48) };
        for tile in backdrop.tiles() {
            assert_eq!(
                (tile.width, tile.height),
                expected,
                "tier {tier}, sprite {}",
                tile.sprite_id
            );
            assert!(tile.rgba.chunks_exact(4).all(|pixel| pixel[3] == 255));
        }
        assert!(
            backdrop.tiles().iter().any(|tile| tile
                .rgba
                .chunks_exact(4)
                .any(|pixel| pixel == [0, 0, 0, 255])),
            "raw black remains opaque"
        );
    }
}

#[v2k_test_support::retail_test]
fn menu_depth_fade_color_is_authored_red_in_both_presentation_tiers() {
    let low =
        GameSession::init(&v2k_test_support::retail_dir()).expect("retail PRELOAD must parse");
    let high = session_with_high_menu_pools();

    for (tier, session) in [("low", &low), ("high", &high)] {
        let (level, local_index, entry) = session
            .cache
            .global_palette_entry_with_source(55)
            .unwrap_or_else(|| panic!("{tier} menu depth-fade color"));
        assert_eq!((level, local_index), (2, 55));
        assert_eq!(entry.rgb555, 0x7C00, "{tier} RGB555 terminal color");
        assert_eq!([entry.r, entry.g, entry.b], [248, 0, 0]);
    }
}

#[v2k_test_support::retail_test]
fn global_string_pool_assembles() {
    let session = session_with_pools();
    // L2 (121) + L3 (182) + L4 (2) + L5 (5) = 310 strings.
    assert_eq!(session.cache.global_strings().len(), 310);
    // L2 entries.
    assert_eq!(
        session.cache.global_string(25),
        Some("Press S to Save, Space to Continue")
    );
    assert_eq!(session.cache.global_string(26), Some("New Game"));
    assert_eq!(session.cache.global_string(66), Some("Exit"));
    // L3 zone starts at 121.
    assert!(session.cache.global_string(124).is_some());
    // Past the end.
    assert_eq!(session.cache.global_string(310), None);
}

#[v2k_test_support::retail_test]
fn fuel_notification_strings_preserve_authored_prefix_and_substitution() {
    let session = session_with_pools();

    assert_eq!(
        session.cache.global_string(0xcd),
        Some("\t\t<*,3000, 4,30,10>%s")
    );
    assert_eq!(
        session.cache.global_string(0xdc),
        Some("\t\t<*,3000, 4,30, *>Your fuel tanks are full")
    );
    assert_eq!(
        session.cache.global_string(0xec),
        Some("\t<*,3000, 1,30, *>You cannot fly without any fuel")
    );
    assert_eq!(
        session.cache.global_string(0xed),
        Some("\t\t<*,3000, 1,30, *>Try changing to flight mode now")
    );
    assert_eq!(session.cache.global_string(0x114), Some("Extra Fuel"));
}

#[v2k_test_support::retail_test]
fn type9_people_left_and_hull_warning_strings_match_section_two() {
    let session = session_with_pools();
    assert_eq!(
        session.cache.global_string(0xc6),
        Some("\t<*,3000, 4,30, 2>People left: %d")
    );
    assert_eq!(
        session.cache.global_string(0xdf),
        Some("\t\t<*,3000,14, *,15>Your ship is critically damaged")
    );
}

#[v2k_test_support::retail_test]
fn hive_gate_notification_strings_match_section_two() {
    let session = session_with_high_menu_pools();
    assert_eq!(
        session.cache.global_string(0xd2),
        Some("\t\t<*,3000, 4,30, *>The Hive is now vulnerable")
    );
    assert_eq!(
        session.cache.global_string(0xeb),
        Some(
            "\t\t<*,3000, 1,30, *>The hive can only be destroyed once the alien creatures are dead"
        )
    );
    assert_eq!(
        session.cache.global_string(0xef),
        Some("\t<*,4000, 1,30, *>It is now possible to destroy the alien hive")
    );
    assert_eq!(
        session.cache.global_string(0xe4),
        Some("\t\t<*,3000, 1,30, *>Fly down the hive to go to the next world")
    );
}

#[v2k_test_support::retail_test]
fn world_complete_results_strings_match_section_two() {
    let session = session_with_high_menu_pools();
    assert_eq!(
        session.cache.global_string(0xbd),
        Some("        <   *,10000,*,10,10,90,30,1>World saved in %d:%02d.")
    );
    assert_eq!(
        session.cache.global_string(0xbe),
        Some("\t     <5000,10000,*,15,17,90,30,14>Time trophy collected.")
    );
    assert_eq!(
        session.cache.global_string(0xbf),
        Some(" <1000,10000,*,15,30,85,30,2>You rescued %d natives.")
    );
    assert_eq!(
        session.cache.global_string(0xc0),
        Some(" <1500,10000,*,15,37,85,30,3>%d natives were killed.")
    );
    assert_eq!(
        session.cache.global_string(0xc1),
        Some(" <2000,10000,*,15,44,85,30,4>You killed %d alien creatures.")
    );
    assert_eq!(
        session.cache.global_string(0xc2),
        Some(" <2500,10000,*,15,51,85,30,5>The landscape was %d%% virused.")
    );
    assert_eq!(
        session.cache.global_string(0xc3),
        Some(" <3000,10000,*,15,58,85,30,13>Your rank is %s.")
    );
    assert_eq!(
        session.cache.global_string(0xc4),
        Some(" <3500,10000,*,15,65,85,30,12>You found the hidden trophy.")
    );
}

#[v2k_test_support::retail_test]
fn world_complete_results_present_authored_section_two_rows() {
    let session = session_with_high_menu_pools();
    let mut runtime = v2k_game::world_complete_results::WorldCompleteResultsRuntime::default();
    runtime.record_hive_completion(v2k_game::world_complete_results::WorldCompleteStats {
        ticks_0x2bc: 6_250,
        natives_rescued: 3,
        natives_killed: 0,
        aliens_killed: 7,
        virus_percent: 0,
        time_trophy: false,
        hidden_trophy: false,
        rank_name: "Absolute Beginner",
    });
    assert!(runtime
        .presentation(|id| session.cache.global_string(id))
        .is_empty());

    runtime.advance(600_000);
    let first = runtime.presentation(|id| session.cache.global_string(id));
    assert_eq!(
        first
            .iter()
            .map(|line| (line.string_id, line.text.as_str()))
            .collect::<Vec<_>>(),
        vec![(0xbd, "World saved in 2:05.")]
    );

    runtime.advance(3_400_000);
    let mid = runtime.presentation(|id| session.cache.global_string(id));
    assert_eq!(
        mid.iter()
            .map(|line| (line.string_id, line.text.as_str()))
            .collect::<Vec<_>>(),
        vec![
            (0xbd, "World saved in 2:05."),
            (0xbf, "You rescued 3 natives."),
            (0xc0, "0 natives were killed."),
            (0xc1, "You killed 7 alien creatures."),
            (0xc2, "The landscape was 0% virused."),
            (0xc3, "Your rank is Absolute Beginner."),
        ]
    );
}

#[v2k_test_support::retail_test]
fn static_pickup_notification_strings_match_controller_operations() {
    let session = session_with_pools();
    assert_eq!(
        session.cache.global_string(0xcd),
        Some("\t\t<*,3000, 4,30,10>%s")
    );
    assert_eq!(
        session.cache.global_string(0xdd),
        Some("\t\t<*,3000, 4,30, *>Your ship has no damage")
    );
    assert_eq!(session.cache.global_string(0x110), Some("Shields"));
    assert_eq!(
        session.cache.global_string(0xee),
        Some("\t\t<*,3000, 1,30, *>This repairs some of the damage to your ship")
    );
}

#[v2k_test_support::retail_test]
fn global_sound_pool_maps_post_intro_sample_to_level_three_entry_43() {
    use v2k_formats::anim_sound::EntryType;

    let session = session_with_pools();
    let tables = session.cache.global_sound_tables();

    // DAT_004FE64C concatenates level 2's seven entries with level 3's 103.
    // Therefore global id 50 is level-3 local entry 43, not local entry 50.
    assert_eq!(tables.len(), 2);
    assert_eq!(tables[0].entries.len(), 7);
    assert_eq!(tables[1].entries.len(), 103);
    assert_eq!(
        tables
            .iter()
            .map(|table| table.entries.len())
            .sum::<usize>(),
        110
    );

    let post_intro = &tables[1].entries[50 - tables[0].entries.len()];
    assert_eq!(post_intro.index, 43);
    assert_eq!(post_intro.entry_type, EntryType::DataBlob);
    assert_eq!(tables[1].blob_data(post_intro).unwrap().len(), 62_700);
}

#[v2k_test_support::retail_test]
fn global_model_pool_menu_props() {
    let session = session_with_pools();
    // L5 menu props occupy global ids 315-323 (MENU_SYSTEM.md §Layer 2).
    let expect = [
        (315usize, "flags"),
        (316, "screenop"),
        (317, "screeno2"),
        (318, "sfxopt"),
        (320, "slopt"),
        (322, "psjoypad"),
        (323, "multipc"),
        // L3 props.
        (41, "player4"),
        (75, "optexit"),
    ];
    for (id, name) in expect {
        let model = session
            .cache
            .global_model(id)
            .unwrap_or_else(|| panic!("no model {id}"));
        assert_eq!(model.name.as_deref(), Some(name), "global model id {id}");
    }
    assert!(session.cache.global_model(324).is_none());

    // Section-8 retains the retail rasterizers' ordinary full-rectangle UVs.
    // Exit and Save expose the rule on both sides. Their screen-readable
    // handedness is supplied by the menu scene's local model basis, not by
    // mutating the parsed texture corners.
    for (id, name) in [(75usize, "optexit"), (320, "slopt")] {
        let model = session.cache.global_model(id).unwrap();
        assert_eq!(model.name.as_deref(), Some(name));
        assert_eq!(
            model.face_uvs[0],
            [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]],
            "{name} front face must retain retail quad corners"
        );
        assert_eq!(
            model.face_uvs[1],
            [[0.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            "{name} front quad must remain continuous"
        );
        let last = model.face_uvs.len() - 1;
        assert_eq!(
            model.face_uvs[last - 1],
            [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]],
            "{name} reverse face must use the same retail quad policy"
        );
        assert_eq!(
            model.face_uvs[last],
            [[0.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            "{name} reverse quad must remain continuous"
        );
    }
}

#[v2k_test_support::retail_test]
fn multipc_retains_authored_network_cable_ribbons() {
    let session = session_with_high_menu_pools();
    let model = session.cache.global_model(323).expect("multipc model");
    assert_eq!(model.name.as_deref(), Some("multipc"));
    assert!(model.instances.is_empty());
    assert!(model.billboards.is_empty());
    assert_eq!(
        model.edges.as_slice(),
        &[
            ModelEdge {
                vertices: [60, 1],
                style: ModelEdgeStyle::Sprite {
                    sprite_id: 608,
                    size: 12,
                },
            },
            ModelEdge {
                vertices: [60, 17],
                style: ModelEdgeStyle::Sprite {
                    sprite_id: 608,
                    size: 12,
                },
            },
            ModelEdge {
                vertices: [60, 8],
                style: ModelEdgeStyle::Sprite {
                    sprite_id: 608,
                    size: 12,
                },
            },
        ],
        "the three rotating links are authored 0x22 geometry, not stray UI art"
    );
    for (index, expected) in [
        (60, [0.0, 18.0, 0.0]),
        (1, [79.0, 0.0, 64.0]),
        (17, [-96.0, 0.0, 33.0]),
        (8, [44.0, 0.0, -91.0]),
    ] {
        assert_eq!(model.vertices[index], expected);
    }
    assert_eq!(
        [60usize, 1, 17, 8].map(|index| model.vertex_type_flags[index]),
        [3, 0, 0, 0],
        "the shared hub is the authored type-3 root; branch tips are ordinary"
    );

    let (atlas, entry) = session
        .cache
        .global_sprite(608)
        .expect("network cable sprite");
    assert_eq!((entry.flags as u16, (entry.flags >> 16) as u16), (64, 16));
    assert_eq!(entry.pal_size, 0x05);
    assert!(entry.is_zero_keyed());
    let decoded = atlas
        .decode_indices(entry)
        .expect("decode network cable sprite");
    assert_eq!((decoded.width, decoded.height), (64, 16));
}

#[v2k_test_support::retail_test]
fn selected_high_detail_menu_pack_shadows_preload_prop_textures() {
    let low =
        GameSession::init(&v2k_test_support::retail_dir()).expect("retail PRELOAD must parse");
    let (low_atlas, low_save_face) = low
        .cache
        .global_sprite(1305)
        .expect("PRELOAD save-prop face");
    let low_save_face = low_atlas
        .decode_indices(low_save_face)
        .expect("decode PRELOAD save-prop face");
    assert_eq!(
        (low_save_face.width, low_save_face.height),
        (32, 32),
        "PRELOAD is the authored 320x240 presentation tier"
    );
    let (low_atlas, low_common_face) = low
        .cache
        .global_sprite(389)
        .expect("PRELOAD common model face");
    let low_common_face = low_atlas
        .decode_indices(low_common_face)
        .expect("decode PRELOAD common model face");
    assert_eq!((low_common_face.width, low_common_face.height), (32, 32));

    let high = session_with_high_menu_pools();
    let menu_source = high
        .cache
        .menu_graphics_ovl()
        .and_then(|level| std::path::Path::new(&level.source_path).file_name())
        .and_then(|name| name.to_str());
    assert_eq!(menu_source, Some("1X5XX.OVL"));

    let (high_atlas, high_save_face) = high
        .cache
        .global_sprite(1305)
        .expect("high-detail save-prop face");
    let high_save_face = high_atlas
        .decode_indices(high_save_face)
        .expect("decode high-detail save-prop face");
    assert_eq!(
        (high_save_face.width, high_save_face.height),
        (64, 64),
        "the save prop must not upscale PRELOAD's 32x32 artwork"
    );
    let (high_atlas, high_common_face) = high
        .cache
        .global_sprite(389)
        .expect("high-detail common model face");
    let high_common_face = high_atlas
        .decode_indices(high_common_face)
        .expect("decode high-detail common model face");
    assert_eq!(
        (high_common_face.width, high_common_face.height),
        (64, 64),
        "gameplay models must also resolve the selected level-2 texture tier"
    );
    assert_eq!(
        high.cache.global_model(320).unwrap().name.as_deref(),
        Some("slopt")
    );
}

#[v2k_test_support::retail_test]
fn level_one_factory_emblem_keeps_source_orientation() {
    let session = session_with_pools();

    // Level-1 `lifter` is local 214 after the thirteen PRELOAD models, hence
    // global 227. Sprite 390 is the asymmetric golden factory emblem: its
    // bird head is authored pointing right. Its parsed quad retains the
    // rasterizer's ordinary U direction; scene/model transforms own any
    // presentation handedness.
    let factory = session.cache.global_model(227).expect("Level-1 lifter");
    assert_eq!(factory.name.as_deref(), Some("lifter"));
    let emblem_faces = factory
        .face_materials
        .iter()
        .enumerate()
        .filter_map(|(index, &packed)| {
            (face_material(packed) == (390, true)).then_some(factory.face_uvs[index])
        })
        .collect::<Vec<_>>();
    assert_eq!(emblem_faces.len(), 4);
    assert!(emblem_faces
        .iter()
        .all(|uvs| uvs[0][0] == 0.0 && uvs[1][0] == 1.0));
}

#[v2k_test_support::retail_test]
fn options_panel_keeps_both_authored_halves_and_front_face_materials() {
    let session = session_with_pools();

    // Global model 73 (`optionsh`) has no body of its own. It mirrors two
    // copies of global model 74 (`anim`) at the left/right attachment points.
    // The latter owns every visual the incomplete back-facing port draw had
    // lost: emblems, dark-green glass, hazard tape, and inner frame.
    let optionsh = session.cache.global_model(73).expect("optionsh model");
    assert_eq!(optionsh.name.as_deref(), Some("optionsh"));
    let wrapper = optionsh.materialize(&AnimVars::default());
    assert_eq!(wrapper.instances.len(), 2);
    assert!(wrapper
        .instances
        .iter()
        .all(|instance| instance.model_id == 74));
    assert_eq!(
        wrapper
            .instances
            .iter()
            .map(|instance| instance.attach_pos.expect("optionsh attachment")[0] as i32)
            .collect::<Vec<_>>(),
        [-2250, 2250]
    );

    let panel_half = session.cache.global_model(74).expect("optionsh half");
    assert_eq!(panel_half.name.as_deref(), Some("anim"));
    let materials = panel_half
        .materialize(&AnimVars::default())
        .face_materials
        .into_iter()
        .collect::<HashSet<_>>();
    for (packed, role) in [
        (0x8186, "gold V emblem"),
        (0x8274, "dark-green panel fill"),
        (0x849c, "yellow/black hazard border"),
        (0x849f, "inner panel frame"),
    ] {
        assert!(materials.contains(&packed), "missing {role} material");
    }
}

#[v2k_test_support::retail_test]
fn player_and_main_base_keep_authored_palette_zero_opaque() {
    let session = session_with_pools();

    // `player4` and `college` use the ordinary 0x04 fixed-row filler family.
    // Their palette-zero texels are model surface colour, not holes. These are
    // the exact affected sources visible in the retail comparison captures.
    for sprite_id in [389u16, 390, 1184, 1185, 1190, 1203, 1206] {
        let (atlas, entry) = session
            .cache
            .global_sprite(sprite_id)
            .unwrap_or_else(|| panic!("missing model sprite {sprite_id}"));
        assert_eq!(entry.pal_size & 0x01, 0, "sprite {sprite_id} zero-key");
        assert_ne!(
            atlas
                .decode_indices(entry)
                .expect("model sprite indices")
                .indices
                .iter()
                .position(|&index| index == 0),
            None,
            "sprite {sprite_id} should exercise authored palette zero"
        );
        let decoded = atlas
            .decode_sprite(entry, BRIGHTEST_SHADE)
            .expect("model sprite RGBA");
        assert!(
            decoded.rgba.chunks_exact(4).all(|pixel| pixel[3] == 255),
            "sprite {sprite_id} must decode as an opaque surface"
        );
    }
}

#[v2k_test_support::retail_test]
fn intro2_uses_the_retail_medieval_resource_pack() {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).unwrap();
    session.load_auxiliary_ovl(3, 0).unwrap();
    // Exercise stale-pack removal: level 20 selects pack 8, which must be
    // completely gone once Intro2's authored pack 6 becomes active.
    session.load_level_by_id(20, 0).unwrap();
    assert!(session.cache.status().contains("0X8XX"));
    session.load_level_by_id(50, 0).unwrap();
    let status = session.cache.status();
    assert!(!status.contains("0X8XX"), "stale Intro2 pack 8:\n{status}");
    assert!(status.contains("0X6XX"), "missing Intro2 pack 6:\n{status}");

    // The retail full-session trace loads 0X6XX beside 0X50XX. Compare the
    // active Section-9 table itself so a water auxiliary cannot accidentally
    // make the model-only assertion below pass while a later wrong biome still
    // supplies palms, pyramids, and hazard-strip terrain props.
    let medieval = session.load_ovl_by_id(6, 0).unwrap();
    let expected_objects = medieval.anim_frames.as_ref().unwrap();
    let active_objects = session.cache.terrain_objects().unwrap();
    assert_eq!(active_objects.records.len(), expected_objects.records.len());
    for (active, expected) in active_objects.records.iter().zip(&expected_objects.records) {
        assert_eq!(active.model_ids, expected.model_ids);
        assert_eq!(active.kind_index, expected.kind_index);
    }

    // Intro2's four meteor spawns override their model to global id 560.
    // With its active world-resource pack appended, that is L6's `grock` model.
    assert_eq!(
        session
            .cache
            .global_model(560)
            .and_then(|m| m.name.as_deref()),
        Some("grock")
    );
    for name in ["pesnthut", "dpsnthut", "windmill", "windmillblades"] {
        assert!(
            session.cache.global_model_by_name(name).is_some(),
            "Intro2 medieval model {name} is unavailable"
        );
    }

    // Medieval trees are composite models: mixed type-13 faces form the
    // view-pinned trunk/stalk and the Section-8 billboard commands place the
    // camera-facing crown afterward. Losing either half produces the floating
    // crowns seen in the incomplete renderer.
    let (_, tree) = session.cache.global_model_by_name("bigtree1").unwrap();
    assert_eq!(tree.billboards.len(), 3);
    assert!(!tree.vertex_type_flags.contains(&14));
    assert!(tree.triangles.iter().any(|triangle| {
        let pinned = triangle
            .iter()
            .filter(|&&index| tree.vertex_type_flags.get(index as usize) == Some(&13))
            .count();
        pinned > 0 && pinned < 3
    }));
    assert_eq!(
        tree.face_materials
            .iter()
            .copied()
            .map(face_material)
            .collect::<Vec<_>>(),
        vec![(1448, true), (1448, true), (1402, true), (1402, true)]
    );
    // The first mixed pinned quad is not an opaque trunk strip. Its ordinary
    // textured-face filler receives sprite 1448's 0x0D flags and therefore
    // uses the retail source-plus-destination-half family. The complete-tree fallback and the
    // three crown attachments remain masked (0x05).
    assert_eq!(
        session.cache.global_sprite(1448).unwrap().1.pal_size as u8,
        0x0d
    );
    for sprite_id in [1402, 1433] {
        assert_eq!(
            session.cache.global_sprite(sprite_id).unwrap().1.pal_size as u8,
            0x05,
            "tree sprite {sprite_id} render flags"
        );
    }

    let type_models = session.cache.global_entity_model_table();
    let entities = EntityManager::from_level(
        session.cache.level_desc().unwrap(),
        &type_models,
        session.cache.terrain(),
    );
    let actor_models: HashSet<String> = entities
        .iter()
        .flat_map(|entity| entity.model_slots.into_iter().flatten())
        .filter_map(|model_id| session.cache.global_model(model_id))
        .filter_map(|model| model.name.clone())
        .collect();
    for name in ["pesnthut", "dpsnthut", "man2"] {
        assert!(
            actor_models.contains(name),
            "Intro2 does not instantiate {name}; resolved models: {actor_models:?}"
        );
    }
    for name in ["man2", "lev1sci2"] {
        let (_, actor) = session.cache.global_model_by_name(name).unwrap();
        let footprint_faces = actor
            .triangles
            .iter()
            .filter(|triangle| {
                triangle
                    .iter()
                    .all(|&index| actor.vertex_type_flags[index as usize] == 13)
            })
            .count();
        assert_eq!(actor.triangles.len(), 4);
        assert_eq!(footprint_faces, 2);
        assert_eq!(actor.triangles.len() - footprint_faces, 2);
        assert!(actor.vertex_type_flags.contains(&13));
        assert!(actor
            .vertices
            .iter()
            .all(|vertex| (vertex[2] - actor.vertices[0][2]).abs() < f64::EPSILON));
        assert!(actor
            .face_materials
            .iter()
            .all(|material| *material & 0x8000 != 0));
    }

    let terrain = session.cache.terrain().unwrap();
    let static_models: HashSet<String> = terrain
        .cells
        .iter()
        .filter(|cell| cell.attribute != 0)
        .filter_map(|cell| {
            active_objects
                .records
                .get(usize::from(cell.attribute))
                .map(|descriptor| descriptor.model_id_for(cell.terrain_type))
        })
        .filter_map(|model_id| session.cache.global_model(usize::from(model_id)))
        .filter_map(|model| model.name.clone())
        .collect();
    assert!(
        static_models.contains("windmill"),
        "Intro2 terrain does not select windmill; resolved models: {static_models:?}"
    );
}

#[v2k_test_support::retail_test]
fn first_world_static_object_descriptors_resolve_every_authored_model() {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).unwrap();
    session.load_auxiliary_ovl(3, 0).unwrap();
    session.load_level_by_id(13, 0).unwrap();

    let terrain = session.cache.terrain().unwrap();
    let objects = session.cache.terrain_objects().unwrap();
    let mut selected_models = HashSet::new();
    let mut object_cells = 0;
    for cell in &terrain.cells {
        if cell.attribute == 0 {
            continue;
        }
        object_cells += 1;
        let descriptor = &objects.records[usize::from(cell.attribute)];
        let model_id = descriptor.model_id_for(cell.terrain_type);
        let model = session
            .cache
            .global_model(usize::from(model_id))
            .unwrap_or_else(|| {
                panic!(
                    "static attribute {} selected unresolved global model {}",
                    cell.attribute, model_id
                )
            });
        assert_ne!(model.collision_radius_raw, 0, "static model {model_id}");
        assert!(
            !model.collision_program.is_empty(),
            "static model {model_id} has no authored collision program"
        );
        selected_models.insert(model_id);
    }

    assert_eq!(object_cells, 242);
    assert_eq!(selected_models.len(), 32);
}

#[v2k_test_support::retail_test]
fn later_biome_model_ids_keep_unloaded_global_ranges() {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).unwrap();
    session.load_auxiliary_ovl(3, 0).unwrap();

    // Alpine is system level 8. Its canonical model range starts at 693,
    // after the unloaded Medieval (324..563) and Colorado (563..693) pools.
    let alpine = session.load_ovl_by_id(8, 0).unwrap();
    let alpine_models = &alpine.models.as_ref().unwrap().all_entries;
    assert_eq!(alpine_models.len(), 280);
    let (named_local_id, named_model) = alpine_models
        .iter()
        .enumerate()
        .find(|(_, model)| {
            model
                .name
                .as_deref()
                .is_some_and(|name| session.cache.global_model_by_name(name).is_none())
        })
        .expect("Alpine should contain a model name absent from the core pool");
    let named_model_name = named_model.name.clone().unwrap();

    session.load_auxiliary_ovl(8, 0).unwrap();

    assert!(session.cache.global_model(324).is_none());
    assert!(session.cache.global_model(692).is_none());
    assert_eq!(session.cache.global_model(693).unwrap().index, 0);
    assert_eq!(session.cache.global_model(972).unwrap().index, 279);
    assert_eq!(
        session
            .cache
            .global_model_by_name(&named_model_name)
            .map(|(id, _)| id),
        Some(693 + named_local_id)
    );
}

#[v2k_test_support::retail_test]
fn wrapper_props_instance_their_geometry() {
    let session = session_with_pools();
    // screenop (316) and sfxopt (318) are wrappers with no own faces; they
    // inline-instance screeno2 (0x13D = 317) / sfxopt2 (0x13F = 319) via
    // op 0x0E — the operand is the GLOBAL pool id.
    for (wrapper, target) in [(316usize, 317u16), (318, 319)] {
        let model = session.cache.global_model(wrapper).unwrap();
        assert!(
            model.triangles.is_empty(),
            "wrapper {wrapper} has own faces"
        );
        assert_eq!(
            model
                .instances
                .iter()
                .map(|i| i.model_id)
                .collect::<Vec<_>>(),
            vec![target],
            "wrapper {wrapper} instances"
        );
        let child = session.cache.global_model(target as usize).unwrap();
        assert!(
            !child.triangles.is_empty(),
            "instanced model {target} has no faces"
        );
    }
}

#[v2k_test_support::retail_test]
fn settings_wrappers_compose_both_fly_animation_rotations() {
    let session = session_with_high_menu_pools();
    for (global_id, name) in [(316, "screenop"), (318, "sfxopt")] {
        let wrapper = session.cache.global_model(global_id).unwrap();
        assert_eq!(wrapper.name.as_deref(), Some(name));
        // Both authored Y rotations consume callback[1]. FUN_004671D0
        // adds opcode 0x1C's angle to opcode 0x5C's existing mount, so the
        // child turns through twice the fly callback's angle.
        assert_eq!(&wrapper.cmd_words[..7], &[0x5c, 0, 1, 0x81, 0x1c, 1, 0x81]);
        for value in [0, 0x2000, 0x3800, 0x7000] {
            let mut vars = AnimVars::default();
            vars.dynamic[1] = value;
            let actual = wrapper.materialize(&vars).instances[0].orientation;
            let radians = (2 * value) as f64 / 65536.0 * std::f64::consts::TAU;
            let (sin, cos) = radians.sin_cos();
            let expected = [[cos, 0.0, sin], [0.0, 1.0, 0.0], [-sin, 0.0, cos]];
            for row in 0..3 {
                for column in 0..3 {
                    assert!(
                        (actual[row][column] - expected[row][column]).abs() < 1.0e-12,
                        "{name} callback {value:#06x} matrix [{row}][{column}]"
                    );
                }
            }
        }
    }
}

#[v2k_test_support::retail_test]
fn frontend_menu_ring_labels_resolve() {
    let session = session_with_pools();
    let config = GameConfig::default();
    let ctx = MenuCtx {
        cache: &session.cache,
        config: &config,
        saves: None,
        display_modes: &v2k_render::DisplayModes::default(),
    };
    let shell = MenuShell::new_frontend(&ctx, false);
    let view = shell.view();
    assert_eq!(view.layout, MenuLayout::Carousel);
    assert_eq!(view.items.len(), 7);

    let labels: Vec<&str> = view.items.iter().map(|i| i.label.as_str()).collect();
    for expected in [
        "New Game",
        "Load Game",
        "Display",
        "Sounds",
        "Controls",
        "Exit",
        "Network",
    ] {
        assert!(
            labels.contains(&expected),
            "ring missing '{expected}', got {labels:?}"
        );
    }
}

#[v2k_test_support::retail_test]
fn settings_screens_resolve_values() {
    let session = session_with_pools();
    let config = GameConfig::default();
    let ctx = MenuCtx {
        cache: &session.cache,
        config: &config,
        saves: None,
        display_modes: &v2k_render::DisplayModes::default(),
    };
    let mut shell = MenuShell::new_frontend(&ctx, false);

    // Open the Controls screen via the engine and check the view.
    assert!(shell.engine.push(v2k_game::menu_data::CONTROLS));
    shell.refresh(&ctx);
    let view = shell.view();
    assert_eq!(view.layout, MenuLayout::VerticalList);
    let labels: Vec<&str> = view.items.iter().map(|i| i.label.as_str()).collect();
    for expected in ["Absolute Mode", "Sensitivity", "Joystick", "Self Righting"] {
        assert!(
            labels.contains(&expected),
            "controls missing '{expected}', got {labels:?}"
        );
    }
    // The hidden Vibration item must not be in the view.
    assert!(!labels.iter().any(|l| l.contains("Vibration")));
    // Self Righting is the same FUN_0043B740 Off-or-bar control as the
    // volumes. The executable default is 1, hence a 14-cell bar with zero
    // filled cells (the callback passes value-1 to FUN_0043B690).
    let sr = view
        .items
        .iter()
        .find(|i| i.label == "Self Righting")
        .unwrap();
    assert_eq!(sr.bar, Some((0, 14)));
    // Joystick executable default 1 → "Relative" (id 54).
    let joy = view.items.iter().find(|i| i.label == "Joystick").unwrap();
    assert_eq!(joy.value_label.as_deref(), Some("Relative"));
}

#[v2k_test_support::retail_test]
fn filled_bar_glyph_decodes_visible_pixels() {
    let session = session_with_pools();
    let fonts = MenuFonts::from_cache(&session.cache).unwrap();
    assert_eq!(fonts.normal.max_ascent(), 9.0);
    assert_eq!(fonts.selected.max_ascent(), 9.0);
    let filled = fonts.selected.glyph(GLYPH_BAR_FILLED).unwrap();
    assert!(
        filled.rgba.chunks_exact(4).any(|p| p[3] != 0),
        "selected filled bar glyph decoded fully transparent"
    );
}

#[v2k_test_support::retail_test]
fn high_detail_menu_uses_variant_one_font_and_layout() {
    let session = session_with_pools();
    let high = session.load_ovl_by_id(2, 1).unwrap();
    let fonts = MenuFonts::from_level(&high).unwrap();
    assert_eq!((fonts.virtual_w, fonts.virtual_h), (640.0, 480.0));
    assert_eq!(fonts.layout_point(12), Some((320.0, 420.0)));
    assert_eq!(fonts.layout_point(6), Some((320.0, 360.0)));
    assert!(fonts.selected.glyph('N').is_some());
}

#[v2k_test_support::retail_test]
fn menu_fonts_retain_each_selected_retail_display_size() {
    let session = session_with_pools();
    for (variant, width, height) in [(0, 320, 240), (1, 640, 480), (2, 800, 600), (3, 1024, 768)] {
        let level = session.load_ovl_by_id(2, variant).unwrap();
        let fonts = MenuFonts::from_level(&level).unwrap();
        assert_eq!(
            (fonts.virtual_w, fonts.virtual_h),
            (width as f32, height as f32)
        );
        assert_eq!(fonts.layout_point(12).unwrap().0, width as f32 / 2.0);
        assert!(fonts.selected.glyph('N').is_some());
    }
}

#[v2k_test_support::retail_test]
fn high_detail_font_preserves_integer_descender_offsets() {
    let session = session_with_high_menu_pools();
    let fonts = MenuFonts::from_cache(&session.cache).unwrap();

    assert_eq!(fonts.selected.glyph('D').unwrap().yoff, 0.0);
    assert_eq!(fonts.selected.glyph('p').unwrap().yoff, 4.0);
    assert_eq!(fonts.selected.glyph('y').unwrap().yoff, 4.0);
}

#[v2k_test_support::retail_test]
fn sound_volume_bar_uses_original_row_width_hint() {
    let session = session_with_pools();
    let config = GameConfig::default();
    let ctx = MenuCtx {
        cache: &session.cache,
        config: &config,
        saves: None,
        display_modes: &v2k_render::DisplayModes::default(),
    };
    let mut shell = MenuShell::new_frontend(&ctx, false);
    shell.engine.settings.set(SettingId::SoundVolume, 15);
    assert!(shell.engine.push(SOUNDS));
    shell.refresh(&ctx);

    let sound = shell
        .view()
        .items
        .iter()
        .find(|i| i.label == "Sound")
        .unwrap();
    assert_eq!(sound.bar, Some((14, 14)));
}

#[v2k_test_support::retail_test]
fn network_shows_disabled_notice_and_only_back_is_interactive() {
    use v2k_game::menu_data::{MAIN_RING, NETWORK};
    use v2k_game::menu_engine::{MenuCommand, MenuInput};

    let session = session_with_high_menu_pools();
    let config = GameConfig::default();
    let ctx = MenuCtx {
        cache: &session.cache,
        config: &config,
        saves: None,
        display_modes: &v2k_render::DisplayModes::default(),
    };

    for back_input in [MenuInput::Select, MenuInput::Back] {
        let mut shell = MenuShell::new_frontend(&ctx, false);
        // Up from New Game reaches Network through the normal ring route.
        shell.engine.handle(MenuInput::Up);
        assert!(shell
            .engine
            .handle(MenuInput::Select)
            .contains(&MenuCommand::ScreenPushed(NETWORK)));
        shell.refresh(&ctx);

        let view = shell.view();
        assert_eq!(
            view.items
                .iter()
                .map(|row| (row.label.as_str(), row.selectable, row.enabled))
                .collect::<Vec<_>>(),
            [("Not available yet", false, false), ("Back", true, true)]
        );
        assert_eq!(view.selected, 1);
        assert_eq!(shell.list_window_rows(), 2);
        assert!(view.has_backdrop_panel);
        let groups = &shell.engine.current().unwrap().groups;
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].count, 2);
        assert_ne!(groups[0].flags & 0x08, 0);

        for direction in [
            MenuInput::Up,
            MenuInput::Down,
            MenuInput::Left,
            MenuInput::Right,
        ] {
            assert!(shell.engine.handle(direction).is_empty());
            assert_eq!(shell.engine.selected(), 1);
        }

        assert_eq!(
            shell.engine.handle(back_input),
            [
                MenuCommand::PlaySound(3),
                MenuCommand::ScreenPopped,
                MenuCommand::IntroSequenceCommand(1),
            ]
        );
        shell.refresh(&ctx);
        assert_eq!(shell.engine.current_va(), Some(MAIN_RING));
        assert_eq!(shell.view().items[shell.view().selected].label, "Network");
    }
}

#[v2k_test_support::retail_test]
fn display_exposes_retail_rows_and_port_scaling() {
    let session = session_with_pools();
    let config = GameConfig::default();
    let ctx = MenuCtx {
        cache: &session.cache,
        config: &config,
        saves: None,
        display_modes: &v2k_render::DisplayModes::default(),
    };
    let mut shell = MenuShell::new_frontend(&ctx, false);
    shell.engine.settings.set(SettingId::ActiveCamera, 0);
    assert!(shell.engine.push(DISPLAY));
    shell.refresh(&ctx);

    let rendering = shell
        .view()
        .items
        .iter()
        .find(|item| item.label == "Rendering")
        .unwrap();
    assert!(rendering.selectable, "retain the authored renderer row");
    assert!(rendering.enabled);
    assert!(rendering.is_interactive());
    assert_eq!(rendering.value_label.as_deref(), Some("OpenGL"));
    // Value 0 shows retail's own "Software" string.
    shell.engine.settings.set(SettingId::Rendering, 0);
    shell.refresh(&ctx);
    let rendering = shell
        .view()
        .items
        .iter()
        .find(|item| item.label == "Rendering")
        .unwrap();
    assert_eq!(rendering.value_label.as_deref(), Some("Software"));
    let camera = shell
        .view()
        .items
        .iter()
        .find(|i| i.label == "Active Camera")
        .unwrap();
    assert_eq!(camera.bar, Some((0, 10)));
    let scaling = shell
        .view()
        .items
        .iter()
        .find(|i| i.label == "Scaling")
        .unwrap();
    assert_eq!(scaling.value_label.as_deref(), Some("Native"));
    assert_eq!(
        shell
            .view()
            .items
            .iter()
            .filter(|item| item.is_interactive())
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>(),
        [
            "Rendering",
            "Resolution",
            "Bilinear Filtering",
            "Display",
            "Scaling",
            "Active Camera",
            "Targetter",
        ]
    );

    shell.engine.settings.set(SettingId::Scaling, 1);
    shell.refresh(&ctx);
    assert_eq!(
        shell
            .view()
            .items
            .iter()
            .find(|item| item.label == "Scaling")
            .and_then(|item| item.value_label.as_deref()),
        Some("4:3")
    );
    shell.engine.settings.set(SettingId::Scaling, 2);
    shell.refresh(&ctx);
    assert_eq!(
        shell
            .view()
            .items
            .iter()
            .find(|item| item.label == "Scaling")
            .and_then(|item| item.value_label.as_deref()),
        Some("Stretched")
    );
}

#[v2k_test_support::retail_test]
fn value_column_typewriter_prefix_stays_right_aligned() {
    let session = session_with_high_menu_pools();
    let fonts = MenuFonts::from_cache(&session.cache).unwrap();
    let font = fonts.font(false);
    let column_right = fonts
        .layout_point(7)
        .map(|(x, _)| 67.0 + x)
        .unwrap_or(252.0);

    for value in ["Off", "On", "Relative", "Native"] {
        let full_x = column_right - font.measure(value);
        let prefix_x = column_right - font.measure_prefix(value, 1);
        assert!(
            prefix_x > full_x,
            "{value}: visible prefix must sit further right than the completed string"
        );
        assert_eq!(
            column_right - font.measure_prefix(value, value.chars().count()),
            full_x,
            "{value}: completed prefix keeps the same right edge"
        );
    }

    let bar = v2k_game::menu_text::bar_string(0, 14);
    let full_bar_x = column_right - font.measure(&bar);
    let prefix_bar_x = column_right - font.measure_prefix(&bar, 3);
    assert!(
        prefix_bar_x > full_bar_x,
        "a partial bar prefix must sit further right than the completed bar"
    );
}

#[test]
fn runtime_setting_bridge_round_trips_authored_and_external_values() {
    use v2k_game::game_state::{apply_setting_to_config, sync_settings_from_config};

    let mut config = GameConfig::default();
    let mut engine = v2k_game::menu_engine::MenuEngine::main_menu();
    let modes = v2k_render::DisplayModes::default();
    for (setting, value) in [
        (SettingId::Bilinear, 0),
        (SettingId::Joystick, 0),
        (SettingId::AbsoluteMode, 1),
        (SettingId::SelfRighting, 12),
        (SettingId::ActiveCamera, 8),
        (SettingId::Targetter, 0),
        (SettingId::Scaling, 2),
        (SettingId::AmbientVolume, 7),
    ] {
        apply_setting_to_config(&mut config, setting, value, &modes);
    }
    sync_settings_from_config(&mut engine, &config, &modes);

    assert_eq!(engine.settings.get(SettingId::Bilinear), 0);
    assert_eq!(engine.settings.get(SettingId::Joystick), 0);
    assert_eq!(engine.settings.get(SettingId::AbsoluteMode), 1);
    assert_eq!(engine.settings.get(SettingId::SelfRighting), 12);
    assert_eq!(engine.settings.get(SettingId::ActiveCamera), 8);
    assert_eq!(engine.settings.get(SettingId::Targetter), 0);
    assert_eq!(engine.settings.get(SettingId::Scaling), 2);
    assert_eq!(engine.settings.get(SettingId::AmbientVolume), 7);
    assert_eq!(config.scaling, v2k_render::ScalingMode::Stretched);
}

#[v2k_test_support::retail_test]
fn authored_tree_records_preserve_intrinsic_aliases_and_front_face_planes() {
    let dir = v2k_test_support::retail_dir();
    // The canonical high-resolution tier owns these model assertions.
    let mut session = GameSession::init(&dir).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(13, 1).unwrap();

    // tf 12 is a pure positional alias in intrinsic materialization
    // (FUN_0046e9f0/FUN_0046ea60/FUN_0046eae0 copy X/Y/Z exactly), so the
    // trunk-base pair aliases record 0 at crown height. Grounding to the
    // terrain surface happens in the renderer's world tf-12 callback
    // family (FUN_004340b0/004343a0/004346b0 installed by FUN_00433FA0),
    // not in the parsed model data.
    let bigtree = session.cache.global_model(436).unwrap();
    assert_eq!(bigtree.records[0], [0, 350, 1050, 0]);
    assert_eq!(bigtree.records[1], [12, 0, 0, 0]);
    assert_eq!(bigtree.vertices[2][1], 1050.0);
    assert_eq!(bigtree.vertices[3][1], 1050.0);
    assert_eq!(bigtree.vertices[4][1], 1050.0);
    assert_eq!(bigtree.vertices[5][1], 1050.0);

    let tree3 = session.cache.global_model(444).unwrap();
    assert_eq!(tree3.records[0], [0, 360, 720, 0]);
    assert_eq!(tree3.records[1], [12, 0, 0, 0]);
    assert_eq!(tree3.vertices[2][1], 720.0);
    assert_eq!(tree3.vertices[3][1], 720.0);

    // Static trees keep their authored root orientation: bigtree3's
    // textured trunk faces carry a strict authored plane whose normal is
    // local -Z, so under identity orientation only the authored front
    // side passes FUN_0046D3F0's plane test.
    let identity = [[1.0f32, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let origin = [0.0f32, 0.0, 0.0];
    if let v2k_formats::models::ModelFaceCull::Plane(plane) = tree3.face_cull[2] {
        assert_eq!(plane.normal_raw, [0, 0, -32767]);
        assert!(v2k_render::authored_face_plane_visible(
            plane,
            identity,
            origin,
            100.0,
            [0.0, 50.0, -80.0],
        ));
        assert!(!v2k_render::authored_face_plane_visible(
            plane,
            identity,
            origin,
            100.0,
            [0.0, 50.0, 80.0],
        ));
    } else {
        panic!("bigtree3 trunk must retain its authored cull plane");
    }

    // Burned dbigtre3 (Level-1 descriptor 24 kind-0 burn target): record 1
    // aliases the crown anchor and record 2 sky-pins slot 0. Under world
    // submission the tf-12 pair grounds onto sampled terrain while the
    // plain pair keeps authored height 525, forming the upright
    // rooted-stump quad (sprite 1427) over the ground overlay quad
    // (sprite 1449). Every face is two-sided and spans the full sprite.
    let deadtree3 = session.cache.global_model(445).unwrap();
    assert_eq!(deadtree3.records[0], [0, 244, 525, 0]);
    assert_eq!(deadtree3.records[1], [12, 0, 0, 0]);
    assert_eq!(deadtree3.records[2], [13, 0, 0, 0]);
    assert_eq!(deadtree3.vertices[2][1], 525.0);
    assert_eq!(deadtree3.vertices[3][1], 525.0);
    assert_eq!(
        deadtree3.triangles,
        vec![[0, 1, 2], [0, 2, 3], [4, 5, 2], [4, 2, 3]]
    );
    assert_eq!(
        deadtree3.face_materials,
        vec![0x85a9, 0x85a9, 0x8593, 0x8593]
    );
    assert!(deadtree3
        .face_cull
        .iter()
        .all(|cull| matches!(cull, v2k_formats::models::ModelFaceCull::AlwaysVisible)));
    assert!(deadtree3.face_uvs.iter().all(|face| face
        .iter()
        .all(|uv| { (0.0..=1.0).contains(&uv[0]) && (0.0..=1.0).contains(&uv[1]) })));

    // The relaxed "camera-facing static tree" detector is gone: flat props
    // (fences/gates) and flat actors (man2) share its coarse shape but are
    // distinguished by their records — none carries the tree signature of
    // record 0 plain + record 1 aliasing slot 0 + a slot-0 type-13 record.
    for id in [492usize, 508, 558] {
        let other = session.cache.global_model(id).unwrap();
        let name = other.name.clone().unwrap_or_default();
        assert_ne!(other.records[1][0], 12, "{name}");
        assert!(
            !other.records.iter().any(|r| r[0] == 12 && r[1] == 0),
            "{name}"
        );
    }
}
