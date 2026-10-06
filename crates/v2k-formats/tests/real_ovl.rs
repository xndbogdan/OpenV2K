//! Integration tests against real V2000 OVL and PRELOAD.DAT files.
//!
//! Most single-tier fixtures intentionally use `0X3XX.OVL` because the parser
//! assertions target model, collision, sound, or format structure proven
//! invariant across display tiers. Variant 0 is the 320x240 low-resolution
//! visual source and must not be copied from these tests as a runtime default.

use std::path::PathBuf;

fn overlay_dir() -> PathBuf {
    v2k_test_support::retail_dir().join("Overlay")
}

fn preload_path() -> PathBuf {
    v2k_test_support::retail_dir().join("PRELOAD.DAT")
}

fn load_from(path: PathBuf) -> v2k_formats::ovl::OvlFile {
    let data = std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    v2k_formats::ovl::OvlFile::parse(&data)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn load_ovl(name: &str) -> v2k_formats::ovl::OvlFile {
    load_from(overlay_dir().join(name))
}

fn load_demo_ovl(name: &str) -> v2k_formats::ovl::OvlFile {
    load_from(v2k_test_support::demo_dir().join("OVERLAY").join(name))
}

fn flat_terrain(height: i8) -> v2k_formats::terrain::TerrainGrid {
    use v2k_formats::terrain::{TerrainCell, TerrainGrid, GRID_SIZE};

    TerrainGrid {
        header: [0; 5],
        cells: vec![
            TerrainCell {
                height: height as u8,
                attribute: 0,
                terrain_type: 0,
            };
            GRID_SIZE * GRID_SIZE
        ],
    }
}

#[v2k_test_support::retail_test]
fn section8_player4_model_pair_executes_first_world_static_programs() {
    let ovl = load_ovl("0X3XX.OVL");
    let models = v2k_formats::sections::parse_models(&ovl).unwrap();
    let pool = Level3ModelPool(&models);
    let vars = v2k_formats::models::AnimVars::default();
    let player = models
        .all_entries
        .iter()
        .find(|model| model.name.as_deref() == Some("player4"))
        .unwrap();
    for (name, expected_normal, expected_penetration) in [
        ("weight", [0.0, -1.0, 0.0], 105.0),
        ("college", [0.0, 1.0, 0.0], 400.0),
        ("lifter", [1.0, 0.0, 0.0], 490.0),
    ] {
        let target = models
            .all_entries
            .iter()
            .find(|model| model.name.as_deref() == Some(name))
            .unwrap();
        let hit = player
            .collide_model_raw_oriented(
                target,
                [0.0; 3],
                [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                &vars,
                &vars,
                &pool,
            )
            .unwrap_or_else(|error| panic!("{name} model pair failed: {error:?}"))
            .unwrap_or_else(|| panic!("{name} should overlap player4 at a shared origin"));
        assert_eq!(hit.normal, expected_normal, "{name} normal");
        assert_eq!(
            hit.penetration_raw, expected_penetration,
            "{name} penetration"
        );
    }
}

#[v2k_test_support::retail_test]
fn section8_player4_pitched_detail_contacts_terrain_before_scalar_clearance() {
    let ovl = load_ovl("1X3XX.OVL");
    let models = v2k_formats::sections::parse_models(&ovl).unwrap();
    let player = models
        .all_entries
        .iter()
        .find(|model| model.name.as_deref() == Some("player4"))
        .unwrap();
    let vars = v2k_formats::models::AnimVars::default();
    // The 20260719-123523 guided retail survey's dry-ground response at tick
    // 6184 has origin Y=-344, scalar bottom Y=-484, and interpolated terrain
    // Y=-580. A flat -576 fixture preserves its decisive 92-raw scalar gap.
    let terrain = flat_terrain(-18);
    let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let q31 = 2_147_483_648.0;
    let captured_basis = [
        [
            -2_147_090_432.0 / q31,
            -36_175_872.0 / q31,
            7_929_856.0 / q31,
        ],
        [0.0, -460_062_720.0 / q31, -2_097_414_144.0 / q31],
        [
            37_027_840.0 / q31,
            -2_097_086_464.0 / q31,
            459_931_648.0 / q31,
        ],
    ];
    let origin = [-25_086, -344, 32_287];

    // The legacy scalar check used half the model header/LOD radius. It still
    // reports 92 raw units of clearance here, and the unpitched detailed
    // spheres agree that there is no contact.
    assert_eq!(
        i32::from(origin[1]) - i32::from(player.radius / 2) - (-576),
        92
    );
    assert!(player
        .collide_terrain_raw_oriented(&terrain, origin, identity, &vars)
        .expect("player4 terrain program should be supported")
        .is_none());

    // Retail's captured pitch rotates authored detailed geometry into the
    // terrain despite that positive scalar clearance.
    let hit = player
        .collide_terrain_raw_oriented(&terrain, origin, captured_basis, &vars)
        .expect("player4 terrain program should be supported")
        .expect("pitched detailed player4 sphere must contact flat terrain");
    assert!(hit.normal[1] > 0.0);
    assert!(hit.penetration_raw >= 1.0);
}

#[v2k_test_support::retail_test]
fn section8_player4_nose_up_tail_contacts_terrain_before_scalar_clearance() {
    let ovl = load_ovl("1X3XX.OVL");
    let models = v2k_formats::sections::parse_models(&ovl).unwrap();
    let player = models
        .all_entries
        .iter()
        .find(|model| model.name.as_deref() == Some("player4"))
        .unwrap();
    let vars = v2k_formats::models::AnimVars::default();
    let terrain = flat_terrain(0);
    let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    // player4's nose points along local -Z. This quarter-turn raises the nose
    // and rotates the authored tail spheres down toward the terrain.
    let nose_up_90 = [[1.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0]];
    let origin = [128, 190, 128];

    assert_eq!(i32::from(origin[1]) - i32::from(player.radius / 2), 50);
    assert!(player
        .collide_terrain_raw_oriented(&terrain, origin, identity, &vars)
        .expect("player4 terrain program should be supported")
        .is_none());

    let hit = player
        .collide_terrain_raw_oriented(&terrain, origin, nose_up_90, &vars)
        .expect("player4 terrain program should be supported")
        .expect("nose-up player4 tail sphere must contact flat terrain");
    assert!(hit.normal[1] > 0.0);
    assert!(hit.penetration_raw >= 1.0);
}

struct Level3ModelPool<'a>(&'a v2k_formats::models::ModelCollection);

impl v2k_formats::models::CollisionModelPool for Level3ModelPool<'_> {
    fn collision_model(&self, global_id: usize) -> Option<&v2k_formats::models::ModelEntry> {
        // System level 2 contributes the first 13 entries to DAT_004FE640;
        // 0X3XX begins at global model id 13.
        self.0.all_entries.get(global_id.checked_sub(13)?)
    }
}

#[v2k_test_support::retail_test]
fn section8_first_world_base_and_factory_staged_effect_points_match_retail_walk() {
    use v2k_formats::models::{AnimVars, StagedEffectRequest};

    let ovl = load_ovl("1X3XX.OVL");
    let models = v2k_formats::sections::parse_models(&ovl).unwrap();
    let pool = Level3ModelPool(&models);

    for (name, expected_radii, expected_slots) in [
        (
            "college",
            vec![1_100, 200, 200, 200, 600, 200, 200],
            vec![62, 52, 54, 56, 58, 50, 60],
        ),
        (
            "lifter",
            vec![1_248, 390, 234, 468, 156, 400, 60, 56, 56],
            vec![90, 90, 86, 94, 36, 68, 68, 60, 64],
        ),
    ] {
        let model = models
            .all_entries
            .iter()
            .find(|model| model.name.as_deref() == Some(name))
            .unwrap_or_else(|| panic!("missing {name}"));
        let points = model
            .staged_effect_points_raw(StagedEffectRequest::default(), &AnimVars::default(), &pool)
            .unwrap_or_else(|error| panic!("{name} staged-effect walk failed: {error:?}"));

        assert_eq!(
            points
                .iter()
                .map(|point| point.scatter_radius_raw)
                .collect::<Vec<_>>(),
            expected_radii,
            "{name} radii"
        );
        assert_eq!(
            points
                .iter()
                .map(|point| point.source_slot)
                .collect::<Vec<_>>(),
            expected_slots,
            "{name} slots"
        );
    }
}

// ── Existing tests ──────────────────────────────────────────────────────────

#[v2k_test_support::retail_test]
fn parse_ovl_sections() {
    let path = overlay_dir().join("0X3XX.OVL");
    let data = std::fs::read(&path).unwrap();
    let ovl = v2k_formats::ovl::OvlFile::parse(&data).unwrap();
    assert_eq!(ovl.sections.len(), 15);

    // All sections should have valid indices
    for (i, sec) in ovl.sections.iter().enumerate() {
        assert_eq!(sec.index, i);
    }
}

#[v2k_test_support::retail_test]
fn parse_sprite_atlas() {
    // Exercise the parser against the authored low-tier atlas deliberately;
    // high-tier selection is a runtime/tool policy tested elsewhere.
    let path = overlay_dir().join("0X3XX.OVL");
    let data = std::fs::read(&path).unwrap();
    let ovl = v2k_formats::ovl::OvlFile::parse(&data).unwrap();

    let atlas = v2k_formats::sections::parse_sprites(&ovl).unwrap();
    assert_eq!(atlas.width, 2048);
    assert!(atlas.height > 0);
    assert!(!atlas.entries.is_empty());
    assert!(!atlas.rects.is_empty());

    // Decode all sprites at brightest shade
    let sprites = atlas.decode_all(15);
    assert!(!sprites.is_empty(), "should decode at least one sprite");

    for sprite in &sprites {
        assert!(sprite.width > 0);
        assert!(sprite.height > 0);
        assert_eq!(
            sprite.rgba.len(),
            sprite.width as usize * sprite.height as usize * 4
        );
    }
}

#[v2k_test_support::retail_test]
fn overlay_51_section_one_last_point_is_continue_baseline() {
    use v2k_formats::preload::PreloadDat;
    use v2k_formats::system::parse_fixup_table;

    let ovl = load_ovl("1X51XX.OVL");
    let table = parse_fixup_table(&ovl.sections[1].data);
    assert_eq!(table.entries.len(), 31);
    let last = table.entries[30];
    let y = (last >> 16) as i16;
    let x = last as i16;
    assert_eq!((x, y), (0, 95));

    let preload = PreloadDat::parse(&std::fs::read(preload_path()).unwrap()).unwrap();
    let section1 = &preload.grid[1];
    let start: u32 = section1.iter().take(51).sum();
    assert_eq!(start, 48);
    assert_eq!(section1[51], 31);
    assert_eq!(start + 30, 78);
}

#[v2k_test_support::retail_test]
fn level_51_sprite_table_stops_before_palette_data() {
    let ovl = load_ovl("0X51XX.OVL");
    let atlas = v2k_formats::sections::parse_sprites(&ovl).unwrap();

    assert_eq!(atlas.entries.len(), 33);
    assert_eq!(atlas.entries.first().unwrap().index, 3733);
    assert_eq!(atlas.entries.last().unwrap().index, 3765);
    assert_eq!(
        atlas
            .decode_all_strict(v2k_formats::palette::BRIGHTEST_SHADE)
            .unwrap()
            .len(),
        33
    );
}

#[v2k_test_support::retail_test]
fn extract_strings_from_ovl() {
    let path = overlay_dir().join("0X3XX.OVL");
    let data = std::fs::read(&path).unwrap();
    let ovl = v2k_formats::ovl::OvlFile::parse(&data).unwrap();

    let strings = v2k_formats::sections::extract_strings(&ovl);
    // Level 3 (system) should have strings
    assert!(!strings.is_empty(), "system OVL should contain strings");
}

#[v2k_test_support::retail_test]
fn parse_preload_dat() {
    let path = preload_path();
    let data = std::fs::read(&path).unwrap();
    let preload = v2k_formats::preload::PreloadDat::parse(&data).unwrap();

    // Matrix should have 15x53 entries
    assert_eq!(preload.grid.len(), 15);
    for row in &preload.grid {
        assert_eq!(row.len(), 53);
    }

    // Should have non-zero entries (204 per docs)
    assert!(preload.non_zero_count() > 100);

    // Should have 7 embedded OVLs
    assert_eq!(preload.embedded_ovls.len(), 7);
}

// ── Section 0: Pointer fixup (data) ─────────────────────────────────────────

#[v2k_test_support::retail_test]
fn section0_fixup_data() {
    let ovl = load_ovl("0X2XX.OVL");
    let table = v2k_formats::sections::parse_fixup_data(&ovl);
    let table = table.expect("0X2XX should have section 0 data");
    assert_eq!(table.entries.len(), 6, "section 0 should have 6 entries");
}

// ── Section 1: Pointer fixup (code) ─────────────────────────────────────────

#[v2k_test_support::retail_test]
fn section1_fixup_code() {
    let ovl = load_ovl("0X3XX.OVL");
    let table = v2k_formats::sections::parse_fixup_code(&ovl);
    let table = table.expect("0X3XX should have section 1 data");
    assert_eq!(table.entries.len(), 35, "section 1 should have 35 entries");
}

// ── Section 5: Display modes ────────────────────────────────────────────────

#[v2k_test_support::retail_test]
fn section5_display_modes() {
    let ovl = load_ovl("0X2XX.OVL");
    let modes = v2k_formats::sections::parse_display_modes(&ovl).unwrap();
    assert_eq!(modes.len(), 4, "should have 4 display modes");
    assert_eq!(modes[0].width, 320);
    assert_eq!(modes[0].height, 240);
    assert_eq!(modes[3].width, 1024);
    assert_eq!(modes[3].height, 768);
}

// ── Section 6: Fog gradient ─────────────────────────────────────────────────

#[v2k_test_support::retail_test]
fn section6_fog_gradient() {
    let ovl = load_ovl("0X2XX.OVL");
    let entries = v2k_formats::sections::parse_fog_gradient(&ovl).unwrap();
    assert_eq!(entries.len(), 26, "should have 26 fog gradient entries");
    assert_eq!(
        entries[..8]
            .iter()
            .map(|entry| entry.shade_level)
            .collect::<Vec<_>>(),
        vec![7, 10, 14, 17, 21, 24, 28, 31],
        "primary terrain shade indices must select the retail 32-row ramp"
    );
    // Shade levels should be in range 7-31
    for e in &entries {
        assert!(
            e.shade_level >= 7 && e.shade_level <= 31,
            "shade_level {} out of range 7-31",
            e.shade_level
        );
    }
}

// ── Section 7: Color palettes ───────────────────────────────────────────────

#[v2k_test_support::retail_test]
fn section7_color_palettes() {
    let ovl = load_ovl("0X2XX.OVL");
    let entries = v2k_formats::sections::parse_color_palettes(&ovl).unwrap();
    assert_eq!(entries.len(), 86, "level 2 should have 86 palette entries");
}

// ── Section 8: Model sub-blocks ─────────────────────────────────────────────

#[v2k_test_support::retail_test]
fn section8_models_parse() {
    let ovl = load_ovl("0X3XX.OVL");
    let models = v2k_formats::sections::parse_models(&ovl).unwrap();
    assert!(
        !models.sub_blocks.is_empty(),
        "should have model sub-blocks"
    );
    assert!(!models.all_entries.is_empty(), "should have model entries");
    // At least some entries should have vertices
    let with_verts = models
        .all_entries
        .iter()
        .filter(|e| !e.vertices.is_empty())
        .count();
    assert!(with_verts > 0, "some entries should have vertex data");
    eprintln!(
        "Section 8: {} sub-blocks, {} entries, {} vertices, {} triangles, {} edges",
        models.sub_blocks.len(),
        models.all_entries.len(),
        models.total_vertices(),
        models.total_triangles(),
        models.total_edges(),
    );
}

#[v2k_test_support::retail_test]
fn section8_models_names() {
    let ovl = load_ovl("0X3XX.OVL");
    let models = v2k_formats::sections::parse_models(&ovl).unwrap();
    let named = models.named_count();
    assert!(named > 0, "some model entries should have names");
    let names: Vec<&str> = models
        .all_entries
        .iter()
        .filter_map(|e| e.name.as_deref())
        .collect();
    eprintln!(
        "Named models ({named}): {:?}",
        &names[..names.len().min(20)]
    );
}

#[v2k_test_support::retail_test]
fn section8_player4_keeps_type13_footprint_in_canonical_body_mesh() {
    let ovl = load_ovl("0X3XX.OVL");
    let models = v2k_formats::sections::parse_models(&ovl).unwrap();
    let player = models
        .all_entries
        .iter()
        .find(|model| model.name.as_deref() == Some("player4"))
        .expect("player4 model");

    assert!(player.shadow_triangles.is_empty());
    assert_eq!(
        player
            .triangles
            .iter()
            .filter(|triangle| triangle
                .iter()
                .all(|&index| player.vertex_type_flags[index as usize] == 13))
            .count(),
        8,
        "player4 authors an eight-triangle type-13 footprint"
    );
    assert_eq!(player.normals.len(), player.triangles.len());
    assert_eq!(player.face_cull.len(), player.triangles.len());
    assert_eq!(player.face_materials.len(), player.triangles.len());
    assert_eq!(player.face_uvs.len(), player.triangles.len());
    assert_eq!(player.face_corner_normals.len(), player.triangles.len());
    assert_eq!(player.face_shading.len(), player.triangles.len());
}

#[v2k_test_support::retail_test]
fn section8_player4_tf8_uses_retail_half_morph() {
    use v2k_formats::models::AnimVars;

    let ovl = load_ovl("0X3XX.OVL");
    let models = v2k_formats::sections::parse_models(&ovl).unwrap();
    let player = models
        .all_entries
        .iter()
        .find(|model| model.name.as_deref() == Some("player4"))
        .expect("player4 model");

    let base = player
        .resolve_slot_with_context(
            88,
            v2k_formats::models::ModelMaterializationContext::intrinsic(&AnimVars::default(), None),
        )
        .expect("player4 tf-8 slot");
    assert_eq!(base.position_raw, [100.0, -20.0, -100.0]);

    let mut vars = AnimVars::default();
    vars.dynamic[5] = 0x8000;
    let half = player
        .resolve_slot_with_context(
            88,
            v2k_formats::models::ModelMaterializationContext::intrinsic(&vars, None),
        )
        .expect("player4 half-morph slot");
    assert_eq!(half.position_raw, [120.0, -34.0, -100.0]);
}

#[v2k_test_support::retail_test]
fn section8_first_world_collision_programs_execute_on_real_models() {
    use v2k_formats::models::AnimVars;

    let ovl = load_ovl("0X3XX.OVL");
    let models = v2k_formats::sections::parse_models(&ovl).unwrap();
    let pool = Level3ModelPool(&models);
    let vars = AnimVars::default();

    for name in ["player4", "weight", "college", "lifter"] {
        let model = models
            .all_entries
            .iter()
            .find(|model| model.name.as_deref() == Some(name))
            .unwrap_or_else(|| panic!("missing {name} model"));
        assert!(
            !model.collision_program.is_empty(),
            "{name} collision bytes"
        );
        assert!(
            model
                .collide_sphere_raw([0.0; 3], u16::MAX, &vars, &pool)
                .unwrap_or_else(|error| panic!("{name} collision program failed: {error:?}"))
                .is_some(),
            "{name} should resolve at least one authored primitive"
        );
    }
}

#[v2k_test_support::retail_test]
fn section8_klaus_live_hierarchy() {
    use v2k_formats::models::AnimVars;

    fn assert_matrix_close(actual: [[f64; 3]; 3], expected: [[f64; 3]; 3]) {
        for row in 0..3 {
            for column in 0..3 {
                assert!(
                    (actual[row][column] - expected[row][column]).abs() < 1.0e-12,
                    "matrix mismatch at [{row}][{column}]: {actual:?} != {expected:?}"
                );
            }
        }
    }

    let ovl = load_ovl("1X2XX.OVL");
    let models = v2k_formats::sections::parse_models(&ovl).unwrap();
    let klaus = &models.all_entries[1];
    assert_eq!(klaus.name.as_deref(), Some("klaus"));
    let root = klaus.materialize(&AnimVars::default());
    assert_eq!(root.instances.len(), 1);
    assert_eq!(root.instances[0].model_id, 2);
    // Klaus's op-0x5C turns Y by 180 degrees; its following op-0x1C
    // adds the authored -22.5-degree X rest tilt before instancing the body.
    let (sin, cos) = (-std::f64::consts::TAU / 16.0).sin_cos();
    assert_matrix_close(
        root.instances[0].orientation,
        [[-1.0, 0.0, 0.0], [0.0, cos, -sin], [0.0, -sin, -cos]],
    );

    let body = models.all_entries[2].materialize(&AnimVars::default());
    assert_eq!(body.instances.len(), 4);
    assert_eq!(body.instances[0].model_id, 3);
    assert_eq!(body.instances[1].model_id, 3);
    assert_matrix_close(
        body.instances[0].orientation,
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
    );
    // The second wing's op-0x5C has orientation code 0x20, which mirrors its
    // X basis. This is command-stream data, not attach-slot parity.
    assert_matrix_close(
        body.instances[1].orientation,
        [[-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
    );

    // FUN_0042C660 seeds Klaus's menu rest pose. In particular callback #3
    // is -0x1000 (-22.5 degrees), folding both wing roots instead of leaving
    // the nested chains as straight horizontal bars.
    let mut menu_vars = AnimVars::default();
    menu_vars.dynamic[3] = 0xF000;
    menu_vars.dynamic[4] = 0xF800;
    menu_vars.dynamic[5] = 0xE800;
    menu_vars.dynamic[7] = 0x0800;
    menu_vars.dynamic[10] = 0xF800;
    menu_vars.dynamic[11] = 0xF000;
    let folded_body = models.all_entries[2].materialize(&menu_vars);
    let angle = -std::f64::consts::TAU / 16.0;
    let (sin, cos) = angle.sin_cos();
    assert_matrix_close(
        folded_body.instances[0].orientation,
        [[cos, -sin, 0.0], [sin, cos, 0.0], [0.0, 0.0, 1.0]],
    );
}

#[v2k_test_support::retail_test]
fn section8_klaus_jaw_combines_fly_through_progress_with_the_spin_angle() {
    use v2k_formats::models::AnimVars;

    let ovl = load_ovl("1X2XX.OVL");
    let models = v2k_formats::sections::parse_models(&ovl).unwrap();
    let head = &models.all_entries[8];
    assert_eq!(head.name.as_deref(), Some("pteranasetheadklaus"));
    assert_eq!(&head.cmd_words[63..70], &[0x5C, 0, 0, 0x89, 0x1C, 0, 0xC7]);
    for (progress, opening) in [
        (0, 0),
        (0x7FFF, 0),
        (0x8000, 0),
        (0xC000, 0x2000),
        (0xFFFF, 0x3FFF),
    ] {
        for spin_angle in [-0x1000, 0, 0x1000] {
            let mut vars = AnimVars::default();
            vars.dynamic[1] = progress;
            vars.dynamic[9] = spin_angle;
            let materialized = head.materialize(&vars);
            let jaw = materialized
                .instances
                .iter()
                .find(|instance| instance.model_id == 9)
                .expect("authored jaw child");
            assert_eq!(jaw.attach_slot, 28);
            assert_eq!(jaw.attach_pos, Some([0.0, -150.0, 240.0]));
            // The actual stream computes r7=max(progress>>1,0x4000)-0x4000,
            // then applies that X rotation after the callback-9 spin angle.
            assert_eq!(jaw.registers[7], opening);
            let angle =
                f64::from(spin_angle + i32::from(opening)) * std::f64::consts::TAU / 65536.0;
            let (sin, cos) = angle.sin_cos();
            let expected = [[1.0, 0.0, 0.0], [0.0, cos, -sin], [0.0, sin, cos]];
            for row in 0..3 {
                for column in 0..3 {
                    assert!(
                        (jaw.orientation[row][column] - expected[row][column]).abs() < 1.0e-12,
                        "progress={progress:#X}, spin={spin_angle:#X}: {:?} != {expected:?}",
                        jaw.orientation
                    );
                }
            }
        }
    }
}

#[v2k_test_support::retail_test]
fn section8_klaus_preserves_sprite_textures_and_gouraud_data() {
    use std::collections::BTreeSet;
    use v2k_formats::models::face_material;

    let ovl = load_ovl("0X2XX.OVL");
    let models = v2k_formats::sections::parse_models(&ovl).unwrap();
    let wing = models
        .all_entries
        .iter()
        .find(|m| m.name.as_deref() == Some("insectwing1aklaus"))
        .expect("Klaus wing root model");

    assert_eq!(wing.triangles.len(), 8, "four textured Gouraud quads");
    assert_eq!(wing.face_uvs.len(), wing.triangles.len());
    assert_eq!(wing.face_corner_normals.len(), wing.triangles.len());
    let sprite_ids: BTreeSet<u16> = wing
        .face_materials
        .iter()
        .map(|&packed| {
            let (id, is_sprite) = face_material(packed);
            assert!(is_sprite, "Klaus wing faces must use Section-3 sprites");
            id
        })
        .collect();
    assert_eq!(sprite_ids, BTreeSet::from([408, 409]));

    // Each source quad remains one continuous sprite rectangle after it is
    // split into two GL triangles.
    assert_eq!(wing.face_uvs[0], [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]);
    assert_eq!(wing.face_uvs[1], [[0.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
    assert!(
        wing.face_corner_normals
            .iter()
            .any(|n| n[0] != n[1] || n[1] != n[2]),
        "0xA4 Gouraud corner normals must not be flattened"
    );

    // Sprite 409 is the literal wing-root artwork: both transparent cutout
    // texels and opaque bone/membrane pixels must survive decoding.
    let atlas = v2k_formats::sections::parse_sprites(&ovl).unwrap();
    let entry = atlas
        .entries
        .iter()
        .find(|e| e.index == 409)
        .expect("global Klaus sprite 409");
    let decoded = atlas
        .decode_sprite(entry, v2k_formats::palette::BRIGHTEST_SHADE)
        .unwrap();
    assert_eq!((decoded.width, decoded.height), (32, 32));
    let transparent = decoded.rgba.chunks_exact(4).filter(|p| p[3] == 0).count();
    let opaque = decoded.rgba.chunks_exact(4).filter(|p| p[3] != 0).count();
    assert!(transparent > 0 && opaque > 0);
}

#[v2k_test_support::retail_test]
fn section8_peasant_hut_roof_uses_retail_triangle_texture_domain() {
    use v2k_formats::models::face_material;

    let ovl = load_ovl("1X6XX.OVL");
    let models = v2k_formats::sections::parse_models(&ovl).unwrap();
    let hut = models
        .all_entries
        .iter()
        .find(|model| model.name.as_deref() == Some("pesnthut"))
        .expect("Level-1 peasant hut");

    let roof_uvs: Vec<_> = hut
        .face_materials
        .iter()
        .zip(&hut.face_uvs)
        .filter_map(|(&packed, uvs)| (face_material(packed) == (1365, true)).then_some(*uvs))
        .collect();
    assert_eq!(roof_uvs.len(), 12, "twelve authored thatch triangles");
    for uvs in roof_uvs {
        assert!(
            uvs == [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]
                || uvs == [[1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
            "the roof must sample FUN_0047C600's upper/right sprite triangle: {uvs:?}"
        );
    }

    let atlas = v2k_formats::sections::parse_sprites(&ovl).unwrap();
    let thatch = atlas
        .entries
        .iter()
        .find(|entry| entry.index == 1365)
        .expect("global roof sprite 1365");
    assert_eq!(thatch.pal_size, 0x04);
    assert!(
        !thatch.is_zero_keyed(),
        "the unused half is authored palette colour, not transparency"
    );
    assert_eq!(
        atlas.palette_row(thatch, 28).unwrap()[0],
        [248, 80, 248, 255],
        "flag 0x04 selects the authored magenta padding in fixed row 28"
    );
    let indexed = atlas.decode_indices(thatch).unwrap();
    assert!(
        indexed.indices.contains(&0) && indexed.indices.iter().any(|&index| index != 0),
        "the triangular artwork must retain both its fill and thatch texels"
    );
}

#[v2k_test_support::retail_test]
fn section8_batch_all_ovls() {
    let dir = overlay_dir();

    let mut with_models = 0u32;
    let mut total_entries = 0usize;
    let mut total_named = 0usize;
    let mut body_faces = 0usize;
    let mut missing_cull_planes = 0usize;

    for entry in std::fs::read_dir(&dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "OVL") {
            continue;
        }
        let data = std::fs::read(&path).unwrap();
        let ovl = v2k_formats::ovl::OvlFile::parse(&data).unwrap();

        if ovl.has_data(8) {
            let models = v2k_formats::sections::parse_models(&ovl)
                .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            let mut path_missing_cull_planes = 0usize;
            with_models += 1;
            total_entries += models.all_entries.len();
            total_named += models.named_count();
            for model in &models.all_entries {
                assert_eq!(
                    model.face_cull.len(),
                    model.triangles.len(),
                    "{} has a non-parallel authored face-plane stream",
                    path.display()
                );
                body_faces += model.triangles.len();
                let model_missing = model
                    .face_cull
                    .iter()
                    .filter(|mode| matches!(mode, v2k_formats::models::ModelFaceCull::Unresolved))
                    .count();
                missing_cull_planes += model_missing;
                path_missing_cull_planes += model_missing;
            }
            if path_missing_cull_planes != 0 {
                eprintln!(
                    "{}: {path_missing_cull_planes} unresolved face planes",
                    path.display()
                );
            }
        }
    }

    eprintln!(
        "Section 8 batch: {with_models} OVLs with models, {total_entries} entries, {total_named} named, {body_faces} body faces"
    );
    assert!(
        with_models > 0,
        "some OVLs should have Section 8 model data"
    );
    assert_eq!(
        missing_cull_planes, 0,
        "shipped body faces should all resolve their authored cull planes"
    );
}

// ── Section 9: Animation frames ─────────────────────────────────────────────

#[v2k_test_support::retail_test]
fn section9_terrain_objects() {
    let ovl = load_ovl("0X6XX.OVL");
    let table = v2k_formats::sections::parse_terrain_objects(&ovl).unwrap();
    assert_eq!(table.records.len(), 256, "should have 256 records");

    // Retail FUN_0042F650/FUN_00427410 use these as absolute Section-8
    // model ids plus one dword object-kind index, not sprite frames/count.
    assert_eq!(table.records[24].model_ids, [444, 445, 446, 447]);
    assert_eq!(table.records[24].kind_index, 0);
    assert_eq!(table.records[216].model_ids, [493, 501, 493, 501]);
    assert_eq!(table.records[216].kind_index, 9);

    // Shared tail descriptors use model 39 in every state slot.
    for i in 251..=255 {
        let r = &table.records[i];
        assert_eq!(
            r.model_ids,
            [39, 39, 39, 39],
            "reserved record {} should have quad=[39,39,39,39]",
            i
        );
    }
}

// ── Section 10: Terrain heightmap ───────────────────────────────────────────

#[v2k_test_support::retail_test]
fn section10_terrain() {
    let ovl = load_ovl("0X14XX.OVL");
    let grid = v2k_formats::sections::parse_terrain(&ovl).unwrap();
    let (min_h, max_h) = grid.height_range();
    assert!(max_h > min_h, "terrain should have height variation");
    // Grid should be addressable
    assert!(grid.cell(0, 0).is_some());
    assert!(grid.cell(255, 255).is_some());
    assert!(grid.cell(256, 0).is_none());
    // Medaeval's player spawn is X=67, Z=-125 (unsigned terrain Z=131).
    // The raw executable address is 67*256+131; the transposed address has a
    // different height and guards this real-data path against row-major drift.
    assert_eq!(grid.cell(67, 131).unwrap().height, 225);
    assert_eq!(grid.cell(131, 67).unwrap().height, 212);
}

#[v2k_test_support::retail_test]
fn section10_peasant_x_major_world_axes_match_retail_anchors() {
    let ovl = load_ovl("0X13XX.OVL");
    let grid = v2k_formats::sections::parse_terrain(&ovl).unwrap();

    // Weight, player, base and lifter coordinates from retail captures. The
    // X-major samples reproduce their island surface exactly; swapping X/Z
    // lands on deep seabed and is retained here as an explicit regression
    // guard against confusing the old camera-side bug with terrain storage.
    let anchors = [
        ((75, 60), -26_i8, -65_i8),
        ((77, 58), -24_i8, -63_i8),
        ((80, 60), -24_i8, -62_i8),
        ((87, 58), -24_i8, -59_i8),
    ];
    for &((x, z), height, transposed_height) in &anchors {
        assert_eq!(grid.cell(x, z).unwrap().height as i8, height);
        assert_eq!(grid.cell(z, x).unwrap().height as i8, transposed_height);
    }
}

#[v2k_test_support::retail_test]
fn section10_peasant_weight_spawn_retains_authored_ground_shading() {
    let ovl = load_ovl("0X13XX.OVL");
    let grid = v2k_formats::sections::parse_terrain(&ovl).unwrap();

    // The cargo weight is authored at (75, 60). Its exact vertex is the
    // darkest Section-10 shade while the four adjacent vertices rise through
    // rows 3/4, producing a small interpolated ground mark which remains after
    // the separately rendered weight model is attached to the player.
    let center = grid.cell(75, 60).unwrap();
    assert_eq!(center.terrain_type & 7, 4);
    assert_eq!(center.terrain_type >> 5, 0);
    assert_eq!(
        [
            grid.cell(74, 60).unwrap().terrain_type >> 5,
            grid.cell(76, 60).unwrap().terrain_type >> 5,
            grid.cell(75, 59).unwrap().terrain_type >> 5,
            grid.cell(75, 61).unwrap().terrain_type >> 5,
        ],
        [3, 4, 3, 4]
    );
}

/// End-to-end water plane: load real level OVLs and assert the Section 10
/// sea level and the water-render gate against ground-truth values read from
/// V2000.EXE / the OVL bytes (sea level = header[0] >> 8 world units, port
/// plane = header[0] / 65536).
#[v2k_test_support::retail_test]
fn section10_water_levels() {
    // (file, world sea level (>>8), port sea plane Y, water rendered?)
    let cases: &[(&str, i32, f32, bool)] = &[
        ("0X22XX.OVL", 4096, 16.0, true), // Water — fully submerged
        ("0X23XX.OVL", 4096, 16.0, true), // Reef — fully submerged
        ("0X21XX.OVL", -4043, -4043.0 / 256.0, true), // Flood — starts just above seabed
        ("0X13XX.OVL", -847, -847.0 / 256.0, true), // Peasant — island in ocean
        ("0X36XX.OVL", 1011, 1011.0 / 256.0, true), // Atoll — island in deep sea
        ("0X26XX.OVL", -6144, -6144.0 / 256.0, false), // VSpread — dry (plane below floor)
        ("0X46XX.OVL", -4096, -4096.0 / 256.0, false), // Alien2 — sea==floor, strict '>' → dry
    ];

    for &(name, sea_world, plane_y, enabled) in cases {
        let ovl = load_ovl(name);
        let grid = v2k_formats::sections::parse_terrain(&ovl).unwrap();
        assert_eq!(
            grid.header[0] >> 8,
            sea_world,
            "{name}: sea level (world units)"
        );
        assert!(
            (grid.sea_level_world_y() - plane_y).abs() < 0.001,
            "{name}: port sea plane Y = {} expected {plane_y}",
            grid.sea_level_world_y()
        );
        assert_eq!(grid.water_enabled(), enabled, "{name}: water_enabled gate");
    }
}

// ── Section 11: Global sound pool ──────────────────────────────────────────

#[v2k_test_support::retail_test]
fn section11_global_sound_pool() {
    let level3_ovl = load_ovl("0X3XX.OVL");
    let preload_path = preload_path();
    let preload =
        v2k_formats::preload::PreloadDat::parse(&std::fs::read(&preload_path).unwrap()).unwrap();
    let level2_ovl = &preload
        .embedded_ovls
        .iter()
        .find(|embedded| embedded.index == 2)
        .expect("PRELOAD embedded OVL 2")
        .ovl;
    let level2 = v2k_formats::sections::parse_anim_sound(level2_ovl).unwrap();
    let level3 = v2k_formats::sections::parse_anim_sound(&level3_ovl).unwrap();
    let pool = v2k_formats::anim_sound::SoundPool::from_tables(&[&level2, &level3]);

    assert_eq!(level2.entries.len(), 7);
    assert_eq!(level3.entries.len(), 103);
    assert_eq!(pool.len(), 110);
    assert_eq!(level2.type1_count() + level3.type1_count(), 52);
    assert_eq!(level2.type5_count() + level3.type5_count(), 58);
    for global_id in 0..pool.len() {
        pool.resolve(global_id)
            .unwrap_or_else(|error| panic!("global sound {global_id}: {error}"));
    }
}

// ── Section 12: Collision models ────────────────────────────────────────────

#[v2k_test_support::retail_test]
fn section12_collision() {
    let ovl = load_ovl("0X3XX.OVL");
    let model = v2k_formats::sections::parse_collision(&ovl).unwrap();
    assert!(!model.entries.is_empty(), "should have collision entries");
    // At least some entries should have faces
    let with_faces = model.entries.iter().filter(|e| !e.faces.is_empty()).count();
    assert!(with_faces > 0, "some entries should have face data");
    assert_eq!(
        model.entries[4].model_ids,
        [286, 225, 286, 225],
        "L3 record 4 is global entity type 6 (Main Base)"
    );
    assert_eq!(model.entries[4].raw_header.len(), 0x128);
    assert_eq!(
        model.entries[4].behavior_choices,
        [v2k_formats::collision::BehaviorChoice {
            weight_rule_id: 1,
            weight_multiplier: 1,
            behavior_class_id: 41,
        }],
        "Main Base selects its named behavior through the common choice list"
    );
    assert_eq!(model.entries[4].initializer_state_flags_raw, 0x25027);
    assert_eq!(
        model.entries[4]
            .subsections
            .iter()
            .map(|sub| sub.name)
            .collect::<Vec<_>>(),
        model.entries[4]
            .active_subs
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        "all authored Section 12 payloads must survive parsing"
    );
    for sub in &model.entries[4].subsections {
        assert!(
            !sub.data.is_empty(),
            "subsection {} must retain bytes",
            sub.name
        );
        assert!((0x34..=0x43).contains(&sub.field_index));
    }
    for entry in &model.entries {
        assert!(
            entry.behavior_choices.len() <= 8,
            "retail selector has only eight local weight slots (entry {})",
            entry.index
        );
    }
    let mut authored_rules = std::collections::BTreeSet::new();
    for name in [
        "0X2XX.OVL",
        "0X3XX.OVL",
        "1X2XX.OVL",
        "1X3XX.OVL",
        "2X2XX.OVL",
        "2X3XX.OVL",
        "3X2XX.OVL",
        "3X3XX.OVL",
    ] {
        let ovl = load_ovl(name);
        let entries = v2k_formats::sections::parse_collision(&ovl).unwrap();
        for entry in entries.entries {
            assert!(entry.behavior_choices.len() <= 8);
            authored_rules.extend(
                entry
                    .behavior_choices
                    .into_iter()
                    .map(|choice| choice.weight_rule_id),
            );
        }
    }
    assert_eq!(
        authored_rules,
        [1, 2, 5, 6, 7, 8, 9, 10, 11, 12, 13].into_iter().collect()
    );
}

#[v2k_test_support::retail_test]
fn section12_type_8_retains_the_exact_scientist_job_policy() {
    use v2k_formats::collision::{BehaviorChoice, CommonAxisDescriptor};

    let ovl = load_ovl("1X3XX.OVL");
    let collision = v2k_formats::sections::parse_collision(&ovl).unwrap();

    // PRELOAD system level 2 contributes global types 0 and 1. First-world
    // local record 6 is therefore cumulative type 8 (`scientist`). Variant 1
    // is the normal high-resolution runtime tier; this control record is not a
    // reason to select the low-resolution 0X3XX presentation.
    let scientist = &collision.entries[6];
    assert_eq!(scientist.type_tag, 0xCC);
    assert_eq!(scientist.scale, 10);
    assert_eq!(scientist.id_field, 0x1404);
    assert_eq!(scientist.model_ids, [0x022F; 4]);
    assert_eq!(
        scientist.behavior_choices,
        [
            BehaviorChoice {
                weight_rule_id: 13,
                weight_multiplier: 100,
                behavior_class_id: 54,
            },
            BehaviorChoice {
                weight_rule_id: 1,
                weight_multiplier: 1,
                behavior_class_id: 6,
            },
        ]
    );
    assert_eq!(
        scientist.common_axis_descriptor(),
        CommonAxisDescriptor {
            strict_axis_limit_raw: 0x0F00,
            raw_word_at_0x04: 0x84,
        }
    );
    assert_eq!(scientist.alternate_behavior_class_ref, 14);
}

#[v2k_test_support::retail_test]
fn section12_type_47_retains_the_exact_projectile_emitter_descriptor() {
    use v2k_formats::collision::ProjectileEmitterDescriptor;

    let ovl = load_ovl("1X3XX.OVL");
    let collision = v2k_formats::sections::parse_collision(&ovl).unwrap();

    // PRELOAD system level 2 contributes global types 0 and 1. Therefore
    // first-world local records 15 and 45 are cumulative types 17 and 47.
    // This control data is invariant across presentation tiers; variant 1 is
    // used here to remain aligned with the normal high-resolution runtime.
    assert_eq!(collision.entries[15].projectile_emitter_descriptor(), None);
    assert_eq!(
        collision.entries[45].projectile_emitter_descriptor(),
        Some(ProjectileEmitterDescriptor {
            projectile_method: 30,
            random_interval_us: 750_000,
            spread_raw: 100,
            aim_threshold_raw: 16_000,
            speed_override_raw: 0,
            target_axis_tolerance_raw: 0x0600,
            sound_id: 70,
            raw_word_at_0x12: 150,
            alternate_emitter_raw: 0,
            stochastic_gate_mode: 0,
            auxiliary_command: 0,
            variable_bindings: [0; 4],
        })
    );
    assert_eq!(
        collision.entries[45].subsection("E"),
        Some(
            &[
                0x1e, 0x00, 0x00, 0x00, 0xb0, 0x71, 0x0b, 0x00, 0x64, 0x00, 0x80, 0x3e, 0x00, 0x00,
                0x00, 0x06, 0x46, 0x00, 0x96, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ][..]
        )
    );
}

#[v2k_test_support::retail_test]
fn section12_type_9_retains_the_exact_actor_animation_descriptor() {
    use v2k_formats::collision::ActorAnimationDescriptor;

    let ovl = load_ovl("1X3XX.OVL");
    let collision = v2k_formats::sections::parse_collision(&ovl).unwrap();

    // PRELOAD system level 2 contributes global types 0 and 1, so first-world
    // local record 7 is cumulative type 9 (`man2`). Variant 1 is the normal
    // high-resolution runtime tier even though this control payload is
    // invariant across the presentation variants.
    let man2 = &collision.entries[7];
    assert_eq!(
        man2.subsection("I"),
        Some(&[0x48, 0x00, 0x00, 0x00, 0x48, 0x00, 0x01, 0x04][..])
    );
    assert_eq!(
        man2.actor_animation_descriptor(),
        Some(ActorAnimationDescriptor {
            capability_bit_3_sound_id: 72,
            capability_mask_0x201_sound_id: 0,
            attention_stop_sound_id: 72,
            variable_binding: 1,
            frames_per_direction: 4,
        })
    );
}

#[v2k_test_support::retail_test]
fn type9_abort_profile_and_fresh_level1_spawns_are_exact_in_retail_tiers() {
    type9_abort_profile_and_fresh_level1_spawns_are_exact(false);
}

#[v2k_test_support::demo_test]
fn type9_abort_profile_and_fresh_level1_spawns_are_exact_in_demo_tiers() {
    type9_abort_profile_and_fresh_level1_spawns_are_exact(true);
}

fn type9_abort_profile_and_fresh_level1_spawns_are_exact(demo: bool) {
    use v2k_formats::collision::{
        ActorAnimationDescriptor, BehaviorChoice, CommonAxisDescriptor, SubAPropulsionDescriptor,
        SubBLateralDescriptor, SubDSteeringDescriptor,
    };

    const EXPECTED_SPAWNS: [(usize, [i16; 3]); 6] = [
        (9, [-28_672, 0, 32_256]),
        (10, [-29_184, 0, 32_256]),
        (14, [-29_440, 0, 32_512]),
        (15, [29_696, 0, 31_488]),
        (16, [-27_136, 0, -27_136]),
        (22, [-27_648, 0, -27_648]),
    ];

    let tiers = if demo { 0..=1 } else { 0..=3 };
    for tier in tiers {
        let load: fn(&str) -> v2k_formats::ovl::OvlFile =
            if demo { load_demo_ovl } else { load_ovl };
        let system = load(&format!("{tier}X3XX.OVL"));
        let collision = v2k_formats::sections::parse_collision(&system).unwrap();
        // PRELOAD system level 2 contributes global types 0 and 1, making
        // first-world local record 7 cumulative type 9 (`man2`).
        let type9 = &collision.entries[7];
        let build = if demo { "demo" } else { "retail" };
        let label = format!("{build} tier {tier}");
        assert_eq!(type9.index, 7, "{label}: local record");
        assert_eq!(type9.type_tag, 204, "{label}: type tag");
        assert_eq!(type9.scale, 10, "{label}: mass");
        assert_eq!(type9.id_field, 0x1804, "{label}: capabilities");
        assert_eq!(type9.model_ids, [558; 4], "{label}: models");
        assert_eq!(
            type9.dims,
            [1_500, 0, 2_000, 400, 0, 200],
            "{label}: dimensions and health"
        );
        assert_eq!(
            type9.initializer_state_flags_raw, 0x2F,
            "{label}: initializer state"
        );
        assert_eq!(
            type9.common_axis_descriptor(),
            CommonAxisDescriptor {
                strict_axis_limit_raw: 0x0F00,
                raw_word_at_0x04: 0x84,
            },
            "{label}: common-axis descriptor"
        );
        assert_eq!(
            type9.behavior_choices,
            [
                BehaviorChoice {
                    weight_rule_id: 7,
                    weight_multiplier: 10,
                    behavior_class_id: 10,
                },
                BehaviorChoice {
                    weight_rule_id: 6,
                    weight_multiplier: 3,
                    behavior_class_id: 45,
                },
                BehaviorChoice {
                    weight_rule_id: 12,
                    weight_multiplier: 200,
                    behavior_class_id: 54,
                },
                BehaviorChoice {
                    weight_rule_id: 1,
                    weight_multiplier: 1,
                    behavior_class_id: 6,
                },
            ],
            "{label}: ordinary behavior choices"
        );
        assert_eq!(type9.behavior_rule_ref, 1, "{label}: behavior rule");
        assert_eq!(
            type9.alternate_behavior_class_ref, 14,
            "{label}: death behavior"
        );
        assert_eq!(
            type9.active_subs,
            ["A", "B", "D", "I"],
            "{label}: component topology"
        );
        assert_eq!(
            type9.sub_a_propulsion_descriptor(),
            Some(SubAPropulsionDescriptor {
                acceleration_raw: 1_500,
                overspeed_correction_raw: -3_000,
                target_speed_base_raw: 250,
            }),
            "{label}: Sub-A"
        );
        assert_eq!(
            type9.sub_b_lateral_descriptor(),
            Some(SubBLateralDescriptor {
                projection_threshold_rate_raw: 10_000,
                correction_rate_raw: 1_000,
            }),
            "{label}: Sub-B"
        );
        assert_eq!(
            type9.sub_d_steering_descriptor(),
            Some(SubDSteeringDescriptor {
                steering_divisor_raw: 20,
                couple_yaw_into_roll_raw: 0,
                enable_pitch_steering_raw: 0,
                forward_probe_raw: 128,
                lateral_probe_raw: 64,
                classifier_flags: 0x17,
                reserved_at_0x0b: 0,
            }),
            "{label}: Sub-D"
        );
        let actor_cue = if demo { 74 } else { 72 };
        assert_eq!(
            type9.actor_animation_descriptor(),
            Some(ActorAnimationDescriptor {
                capability_bit_3_sound_id: actor_cue,
                capability_mask_0x201_sound_id: 0,
                attention_stop_sound_id: actor_cue,
                variable_binding: 1,
                frames_per_direction: 4,
            }),
            "{label}: Sub-I"
        );
        assert_eq!(
            type9.accepted_hit_presentation_sound_id(),
            Some(if demo { 97 } else { 95 }),
            "{label}: accepted-hit cue"
        );
        assert_eq!(type9.death_sound_id(), Some(35), "{label}: death cue");
        assert_eq!(
            type9.constructor_sound_attachment_id(),
            None,
            "{label}: constructor attachment"
        );
        assert_eq!(
            type9.generic_hit_sound_id(),
            None,
            "{label}: generic-hit cue"
        );

        let level_ovl = load(&format!("{tier}X13XX.OVL"));
        let level = v2k_formats::sections::parse_level(&level_ovl).unwrap();
        let spawns = level
            .entities
            .iter()
            .filter(|spawn| spawn.entity_type == 9)
            .collect::<Vec<_>>();
        assert_eq!(
            spawns.len(),
            EXPECTED_SPAWNS.len(),
            "{label}: exact Type-9 spawn count"
        );
        for (spawn, (expected_index, expected_position)) in spawns.into_iter().zip(EXPECTED_SPAWNS)
        {
            let position = [
                i16::from_le_bytes(spawn.pos_data_1[..2].try_into().unwrap()),
                i16::from_le_bytes(spawn.pos_data_1[2..4].try_into().unwrap()),
                i16::from_le_bytes(spawn.pos_data_2[..2].try_into().unwrap()),
            ];
            let spawn_label = format!("{label}: spawn {expected_index}");
            assert_eq!(spawn.index, expected_index, "{spawn_label}: index");
            assert_eq!(position, expected_position, "{spawn_label}: position");
            assert_eq!(spawn.param, 0, "{spawn_label}: parameter");
            assert_eq!(spawn.rotation, [0; 3], "{spawn_label}: rotation");
            assert_eq!(
                spawn.initial_damage_buffer_raw, 0,
                "{spawn_label}: initial damage buffer"
            );
            assert_eq!(
                spawn.model_overrides, [0; 4],
                "{spawn_label}: model overrides"
            );
            assert_eq!(spawn.extra, [0; 40], "{spawn_label}: extra payload");
            assert!(!spawn.has_animation, "{spawn_label}: animation flag");
            assert!(spawn.animation.is_none(), "{spawn_label}: animation block");
            assert!(!spawn.has_config, "{spawn_label}: config flag");
            assert!(spawn.config.is_none(), "{spawn_label}: config block");
        }
    }
}

#[v2k_test_support::retail_test]
fn type47_sub_d_profile_and_fresh_level1_spawns_are_exact_in_retail_tiers() {
    use v2k_formats::collision::SubDSteeringDescriptor;

    // Accepted constructor transcripts 20260730-034232 / 20260730-035135
    // join these Level-1 Type-47 births to process seeds 0x2B/0x2C/0x2D.
    const EXPECTED_SPAWNS: [(usize, [i16; 3]); 3] = [
        (11, [0xb700u16 as i16, 0, 0x7f00u16 as i16]),
        (12, [0xbe00u16 as i16, 0, 0x8500u16 as i16]),
        (13, [0xbe00u16 as i16, 0, 0x7d00u16 as i16]),
    ];

    for tier in ['0', '1', '2', '3'] {
        let system = load_ovl(&format!("{tier}X3XX.OVL"));
        let collision = v2k_formats::sections::parse_collision(&system).unwrap();
        let type47 = collision
            .entries
            .iter()
            .find(|entry| entry.model_ids == [302; 4])
            .expect("first-world Type-47 model 302 record");
        assert_eq!(
            type47.initializer_state_flags_raw, 0x2039,
            "tier {tier}: Type-47 +0xC0"
        );
        assert_eq!(
            type47.behavior_choices,
            [
                v2k_formats::collision::BehaviorChoice {
                    weight_rule_id: 1,
                    weight_multiplier: 9,
                    behavior_class_id: 32,
                },
                v2k_formats::collision::BehaviorChoice {
                    weight_rule_id: 1,
                    weight_multiplier: 1,
                    behavior_class_id: 6,
                },
            ],
            "tier {tier}: Type-47 +0x118 Always x9 Guard / Always x1 Wander"
        );
        assert_eq!(
            type47.alternate_behavior_class_ref, 12,
            "tier {tier}: Type-47 +0x124 Common Dying"
        );
        assert!(
            type47.active_subs.iter().any(|sub| sub == "D"),
            "tier {tier}: Type-47 authors Sub-D"
        );
        assert_eq!(
            type47.sub_d_steering_descriptor(),
            Some(SubDSteeringDescriptor {
                steering_divisor_raw: 64,
                couple_yaw_into_roll_raw: 0,
                enable_pitch_steering_raw: 0,
                forward_probe_raw: 0x0200,
                lateral_probe_raw: 0x0100,
                classifier_flags: 0x13,
                reserved_at_0x0b: 0,
            }),
            "tier {tier}: Type-47 Sub-D descriptor"
        );
        assert_eq!(
            type47.sub_a_propulsion_descriptor(),
            Some(v2k_formats::collision::SubAPropulsionDescriptor {
                acceleration_raw: 1_500,
                overspeed_correction_raw: -3_000,
                target_speed_base_raw: 300,
            }),
            "tier {tier}: Type-47 Sub-A"
        );
        assert_eq!(
            type47.sub_b_lateral_descriptor(),
            Some(v2k_formats::collision::SubBLateralDescriptor {
                projection_threshold_rate_raw: 10_000,
                correction_rate_raw: 1_000,
            }),
            "tier {tier}: Type-47 Sub-B"
        );
        assert_eq!(
            type47.sub_c_lift_descriptor(),
            Some(v2k_formats::collision::SubCLiftDescriptor {
                base_clearance_raw: 50,
                lift_range_raw: 75,
                strength_raw: 0x0030_0000,
                near_boost_range_raw: 100,
                damping_range_raw: 200,
                surface_mode_raw: 0,
                offset_sample_raw: 0,
                reserved_at_0x0e: [0; 2],
            }),
            "tier {tier}: Type-47 Sub-C"
        );

        let level_ovl = load_ovl(&format!("{tier}X13XX.OVL"));
        let level = v2k_formats::sections::parse_level(&level_ovl).unwrap();
        let spawns = level
            .entities
            .iter()
            .filter(|spawn| spawn.entity_type == 47)
            .collect::<Vec<_>>();
        assert_eq!(
            spawns.len(),
            EXPECTED_SPAWNS.len(),
            "tier {tier}: spawn count"
        );
        for (spawn, (expected_index, expected_position)) in spawns.into_iter().zip(EXPECTED_SPAWNS)
        {
            let position = [
                i16::from_le_bytes(spawn.pos_data_1[..2].try_into().unwrap()),
                i16::from_le_bytes(spawn.pos_data_1[2..4].try_into().unwrap()),
                i16::from_le_bytes(spawn.pos_data_2[..2].try_into().unwrap()),
            ];
            assert_eq!(spawn.index, expected_index, "tier {tier}: spawn index");
            assert_eq!(
                position, expected_position,
                "tier {tier}: spawn {expected_index} position"
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn section12_fun_00411250_sound_is_type17_only_in_the_first_world() {
    let ovl = load_ovl("0X3XX.OVL");
    let collision = v2k_formats::sections::parse_collision(&ovl).unwrap();
    for (local_index, global_type, expected) in [
        (4usize, 6u32, None),
        (7, 9, None),
        (15, 17, Some(92)),
        (44, 46, None),
        (45, 47, None),
        (64, 66, None),
        (65, 67, None),
    ] {
        assert_eq!(
            collision.entries[local_index].infected_model_presentation_sound_id(),
            expected,
            "type {global_type} +0x82"
        );
    }
}

#[v2k_test_support::retail_test]
fn section12_first_world_damage_selectors_keep_their_distinct_roles() {
    let ovl = load_ovl("0X3XX.OVL");
    let collision = v2k_formats::sections::parse_collision(&ovl).unwrap();

    // PRELOAD system level 2 contributes global types 0 and 1, so these are
    // the local 0X3XX records for global types 6, 66, and 68 respectively.
    // This is nonvisual data and is invariant across the presentation tiers.
    for (
        local_index,
        global_type,
        capability_flags,
        health_raw,
        accepted_hit,
        death,
        generic_hit,
    ) in [
        (4, 6, 0x20, 99_999, Some(7), None, None),
        (64, 66, 0x84, 99_999, Some(7), Some(62), None),
        (66, 68, 0x1040, 50_000, None, Some(62), None),
    ] {
        let record = &collision.entries[local_index];
        assert_eq!(record.id_field, capability_flags, "type {global_type}");
        assert_eq!(record.dims[0], health_raw, "type {global_type}");
        assert_eq!(
            record.accepted_hit_presentation_sound_id(),
            accepted_hit,
            "type {global_type} accepted-hit selector"
        );
        assert_eq!(
            record.death_sound_id(),
            death,
            "type {global_type} death selector"
        );
        assert_eq!(
            record.generic_hit_sound_id(),
            generic_hit,
            "type {global_type} generic-hit selector"
        );
    }

    for (local_index, global_type, expected_multiplier_q8) in
        [(4, 6, 256_i32), (64, 66, 256), (66, 68, 0)]
    {
        let header = &collision.entries[local_index].raw_header;
        let threshold = i32::from_le_bytes(header[0x20..0x24].try_into().unwrap());
        let multiplier_q8 = i32::from_le_bytes(header[0x3c..0x40].try_into().unwrap());
        assert_eq!(threshold, 200, "type {global_type} channel-2 threshold");
        assert_eq!(
            multiplier_q8, expected_multiplier_q8,
            "type {global_type} channel-2 multiplier"
        );
        let filtered_primary = multiplier_q8.wrapping_mul(2_000 - threshold) >> 8;
        assert_eq!(
            filtered_primary,
            if global_type == 68 { 0 } else { 1_800 },
            "type {global_type} primary-bullet damage"
        );
    }
}

#[v2k_test_support::retail_test]
fn section12_channel6_f780_filter_is_type9_and_type46() {
    let ovl = load_ovl("1X3XX.OVL");
    let collision = v2k_formats::sections::parse_collision(&ovl).unwrap();
    for (local_index, global_type, expected_multiplier_q8) in [
        (4usize, 6u32, 0_i32),
        (6, 8, 0),
        (7, 9, 512),
        (15, 17, 0),
        (44, 46, 512),
        (45, 47, 0),
        (64, 66, 0),
        (65, 67, 0),
    ] {
        let header = &collision.entries[local_index].raw_header;
        let threshold = i32::from_le_bytes(header[0x30..0x34].try_into().unwrap());
        let multiplier_q8 = i32::from_le_bytes(header[0x4c..0x50].try_into().unwrap());
        assert_eq!(threshold, 0, "type {global_type} channel-6 threshold");
        assert_eq!(
            multiplier_q8, expected_multiplier_q8,
            "type {global_type} channel-6 multiplier"
        );
        assert_eq!(
            multiplier_q8.wrapping_mul(2_000 - threshold) >> 8,
            expected_multiplier_q8.wrapping_mul(2_000) >> 8,
            "type {global_type} F780 channel-6 filter"
        );
    }
}

#[v2k_test_support::retail_test]
fn section12_type17_hit_and_death_sounds_match_the_focused_runtime_trace() {
    let ovl = load_ovl("1X3XX.OVL");
    let collision = v2k_formats::sections::parse_collision(&ovl).unwrap();

    // PRELOAD contributes global types 0 and 1, so first-world local record
    // 15 is cumulative type 17 (`spider`). The July-31 focused death traces
    // uniquely resolve the live PCM aliases to global sounds 84 and 94; keep
    // those runtime identities joined to their authored Section-12 selectors.
    let spider = &collision.entries[15];
    // DCA0 enters its low-health branch for the dead actor, then tests the
    // unsigned word at type-record +0xA0 before cadence or RNG. Authored zero
    // therefore proves that Type 17 reaches the deterministic common suffix.
    assert_eq!(&spider.raw_header[0xA0..0xA2], &[0, 0]);
    assert_eq!(
        &spider.raw_header[0x72..0x78],
        &[1, 0, 0xD0, 0x07, 0, 0],
        "global type 17/local record 15 enables E370 profile 1 with a 2000-ms lifetime"
    );
    assert_eq!(spider.dims[0], 5_000);
    assert_eq!(spider.accepted_hit_presentation_sound_id(), Some(84));
    assert_eq!(spider.death_sound_id(), Some(94));
    assert_eq!(spider.generic_hit_sound_id(), None);
    assert!(
        spider.active_subs.iter().any(|sub| sub == "D"),
        "global type 17 authors Sub-D"
    );
    assert_eq!(
        spider.sub_d_steering_descriptor(),
        Some(v2k_formats::collision::SubDSteeringDescriptor {
            steering_divisor_raw: 64,
            couple_yaw_into_roll_raw: 0,
            enable_pitch_steering_raw: 0,
            forward_probe_raw: 0x0200,
            lateral_probe_raw: 0x0100,
            classifier_flags: 0x13,
            reserved_at_0x0b: 0,
        }),
        "global type 17 Sub-D is flags 0x13, not Type-9 0x17"
    );
}

#[v2k_test_support::retail_test]
fn section12_type17_player_and_type93_attachment_contracts_are_tier_invariant() {
    use v2k_formats::collision::{
        BehaviorChoice, SubJAttachmentDescriptor, SubJAttachmentSlotDescriptor,
    };

    let expected_type17 = SubJAttachmentDescriptor {
        reserved_at_0x01: 0,
        slots: vec![SubJAttachmentSlotDescriptor {
            policy_word_raw: 1,
            local_offset_raw: [0, 10, 110],
        }]
        .into_boxed_slice(),
    };
    let expected_player = SubJAttachmentDescriptor {
        reserved_at_0x01: 0,
        slots: vec![
            SubJAttachmentSlotDescriptor {
                policy_word_raw: 0,
                local_offset_raw: [0; 3],
            };
            5
        ]
        .into_boxed_slice(),
    };
    let expected_type93 = SubJAttachmentDescriptor {
        reserved_at_0x01: 0,
        slots: vec![SubJAttachmentSlotDescriptor {
            policy_word_raw: 0,
            local_offset_raw: [0; 3],
        }]
        .into_boxed_slice(),
    };
    for name in ["0X3XX.OVL", "1X3XX.OVL", "2X3XX.OVL", "3X3XX.OVL"] {
        let ovl = load_ovl(name);
        let collision = v2k_formats::sections::parse_collision(&ovl).unwrap();
        // PRELOAD contributes global types 0 and 1: local records 15 and 44
        // are therefore global type 17 (`spider`) and type 46 (`player4`).
        assert_eq!(
            collision.entries[15].subsection("J"),
            Some(&[1, 0, 1, 0, 0, 0, 10, 0, 110, 0][..]),
            "{name}: type-17 raw Sub-J drifted"
        );
        assert_eq!(
            collision.entries[15].sub_j_attachment_descriptor(),
            Some(expected_type17.clone()),
            "{name}: type-17 typed Sub-J drifted"
        );
        assert_eq!(
            collision.entries[44].sub_j_attachment_descriptor(),
            Some(expected_player.clone()),
            "{name}: player authored Sub-J drifted"
        );
        assert_eq!(
            collision.entries[91].subsection("J"),
            Some(&[1, 0, 0, 0, 0, 0, 0, 0, 0, 0][..]),
            "{name}: type-93 raw Sub-J drifted"
        );
        assert_eq!(
            collision.entries[91].sub_j_attachment_descriptor(),
            Some(expected_type93.clone()),
            "{name}: type-93 authored Sub-J drifted"
        );
        let type93 = &collision.entries[91];
        assert_eq!(type93.type_tag, 0, "{name}: type-93 type tag drifted");
        assert_eq!(type93.scale, 1, "{name}: type-93 mass drifted");
        assert_eq!(type93.id_field, 0, "{name}: type-93 capability drifted");
        assert_eq!(type93.model_ids, [0; 4], "{name}: type-93 models drifted");
        assert_eq!(
            type93.dims,
            [1_000, 0, 0, 0, 0, 0],
            "{name}: type-93 dimensions drifted"
        );
        assert_eq!(
            type93.initializer_state_flags_raw, 0x4023,
            "{name}: type-93 initializer policy drifted"
        );
        assert_eq!(
            type93.common_axis_descriptor(),
            Default::default(),
            "{name}: type-93 common-axis descriptor drifted"
        );
        assert_eq!(
            type93.behavior_choices,
            [BehaviorChoice {
                weight_rule_id: 1,
                weight_multiplier: 1,
                behavior_class_id: 30,
            }],
            "{name}: type-93 Materialiser choice drifted"
        );
        assert_eq!(type93.behavior_rule_ref, 1);
        assert_eq!(type93.alternate_behavior_class_ref, 2);
        assert_eq!(type93.active_subs, ["J"]);
        assert_eq!(type93.accepted_hit_presentation_sound_id(), None);
        assert_eq!(type93.death_sound_id(), None);
        assert_eq!(type93.generic_hit_sound_id(), None);
    }
}

#[v2k_test_support::retail_test]
fn section12_first_world_status_components_are_sub_m_in_every_tier() {
    use v2k_formats::collision::StatusComponentDescriptor;

    for name in ["0X3XX.OVL", "1X3XX.OVL", "2X3XX.OVL", "3X3XX.OVL"] {
        let ovl = load_ovl(name);
        let collision = v2k_formats::sections::parse_collision(&ovl).unwrap();

        for (local_index, global_type, expected) in [
            (
                4,
                6,
                StatusComponentDescriptor {
                    raw_word_at_0x00: 0,
                    variable_bindings: [0, 0, 1, 0, 0, 0],
                    raw_tail: [0; 10],
                },
            ),
            (
                64,
                66,
                StatusComponentDescriptor {
                    raw_word_at_0x00: 8,
                    variable_bindings: [4, 3, 0, 2, 1, 0],
                    raw_tail: [0; 10],
                },
            ),
        ] {
            let record = &collision.entries[local_index];
            assert_eq!(record.subsection("G"), None, "{name}: type {global_type}");
            assert_eq!(
                record.status_component_descriptor(),
                Some(expected),
                "{name}: type {global_type}"
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn section12_quiet_death_profiles_are_exact_in_retail_tiers() {
    assert_quiet_death_profiles(false);
}

#[v2k_test_support::demo_test]
fn section12_quiet_death_profiles_are_exact_in_demo_tiers() {
    assert_quiet_death_profiles(true);
}

fn assert_quiet_death_profiles(demo: bool) {
    use v2k_formats::collision::BehaviorChoice;

    let tiers = if demo { 0..=1 } else { 0..=3 };
    for tier in tiers {
        let name = format!("{tier}X3XX.OVL");
        let ovl = if demo {
            load_demo_ovl(&name)
        } else {
            load_ovl(&name)
        };
        let build = if demo { "demo" } else { "retail" };
        let collision = v2k_formats::sections::parse_collision(&ovl).unwrap();

        // PRELOAD contributes global types 0 and 1, so these cumulative
        // first-world records live at local indices global_type - 2.
        for (
            local_index,
            global_type,
            models,
            mass,
            capabilities,
            health,
            initializer_flags,
            behavior_weight,
            behavior_class,
            death_sound,
            active_subs,
        ) in [
            (50, 52, [145; 4], 1, 0x100, 1, 0x73e7, 1, 0, None, &[][..]),
            (
                60,
                62,
                [38; 4],
                50,
                0,
                800,
                0x2004,
                5,
                6,
                None,
                &["B", "D", "F"][..],
            ),
            (
                66,
                68,
                [81; 4],
                200,
                0x1040,
                50_000,
                0x27224,
                1,
                0,
                Some(if demo { 64 } else { 62 }),
                &[][..],
            ),
        ] {
            let record = &collision.entries[local_index];
            let label = format!("{build} {name}: global type {global_type}");
            assert_eq!(record.model_ids, models, "{label}: models");
            assert_eq!(record.scale, mass, "{label}: mass");
            assert_eq!(record.id_field, capabilities, "{label}: capabilities");
            assert_eq!(record.dims[0], health, "{label}: health");
            assert_eq!(
                record.initializer_state_flags_raw, initializer_flags,
                "{label}: initializer state"
            );
            assert_eq!(
                record.behavior_choices,
                [BehaviorChoice {
                    weight_rule_id: 1,
                    weight_multiplier: behavior_weight,
                    behavior_class_id: behavior_class,
                }],
                "{label}: singleton behavior"
            );
            assert_eq!(record.behavior_rule_ref, 1, "{label}: behavior rule");
            assert_eq!(
                record.alternate_behavior_class_ref, 2,
                "{label}: quiet-death alternate"
            );
            assert_eq!(
                record
                    .active_subs
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
                active_subs,
                "{label}: component topology"
            );
            assert_eq!(
                record.accepted_hit_presentation_sound_id(),
                None,
                "{label}: accepted-hit cue"
            );
            assert_eq!(record.death_sound_id(), death_sound, "{label}: death cue");
            assert_eq!(record.generic_hit_sound_id(), None, "{label}: hit cue");
            assert_eq!(
                record.constructor_sound_attachment_id(),
                None,
                "{label}: constructor attachment"
            );
        }

        let level_name = format!("{}X13XX.OVL", &name[..1]);
        let level_ovl = if demo {
            load_demo_ovl(&level_name)
        } else {
            load_ovl(&level_name)
        };
        let level = v2k_formats::sections::parse_level(&level_ovl).unwrap();
        let cohort = level
            .entities
            .iter()
            .filter(|spawn| [52, 62, 68].contains(&spawn.entity_type))
            .map(|spawn| (spawn.index, spawn.entity_type))
            .collect::<Vec<_>>();
        assert_eq!(
            cohort,
            [
                (0, 52),
                (1, 52),
                (2, 52),
                (3, 52),
                (4, 62),
                (5, 68),
                (7, 52),
                (21, 52),
                (25, 62),
                (26, 62),
                (27, 52),
                (28, 52),
                (29, 52),
                (30, 52),
                (31, 52),
            ],
            "{build} {level_name}: exact quiet-death Section-13 cohort"
        );
    }
}

#[v2k_test_support::retail_test]
fn type54_abort_profile_and_payload_inputs_are_exact_in_retail_tiers() {
    type54_abort_profile_and_payload_inputs_are_exact(false);
}

#[v2k_test_support::demo_test]
fn type54_abort_profile_and_payload_inputs_are_exact_in_demo_tiers() {
    type54_abort_profile_and_payload_inputs_are_exact(true);
}

fn type54_abort_profile_and_payload_inputs_are_exact(demo: bool) {
    use v2k_formats::collision::BehaviorChoice;

    let tiers = if demo { 0..=1 } else { 0..=3 };
    for tier in tiers {
        let load: fn(&str) -> v2k_formats::ovl::OvlFile =
            if demo { load_demo_ovl } else { load_ovl };
        let build = if demo { "demo" } else { "retail" };
        let label = format!("{build} tier {tier}");
        let system_name = |level| format!("{tier}X{level}XX.OVL");

        let system3 = load(&system_name(3));
        let collision = v2k_formats::sections::parse_collision(&system3).unwrap();
        // PRELOAD system level 2 contributes global types 0 and 1, making
        // local record 52 the cumulative global type-54 row.
        let type54 = &collision.entries[52];
        assert_eq!(type54.model_ids, [560; 4], "{label}: models");
        assert_eq!(type54.scale, 100, "{label}: mass");
        assert_eq!(type54.id_field, 0, "{label}: capabilities");
        assert_eq!(type54.dims[0], 1_000, "{label}: health");
        assert_eq!(
            type54.initializer_state_flags_raw, 0x4027,
            "{label}: initializer state"
        );
        assert_eq!(
            type54.behavior_choices,
            [BehaviorChoice {
                weight_rule_id: 1,
                weight_multiplier: 1,
                behavior_class_id: 0,
            }],
            "{label}: initial behavior"
        );
        assert_eq!(type54.behavior_rule_ref, 1, "{label}: behavior rule");
        assert_eq!(
            type54.alternate_behavior_class_ref, 38,
            "{label}: death behavior"
        );
        assert_eq!(
            type54.common_axis_descriptor(),
            Default::default(),
            "{label}: common-axis descriptor"
        );
        assert!(type54.active_subs.is_empty(), "{label}: component topology");
        assert_eq!(
            type54.accepted_hit_presentation_sound_id(),
            None,
            "{label}: accepted-hit cue"
        );
        assert_eq!(type54.death_sound_id(), None, "{label}: death cue");
        assert_eq!(
            type54.constructor_sound_attachment_id(),
            None,
            "{label}: constructor attachment"
        );
        assert_eq!(
            type54.generic_hit_sound_id(),
            None,
            "{label}: generic-hit cue"
        );

        let level_ovl = load(&format!("{tier}X13XX.OVL"));
        let level = v2k_formats::sections::parse_level(&level_ovl).unwrap();
        let spawn = &level.entities[8];
        let position_raw = [
            i16::from_le_bytes(spawn.pos_data_1[..2].try_into().unwrap()),
            i16::from_le_bytes(spawn.pos_data_1[2..4].try_into().unwrap()),
            i16::from_le_bytes(spawn.pos_data_2[..2].try_into().unwrap()),
        ];
        assert_eq!(spawn.index, 8, "{label}: authored spawn index");
        assert_eq!(spawn.entity_type, 54, "{label}: authored entity type");
        assert_eq!(
            position_raw,
            [-23_040, 0, -4_096],
            "{label}: authored position"
        );
        assert_eq!(spawn.rotation, [0; 3], "{label}: authored rotation");
        assert_eq!(spawn.model_overrides, [0; 4], "{label}: model overrides");
        assert_eq!(&spawn.extra[8..12], &[0; 4], "{label}: entity +0x88 source");
        let terrain = v2k_formats::sections::parse_terrain(&level_ovl).unwrap();
        assert_eq!(terrain.header[0], -216_832, "{label}: sea header");

        // The active first-world pool appends system 6 after the Section-8
        // entries from systems 2, 3, and 5. Prove the named local entry still
        // resolves to global model 560 and carries the two independent header
        // radii consumed by class 38 and broad-phase collision.
        let mut global_model_base = 0_usize;
        for prefix_level in [2, 3, 5] {
            let prefix_ovl = load(&system_name(prefix_level));
            global_model_base += v2k_formats::sections::parse_models(&prefix_ovl)
                .unwrap()
                .all_entries
                .len();
        }
        let system6 = load(&system_name(6));
        let system6_models = v2k_formats::sections::parse_models(&system6).unwrap();
        let grock = system6_models
            .all_entries
            .iter()
            .find(|model| model.name.as_deref() == Some("grock"))
            .expect("system 6 must retain grock");
        assert_eq!(
            global_model_base + grock.index,
            560,
            "{label}: grock global id"
        );
        assert_eq!(grock.radius, 256, "{label}: class-38 model extent");
        assert_eq!(
            grock.collision_radius_raw, 284,
            "{label}: active-pair radius"
        );
    }
}

#[v2k_test_support::retail_test]
fn section12_type60_exploding_ring_uses_authored_splash_models_and_control_channel() {
    let ovl = load_ovl("1X3XX.OVL");
    let records = v2k_formats::sections::parse_collision(&ovl).unwrap();
    // System levels 1-2 contribute the first two global entity types, making
    // Level-3 local 58 the global type-60 Exploding Ring record.
    let splash = &records.entries[58];
    assert_eq!(splash.model_ids, [243; 4], "default model is exwave");
    assert_eq!(splash.flags, 0x200);
    assert_eq!(splash.initializer_state_flags_raw, 0x21084);
    assert_eq!(splash.behavior_choices.len(), 1);
    assert_eq!(splash.behavior_choices[0].behavior_class_id, 48);
    assert_eq!(splash.subsection("K"), Some(&[1, 0][..]));

    let models = v2k_formats::sections::parse_models(&ovl).unwrap();
    let named = |name: &str| {
        models
            .all_entries
            .iter()
            .find(|model| model.name.as_deref() == Some(name))
            .unwrap_or_else(|| panic!("missing {name}"))
    };
    let large = named("splash");
    let large_segment = named("splashseg");
    let medium = named("splashmid");
    let medium_segment = named("splashsegmid");
    // Level 3 begins at global model 13, so these are precisely the hard-entry
    // overrides 130/132 and their five instanced segment children 131/133.
    assert_eq!(large.index + 13, 130);
    assert_eq!(large_segment.index + 13, 131);
    assert_eq!(medium.index + 13, 132);
    assert_eq!(medium_segment.index + 13, 133);
    assert_eq!(large.instances.len(), 5);
    assert!(large
        .instances
        .iter()
        .all(|instance| instance.model_id == 131));
    assert_eq!(medium.instances.len(), 5);
    assert!(medium
        .instances
        .iter()
        .all(|instance| instance.model_id == 133));

    let mut birth = v2k_formats::models::AnimVars::default();
    birth.dynamic[1] = 0xffff;
    let settled = v2k_formats::models::AnimVars::default();
    assert_ne!(
        large_segment.materialize(&birth).vertices,
        large_segment.materialize(&settled).vertices,
        "segment type-8 vertices must consume dynamic channel one"
    );
    assert_ne!(
        medium_segment.materialize(&birth).vertices,
        medium_segment.materialize(&settled).vertices,
        "medium segment uses the same control channel"
    );
}

#[v2k_test_support::retail_test]
fn type46_hover_blocks_are_identical_across_display_tiers() {
    const SUB_A: &[u8] = &[0x78, 0x05, 0xd4, 0xfe, 0xb8, 0x0b];
    const SUB_B: &[u8] = &[0, 0, 0, 0, 0xf4, 0x01, 0, 0];
    const SUB_C: &[u8] = &[
        0x96, 0, 0x7d, 0, 0, 0, 0x90, 0, 0x64, 0, 0xc8, 0, 1, 1, 0, 0,
    ];
    const SUB_D: &[u8] = &[0x1c, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0];

    for name in ["0X3XX.OVL", "1X3XX.OVL", "2X3XX.OVL", "3X3XX.OVL"] {
        let ovl = load_ovl(name);
        let model = v2k_formats::sections::parse_collision(&ovl).unwrap();
        // System level 2 supplies global types 0-1, making L3 local 44 type 46.
        let hover = &model.entries[44];
        assert_eq!(hover.subsection("A"), Some(SUB_A), "{name}: Sub-A");
        assert_eq!(hover.subsection("B"), Some(SUB_B), "{name}: Sub-B");
        assert_eq!(hover.subsection("C"), Some(SUB_C), "{name}: Sub-C");
        assert_eq!(hover.subsection("D"), Some(SUB_D), "{name}: Sub-D");
        assert_eq!(hover.subsection("F"), None, "{name}: generic drag absent");
    }
}

// ── Section 13: Level descriptors ───────────────────────────────────────────

#[v2k_test_support::retail_test]
fn section13_level_descriptor() {
    let ovl = load_ovl("0X14XX.OVL");
    let level = v2k_formats::sections::parse_level(&ovl).unwrap();
    assert_eq!(level.name, "Medaeval", "level 14 should be named Medaeval");
    assert_eq!(level.time_trophy_deadline_seconds(), 270);
    assert_eq!(level.terrain_draw_depth, 22);
    assert!(level.sub_count > 0, "should have entity spawns");
    assert_eq!(
        level.entities.len(),
        level.sub_count as usize,
        "entity count should match sub_count"
    );
}

#[v2k_test_support::retail_test]
fn section13_time_trophy_deadlines_match_authored_worlds() {
    for (name, expected_seconds) in [
        ("0X13XX.OVL", 0),
        ("0X14XX.OVL", 270),
        ("0X26XX.OVL", 600),
        ("0X30XX.OVL", 180),
    ] {
        let ovl = load_ovl(name);
        let level = v2k_formats::sections::parse_level(&ovl).unwrap();
        assert_eq!(
            level.time_trophy_deadline_seconds(),
            expected_seconds,
            "{name}: Section 13 +0x5C deadline"
        );
    }
}

#[v2k_test_support::retail_test]
fn section13_first_world_abort_frame_resources_are_tier_invariant() {
    for name in ["0X13XX.OVL", "1X13XX.OVL", "2X13XX.OVL", "3X13XX.OVL"] {
        let ovl = load_ovl(name);
        let level = v2k_formats::sections::parse_level(&ovl).unwrap();
        assert_eq!(
            (level.sky_color_index, level.sky_model),
            (27, 305),
            "{name}: normal sky"
        );
        assert_eq!(
            (
                level.main_base_abort_sky_color_index,
                level.main_base_abort_sky_model,
            ),
            (32, 0),
            "{name}: Main Base abort sky",
        );
        assert_eq!(
            level.terrain_sprite_base, 0x650,
            "{name}: terrain sprite base"
        );
        assert_eq!(level.terrain_draw_depth, 0x15, "{name}: terrain draw depth");
        assert_eq!(
            level.time_trophy_deadline_seconds(),
            0,
            "{name}: time-trophy deadline"
        );
        assert_eq!(level.raw_u32(0x8C), Some(8), "{name}: frame request tail");
    }
}

#[v2k_test_support::retail_test]
fn section13_campaign_marker_routes_match_authored_retail_records() {
    let peasant_ovl = load_ovl("0X13XX.OVL");
    let peasant = v2k_formats::sections::parse_level(&peasant_ovl).unwrap();
    let peasant_routes: Vec<_> = peasant
        .campaign_records
        .iter()
        .filter_map(|record| record.marker_transition())
        .collect();
    assert_eq!(peasant.campaign_record_count, 4);
    assert_eq!(peasant_routes.len(), 2);
    assert_eq!(peasant_routes[0].destination_logical_level, 18);
    assert_eq!(peasant_routes[0].destination_global_level(), Some(30));
    assert_eq!(peasant_routes[0].arrival_raw, [7424, 5120, 9728]);
    assert_eq!(peasant_routes[0].marker_subtype, 2);
    assert_eq!(peasant_routes[1].destination_logical_level, 2);
    assert_eq!(peasant_routes[1].destination_global_level(), Some(14));
    assert_eq!(peasant_routes[1].arrival_raw, [17408, 2560, -32512]);
    assert_eq!(peasant_routes[1].marker_subtype, 1);

    let cistern_ovl = load_ovl("0X30XX.OVL");
    let cistern = v2k_formats::sections::parse_level(&cistern_ovl).unwrap();
    let return_route = cistern
        .campaign_records
        .iter()
        .filter_map(|record| record.marker_transition())
        .find(|route| route.marker_subtype == 4)
        .expect("Cistern subtype-4 return route");
    assert_eq!(return_route.destination_logical_level, 1);
    assert_eq!(return_route.destination_global_level(), Some(13));
    assert_eq!(return_route.arrival_raw, [-27392, 0, -5120]);
}

#[v2k_test_support::retail_test(demo)]
fn section13_late_demo_preserves_retail_first_world_campaign_data() {
    for name in [
        "0X13XX.OVL",
        "1X13XX.OVL",
        "0X14XX.OVL",
        "1X14XX.OVL",
        "0X15XX.OVL",
        "1X15XX.OVL",
        "0X50XX.OVL",
        "1X50XX.OVL",
    ] {
        let retail_ovl = load_ovl(name);
        let demo_ovl = load_demo_ovl(name);
        let retail_terrain = retail_ovl
            .section(v2k_formats::sections::idx::TERRAIN_HEIGHTMAP)
            .expect("retail Section 10");
        let demo_terrain = demo_ovl
            .section(v2k_formats::sections::idx::TERRAIN_HEIGHTMAP)
            .expect("demo Section 10");
        assert_eq!(
            demo_terrain.data, retail_terrain.data,
            "{name}: terrain and authored marker cells"
        );

        let retail_level_section = retail_ovl
            .section(v2k_formats::sections::idx::LEVEL_DESCRIPTORS)
            .expect("retail Section 13");
        let demo_level_section = demo_ovl
            .section(v2k_formats::sections::idx::LEVEL_DESCRIPTORS)
            .expect("demo Section 13");
        // Only compiler/build residue after the NUL-terminated name differs
        // inside the first 0x40 bytes. Every descriptor field, spawn/config/
        // animation byte, and trailing campaign record is exact thereafter.
        assert_eq!(
            &demo_level_section.data[0x40..],
            &retail_level_section.data[0x40..],
            "{name}: Section 13 descriptor and trailing data"
        );

        let retail_level = v2k_formats::sections::parse_level(&retail_ovl).unwrap();
        let demo_level = v2k_formats::sections::parse_level(&demo_ovl).unwrap();
        assert_eq!(
            demo_level.campaign_record_count, retail_level.campaign_record_count,
            "{name}: campaign record count"
        );
        assert_eq!(
            demo_level.campaign_records, retail_level.campaign_records,
            "{name}: exact 0x20-byte campaign/goal records"
        );
        if name.contains("15XX") {
            let routes: Vec<_> = demo_level
                .campaign_records
                .iter()
                .filter_map(|record| record.marker_transition())
                .map(|route| {
                    (
                        route.destination_global_level(),
                        route.arrival_raw,
                        route.marker_subtype,
                    )
                })
                .collect();
            assert_eq!(
                routes,
                vec![
                    (Some(19), [24320, 1536, 19712], 1),
                    (Some(30), [-21248, 0, 18688], 2),
                    (Some(17), [-13312, 5120, -15616], 3),
                    (Some(14), [-16896, 4096, 24320], 4),
                ],
                "{name}: demo retains retail routes to three omitted overlays"
            );
        }
    }
}

#[v2k_test_support::retail_test(demo)]
fn section2_late_demo_appends_promotional_text_block() {
    let demo_system_2 = load_demo_ovl("0X2XX.OVL");
    let demo_system_2_strings = v2k_formats::sections::extract_strings(&demo_system_2);
    assert_eq!(demo_system_2_strings.len(), 118);

    let mut low_tier_tail = None;
    for name in ["0X3XX.OVL", "1X3XX.OVL"] {
        let retail_ovl = load_ovl(name);
        let demo_ovl = load_demo_ovl(name);
        let retail = v2k_formats::sections::extract_strings(&retail_ovl);
        let demo = v2k_formats::sections::extract_strings(&demo_ovl);
        assert_eq!(retail.len(), 182, "{name}: retail common-gameplay strings");
        assert_eq!(demo.len(), 194, "{name}: demo common-gameplay strings");

        let promotional = &demo[182..];
        assert_eq!(promotional.len(), 12);
        assert!(promotional[0].starts_with("V2000 ist  Anfang Oktober"));
        assert_eq!(
            promotional[1],
            "30 riesige Welten, die entdeckt werden wollen"
        );
        assert_eq!(
            promotional[3],
            "Viele verschiedene Wege verbinden die Welten"
        );
        assert_eq!(
            promotional[4],
            "2 \"Fahr\"-Modi: mit dem Luftkissen oder im Flugmodus"
        );
        assert!(promotional[10].contains("http://v2000.grolier.co.uk"));
        assert!(promotional[11].starts_with("Leertaste zum Fortfahren"));
        assert!(!retail.iter().any(|line| line.contains("grolier.co.uk")));

        // The demo's level-2 pool has 118 entries, placing local level-3
        // promotional ids 182..193 at cumulative global ids 300..311.
        assert_eq!(demo_system_2_strings.len() + 182, 300);
        assert_eq!(demo_system_2_strings.len() + demo.len(), 312);

        if let Some(low_tier_tail) = &low_tier_tail {
            assert_eq!(promotional, low_tier_tail, "{name}: tier-invariant text");
        } else {
            low_tier_tail = Some(promotional.to_vec());
        }
    }
}

// ── Intro2 model-slot regression ───────────────────────────────────────────

#[v2k_test_support::retail_test]
fn section13_intro2_model_slots_match_the_retail_constructor_layout() {
    let ovl = load_ovl("0X50XX.OVL");
    let level = v2k_formats::sections::parse_level(&ovl).unwrap();
    assert_eq!(level.name, "Intro2");
    assert_eq!(level.entities.len(), 62);

    // FUN_004104B0 is passed spawn+0x04 and consumes dwords 10..13, so the
    // overrides are the four dwords at spawn offsets +0x2C..+0x38. All four
    // authored meteors select global model 560 (`grock`) in every state.
    for index in [31usize, 33, 34, 35] {
        let meteor = &level.entities[index];
        assert_eq!(meteor.entity_type, 34, "Intro2 entity {index}");
        assert_eq!(meteor.model_overrides, [560; 4], "Intro2 entity {index}");
    }

    // This type-66 record deliberately borrows type-66 behavior while using
    // the medieval peasant-hut/destroyed-hut models. Shifting the slots left
    // makes slot zero empty and produces the spurious factory seen in-port.
    let peasant_hut = &level.entities[36];
    assert_eq!(peasant_hut.entity_type, 66);
    assert_eq!(peasant_hut.model_overrides, [364, 365, 364, 365]);
    assert_eq!(peasant_hut.rotation, [0x4000, 0, 0]);

    // The later real factory uses its type default in slots 0/2 and the
    // authored dying-factory override in slots 1/3.
    let factory = &level.entities[51];
    assert_eq!(factory.entity_type, 66);
    assert_eq!(factory.model_overrides, [0, 225, 0, 225]);
    assert_eq!(factory.rotation, [21117, 0, 0]);
}

#[v2k_test_support::retail_test]
fn section12_constructor_sound_attachments_are_exhaustively_types_15_44_61_87_108_111() {
    for name in ["0X3XX.OVL", "1X3XX.OVL", "2X3XX.OVL", "3X3XX.OVL"] {
        let ovl = load_ovl(name);
        let collision = v2k_formats::sections::parse_collision(&ovl).unwrap();
        // PRELOAD contributes global types 0 and 1, so first-world records live
        // at local index global_type - 2.
        const EXPECTED_ATTACHMENTS: [(usize, u32, u16, u16); 6] = [
            (13, 15, 11, 276),
            (42, 44, 11, 276),
            (59, 61, 44, 82),
            (85, 87, 11, 270),
            (106, 108, 11, 1167),
            (109, 111, 100, 16),
        ];

        let mut actual_attachments = Vec::new();
        for record in &collision.entries {
            if let Some(sound_id) = record.constructor_sound_attachment_id() {
                let global_type = (record.index + 2) as u32;
                actual_attachments.push((record.index, global_type, sound_id, record.model_ids[0]));
            }
        }

        assert_eq!(
            actual_attachments,
            EXPECTED_ATTACHMENTS.to_vec(),
            "{name}: constructor sound attachments must be exhaustively and exactly these six rows"
        );
    }
}

#[v2k_test_support::retail_test]
fn intro2_flyer_section12_descriptors() {
    let ovl = load_ovl("1X3XX.OVL");
    let collision = v2k_formats::sections::parse_collision(&ovl).unwrap();

    // Type 15 (wasp, local index 13)
    let type15 = &collision.entries[13];
    assert_eq!(type15.model_ids, [276, 276, 276, 276]);
    assert!(type15.sub_a_propulsion_descriptor().is_none());
    assert_eq!(
        type15
            .sub_b_lateral_descriptor()
            .unwrap()
            .projection_threshold_rate_raw,
        1_000_000
    );
    assert!(type15.sub_c_lift_descriptor().is_none());
    let sub_d15 = type15.sub_d_steering_descriptor().unwrap();
    assert_eq!(sub_d15.steering_divisor_raw, 64);
    assert_eq!(sub_d15.couple_yaw_into_roll_raw, 1);
    assert_eq!(sub_d15.classifier_flags, 0);
    assert!(type15.actor_animation_descriptor().is_none());
    assert!(type15.sub_h_external_frame_descriptor().is_none());
    assert_eq!(type15.behavior_choices.len(), 1);
    assert_eq!(type15.behavior_choices[0].behavior_class_id, 7);

    // Type 87 (deathwas, local index 85)
    let type87 = &collision.entries[85];
    assert_eq!(type87.model_ids, [270, 270, 270, 270]);
    assert!(type87.sub_a_propulsion_descriptor().is_none());
    assert_eq!(
        type87
            .sub_b_lateral_descriptor()
            .unwrap()
            .projection_threshold_rate_raw,
        1_000_000
    );
    assert!(type87.sub_c_lift_descriptor().is_none());
    let sub_d87 = type87.sub_d_steering_descriptor().unwrap();
    assert_eq!(sub_d87.steering_divisor_raw, 64);
    assert_eq!(sub_d87.couple_yaw_into_roll_raw, 1);
    assert_eq!(sub_d87.classifier_flags, 0);
    assert!(type87.actor_animation_descriptor().is_none());
    assert!(type87.sub_h_external_frame_descriptor().is_none());
    assert_eq!(type87.behavior_choices.len(), 1);
    assert_eq!(type87.behavior_choices[0].behavior_class_id, 7);

    let ovl_intro = load_ovl("0X50XX.OVL");
    let level = v2k_formats::sections::parse_level(&ovl_intro).unwrap();
    let spawn44 = level
        .entities
        .iter()
        .find(|e| e.index == 44)
        .expect("spawn 44");
    assert_eq!(spawn44.entity_type, 15);
    let pos44 = [
        i16::from_le_bytes(spawn44.pos_data_1[..2].try_into().unwrap()),
        i16::from_le_bytes(spawn44.pos_data_1[2..4].try_into().unwrap()),
        i16::from_le_bytes(spawn44.pos_data_2[..2].try_into().unwrap()),
    ];
    assert_eq!(pos44, [-17152, 0, 32256]);

    let spawn46 = level
        .entities
        .iter()
        .find(|e| e.index == 46)
        .expect("spawn 46");
    assert_eq!(spawn46.entity_type, 87);
    let pos46 = [
        i16::from_le_bytes(spawn46.pos_data_1[..2].try_into().unwrap()),
        i16::from_le_bytes(spawn46.pos_data_1[2..4].try_into().unwrap()),
        i16::from_le_bytes(spawn46.pos_data_2[..2].try_into().unwrap()),
    ];
    assert_eq!(pos46, [-18176, 1024, 4864]);
}

#[v2k_test_support::retail_test]
fn section13_intro2_type13_spawn_matches_ttd_constructor_identity() {
    let ovl = load_ovl("0X50XX.OVL");
    let level = v2k_formats::sections::parse_level(&ovl).unwrap();
    assert_eq!(level.name, "Intro2");

    // TTD V200001.run constructs handle 04fc0001 at these authored 8.8 words.
    const EXPECTED: [i16; 3] = [0xB300u16 as i16, 0x0200, 0x2800];
    let spawns = level
        .entities
        .iter()
        .filter(|spawn| spawn.entity_type == 13)
        .collect::<Vec<_>>();
    assert_eq!(spawns.len(), 1, "Intro2 type-13 spawn count");
    assert_eq!(spawns[0].index, 0, "Intro2 type-13 spawn index");
    let position = [
        i16::from_le_bytes(spawns[0].pos_data_1[..2].try_into().unwrap()),
        i16::from_le_bytes(spawns[0].pos_data_1[2..4].try_into().unwrap()),
        i16::from_le_bytes(spawns[0].pos_data_2[..2].try_into().unwrap()),
    ];
    assert_eq!(position, EXPECTED, "Intro2 type-13 spawn 0 position");
    assert_eq!(spawns[0].param, 1);
    assert_eq!(spawns[0].rotation, [0xC000, 0, 0]);
    assert_eq!(spawns[0].model_overrides, [0; 4]);
}

#[v2k_test_support::retail_test]
fn section12_type13_dragon_descriptor() {
    let ovl3 = load_ovl("1X3XX.OVL");
    let collision = v2k_formats::sections::parse_collision(&ovl3).unwrap();
    // Type 13 is at local index 11 (13 - 2)
    let type13 = &collision.entries[11];
    assert_eq!(type13.model_ids, [291, 291, 291, 291]);
    assert_eq!(type13.active_subs, ["D", "E", "G", "K", "L"]);
    assert!(type13.sub_a_propulsion_descriptor().is_none());
    assert!(type13.sub_b_lateral_descriptor().is_none());
    assert!(type13.sub_c_lift_descriptor().is_none());
    let sub_d = type13.sub_d_steering_descriptor().unwrap();
    assert_eq!(sub_d.steering_divisor_raw, 64);
    assert_eq!(sub_d.couple_yaw_into_roll_raw, 1);
    assert_eq!(sub_d.enable_pitch_steering_raw, 0);
    assert_eq!(sub_d.forward_probe_raw, 512);
    assert_eq!(sub_d.lateral_probe_raw, 256);
    assert_eq!(sub_d.classifier_flags, 0);
    assert!(type13.projectile_emitter_descriptor().is_some());
    assert!(type13.sub_g_payload().is_some());
    assert_eq!(type13.behavior_choices.len(), 2);
    assert_eq!(type13.behavior_choices[0].behavior_class_id, 5);
    assert_eq!(type13.behavior_choices[1].behavior_class_id, 7);
}

#[v2k_test_support::retail_test]
fn section13_intro2_type47_spawns_match_ttd_first_query_identities() {
    let ovl = load_ovl("0X50XX.OVL");
    let level = v2k_formats::sections::parse_level(&ovl).unwrap();
    assert_eq!(level.name, "Intro2");

    // TTD V200001.run construction order: seeds 0x06/0x07/0x08 at these
    // authored X/Z words. This is not the Level-1 trio B700/7F00,
    // BE00/8500, BE00/7D00.
    const EXPECTED: [[i16; 3]; 3] = [
        [0xC000u16 as i16, 0, 0x1600],
        [0xBB00u16 as i16, 0, 0x7B00],
        [0xBE00u16 as i16, 0, 0x7C00],
    ];
    let spawns = level
        .entities
        .iter()
        .filter(|spawn| spawn.entity_type == 47)
        .collect::<Vec<_>>();
    assert_eq!(spawns.len(), EXPECTED.len(), "Intro2 Type-47 spawn count");
    assert_eq!(
        spawns.iter().map(|spawn| spawn.index).collect::<Vec<_>>(),
        [6, 7, 8],
        "Intro2 Type-47 spawn indices"
    );
    for (spawn, expected_position) in spawns.into_iter().zip(EXPECTED) {
        let position = [
            i16::from_le_bytes(spawn.pos_data_1[..2].try_into().unwrap()),
            i16::from_le_bytes(spawn.pos_data_1[2..4].try_into().unwrap()),
            i16::from_le_bytes(spawn.pos_data_2[..2].try_into().unwrap()),
        ];
        assert_eq!(
            position, expected_position,
            "Intro2 Type-47 spawn {} position",
            spawn.index
        );
    }
}

#[v2k_test_support::retail_test]
fn section13_intro2_type77_spawn_45_descriptor() {
    let ovl = load_ovl("0X50XX.OVL");
    let level = v2k_formats::sections::parse_level(&ovl).unwrap();
    assert_eq!(level.name, "Intro2");
    assert_eq!(level.entities.len(), 62);
    assert_eq!(level.entities[45].entity_type, 77);

    let ovl3 = load_ovl("1X3XX.OVL");
    let collision = v2k_formats::sections::parse_collision(&ovl3).unwrap();
    let models = v2k_formats::sections::parse_models(&ovl3).unwrap();
    assert_eq!(
        models.all_entries[270 - 13].name.as_deref(),
        Some("deathwas")
    );
    assert_eq!(
        models.all_entries[271 - 13].name.as_deref(),
        Some("bluebee2")
    );
    assert_eq!(models.all_entries[276 - 13].name.as_deref(), Some("wasp"));
    let type77 = &collision.entries[75];
    assert_eq!(type77.model_ids, [271; 4]);
    assert!(type77.sub_a_propulsion_descriptor().is_some());
    assert!(type77.sub_b_lateral_descriptor().is_some());
    assert!(type77.sub_c_lift_descriptor().is_some());
    assert!(!type77.sub_g_payload().is_some());
    assert_eq!(type77.active_subs, ["A", "B", "C", "D", "E", "H"]);
    let sub_a = type77.sub_a_propulsion_descriptor().unwrap();
    assert_eq!(sub_a.acceleration_raw, 1500);
    assert_eq!(sub_a.overspeed_correction_raw, -3000);
    assert_eq!(sub_a.target_speed_base_raw, 300);
    let sub_b = type77.sub_b_lateral_descriptor().unwrap();
    assert_eq!(sub_b.projection_threshold_rate_raw, 10000);
    assert_eq!(sub_b.correction_rate_raw, 1000);
    let sub_c = type77.sub_c_lift_descriptor().unwrap();
    assert_eq!(sub_c.base_clearance_raw, 50);
    assert_eq!(sub_c.lift_range_raw, 75);
    assert_eq!(sub_c.strength_raw, 3145728);
    assert_eq!(type77.behavior_choices.len(), 5);
    assert_eq!(
        type77.behavior_choices,
        [
            v2k_formats::collision::BehaviorChoice {
                weight_rule_id: 1,
                weight_multiplier: 10,
                behavior_class_id: 33,
            },
            v2k_formats::collision::BehaviorChoice {
                weight_rule_id: 8,
                weight_multiplier: 5,
                behavior_class_id: 26,
            },
            v2k_formats::collision::BehaviorChoice {
                weight_rule_id: 2,
                weight_multiplier: 20,
                behavior_class_id: 7,
            },
            v2k_formats::collision::BehaviorChoice {
                weight_rule_id: 1,
                weight_multiplier: 1,
                behavior_class_id: 4,
            },
            v2k_formats::collision::BehaviorChoice {
                weight_rule_id: 1,
                weight_multiplier: 2,
                behavior_class_id: 5,
            },
        ]
    );
    assert_eq!(type77.constructor_sound_attachment_id(), None);
    assert_eq!(type77.death_sound_id(), Some(75));
    assert_eq!(type77.accepted_hit_presentation_sound_id(), Some(84));
    assert_eq!(type77.generic_hit_sound_id(), None);
}

#[v2k_test_support::retail_test]
fn section13_intro2_all_entities_inventory() {
    let ovl = load_ovl("0X50XX.OVL");
    let level = v2k_formats::sections::parse_level(&ovl).unwrap();
    assert_eq!(level.entities.len(), 62);
    let mut type_counts = std::collections::BTreeMap::new();
    for ent in &level.entities {
        *type_counts.entry(ent.entity_type).or_insert(0) += 1;
    }
    assert_eq!(type_counts.get(&13), Some(&1)); // Dragon / Ptersect (spawn 0)
    assert_eq!(type_counts.get(&15), Some(&1)); // Wasp flyer (spawn 44)
    assert_eq!(type_counts.get(&87), Some(&1)); // Deathwas flyer (spawn 46)
    assert_eq!(type_counts.get(&77), Some(&1)); // Flyer (spawn 45)
    assert_eq!(type_counts.get(&47), Some(&3)); // Spider guards (spawns 6, 7, 8)
    assert_eq!(type_counts.get(&26), Some(&2)); // Stag beetle (spawns 10, 25)
    assert_eq!(type_counts.get(&9), Some(&13)); // Peasants / villagers (13 entities)
    assert_eq!(type_counts.get(&8), Some(&3)); // Scientists (3 entities)
    assert_eq!(type_counts.get(&34), Some(&4)); // Meteors (4 entities)
    assert_eq!(type_counts.get(&66), Some(&2)); // Buildings / factories (spawns 36, 51)
    assert_eq!(type_counts.get(&67), Some(&1)); // Hive (spawn 24)
}

// ── Section 14: Entity linkage ──────────────────────────────────────────────

#[v2k_test_support::retail_test]
fn section12_type26_retains_retail_defecate_lifetime_and_choice() {
    assert_type26_defecate_lifetime_and_choice("retail", 67, load_ovl("1X3XX.OVL"));
}

#[v2k_test_support::demo_test]
fn section12_type26_retains_demo_defecate_lifetime_and_choice() {
    assert_type26_defecate_lifetime_and_choice("demo", 69, load_demo_ovl("1X3XX.OVL"));
}

fn assert_type26_defecate_lifetime_and_choice(
    label: &str,
    expected_lifetime_ms: u16,
    ovl: v2k_formats::ovl::OvlFile,
) {
    use v2k_formats::collision::SubDSteeringDescriptor;

    let collision = v2k_formats::sections::parse_collision(&ovl).unwrap();
    // System level 2 contributes global types 0 and 1, so local level-3
    // entry 24 is cumulative entity type 26.
    let type26 = &collision.entries[24];
    assert_eq!(
        type26.terrain_contact_task_lifetime_ms(),
        expected_lifetime_ms,
        "{label}"
    );
    assert_eq!(
        type26.active_subs,
        ["A", "B", "C", "D", "H"],
        "{label}: type-26 topology"
    );
    let sub_h = type26
        .sub_h_external_frame_descriptor()
        .unwrap_or_else(|| panic!("{label}: type-26 Sub-H"));
    assert_eq!(sub_h.records.len(), 6, "{label}: type-26 six H records");
    assert_eq!(sub_h.completion_sound_id, None, "{label}");
    assert_eq!(
        type26.sub_a_propulsion_descriptor(),
        Some(v2k_formats::collision::SubAPropulsionDescriptor {
            acceleration_raw: 1_500,
            overspeed_correction_raw: -3_000,
            target_speed_base_raw: 200,
        }),
        "{label}: type-26 Sub-A"
    );
    assert_eq!(
        type26.sub_b_lateral_descriptor(),
        Some(v2k_formats::collision::SubBLateralDescriptor {
            projection_threshold_rate_raw: 10_000,
            correction_rate_raw: 10_000,
        }),
        "{label}: type-26 Sub-B"
    );
    assert_eq!(
        type26.sub_c_lift_descriptor(),
        Some(v2k_formats::collision::SubCLiftDescriptor {
            base_clearance_raw: 75,
            lift_range_raw: 75,
            strength_raw: 3_145_728,
            near_boost_range_raw: 100,
            damping_range_raw: 200,
            surface_mode_raw: 0,
            offset_sample_raw: 0,
            reserved_at_0x0e: [0, 0],
        }),
        "{label}: type-26 Sub-C is terrain-only"
    );
    assert_eq!(
        type26.sub_d_steering_descriptor(),
        Some(SubDSteeringDescriptor {
            steering_divisor_raw: 128,
            couple_yaw_into_roll_raw: 0,
            enable_pitch_steering_raw: 0,
            forward_probe_raw: 0x0200,
            lateral_probe_raw: 200,
            classifier_flags: 0x12,
            reserved_at_0x0b: 0,
        }),
        "{label}: type-26 Sub-D is flags 0x12, not Type-9 0x17"
    );
    assert_eq!(
        type26.behavior_choices,
        [
            v2k_formats::collision::BehaviorChoice {
                weight_rule_id: 8,
                weight_multiplier: 2,
                behavior_class_id: 26,
            },
            v2k_formats::collision::BehaviorChoice {
                weight_rule_id: 1,
                weight_multiplier: 4,
                behavior_class_id: 33,
            },
            v2k_formats::collision::BehaviorChoice {
                weight_rule_id: 1,
                weight_multiplier: 3,
                behavior_class_id: 4,
            },
        ],
        "{label}"
    );
}

#[v2k_test_support::retail_test]
fn system_level3_radar_projection_tables_follow_display_tier_dimensions() {
    for (name, dimensions) in [("0X3XX.OVL", (64, 64)), ("1X3XX.OVL", (96, 96))] {
        let ovl = load_ovl(name);
        let linkage = v2k_formats::sections::parse_linkage(&ovl, 0).unwrap();
        assert_eq!(linkage.records.len(), 4, "{name}: preserve unrelated table");

        let projection = linkage.radar_projection_tables().unwrap();
        assert_eq!((projection.width, projection.height), dimensions, "{name}");
        assert_eq!(
            projection.shade_grid.len(),
            dimensions.0 * dimensions.1,
            "{name}: full-resolution shade grid"
        );
        assert_eq!(
            projection.coordinate_tuples.len(),
            dimensions.0 / 2 * (dimensions.1 / 2),
            "{name}: half-resolution coordinate grid"
        );
        assert_eq!(
            projection.mask_grid.len(),
            dimensions.0 / 2 * (dimensions.1 / 2),
            "{name}: half-resolution mask grid"
        );
    }
}

#[v2k_test_support::retail_test]
fn section14_linkage() {
    let ovl = load_ovl("0X14XX.OVL");
    let table = v2k_formats::sections::parse_linkage(&ovl, 0).unwrap();
    assert_eq!(
        table.records.len(),
        1,
        "level 14 should have 1 linkage record"
    );
    assert!(
        !table.trailing_data().is_empty(),
        "should have trailing data"
    );
    let dependencies = table.sprite_dependencies().expect("type-1 sprite list");
    assert_eq!(
        dependencies.sprite_ids.len(),
        table.records[0].entry_count as usize
    );
}

// ── Batch: all 212 OVLs ────────────────────────────────────────────────────

#[v2k_test_support::retail_test]
fn parse_multiple_ovls() {
    let dir = overlay_dir();

    let mut parsed = 0;
    let mut with_sprites = 0;

    for entry in std::fs::read_dir(&dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "OVL") {
            let data = std::fs::read(&path).unwrap();
            let ovl = v2k_formats::ovl::OvlFile::parse(&data).unwrap();
            assert_eq!(ovl.sections.len(), 15);
            parsed += 1;

            // Even an empty Section 3 retains the texture allocation header
            // and a zero entry-data-size dword. The canonical extractor's
            // has_sprite_payload uses this dword, not OvlFile::has_data(3).
            let section = ovl
                .section(v2k_formats::sections::idx::SPRITE_ATLAS)
                .unwrap();
            let entry_data_size = v2k_formats::ovl::read_u32(&section.data, 0)
                .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            if entry_data_size != 0 {
                v2k_formats::sections::parse_sprites(&ovl)
                    .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
                with_sprites += 1;
            } else {
                assert_eq!(
                    section.data,
                    [0; 4],
                    "{}: empty atlas must contain only its zero entry-data-size dword",
                    path.display()
                );
            }
        }
    }

    assert_eq!(parsed, 212, "should parse all 212 OVL files");
    // FORMAT_DOCUMENTATION.md: 12 sprite-bearing levels across four tiers.
    assert_eq!(with_sprites, 48, "all authored sprite atlases must parse");
    eprintln!("Parsed {parsed} OVLs, {with_sprites} with sprites");
}
