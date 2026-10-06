//! Headless proof that the recovered edge-quad gate submits quads for the
//! real parsed spider/newant/stag models, plus stage-by-stage drop counts.
//!
//! Fixtures are already **world** coordinates (`vertex / 100`). That is the
//! selector contract. Live `draw_tris_gl` converts raw-local endpoints with
//! `position + O · (raw · scale)` before calling it. The fill keeps the
//! constructor's screen corners and paints them in a HUD-style pixel ortho.

use v2k_formats::models::{AnimVars, ModelEdgeStyle};
use v2k_game::session::GameSession;
use v2k_game::targetter::TARGETTER_TERRAIN_CROSSHAIR_MODEL_ID;
use v2k_render::edge_quads::{select_edge_quads_with_stats, EdgeQuadSubmission, ScreenIntrinsics};
use v2k_render::projection::SceneProjectionAuthority;
use v2k_render::{FaceMaterial, WorldSpriteBlend};

fn fallback_material() -> FaceMaterial {
    FaceMaterial {
        palette_rgb555: None,
        color: [1.0; 3],
        emissive: [0.0; 3],
        texture: None,
        blend: WorldSpriteBlend::Masked,
        flat_shade_row: 31,
    }
}

/// One record through the recovered gate, exposing the clip-drop stage.
fn crate_gate(
    start_px: (i32, i32),
    end_px: (i32, i32),
    start_depth: i32,
    end_depth: i32,
    intr: &ScreenIntrinsics,
    widths: (u16, u16),
    size: i32,
) -> Option<[[f32; 2]; 4]> {
    v2k_render::edge_quads::build_edge_quad(
        start_px,
        end_px,
        start_depth,
        end_depth,
        intr,
        widths.0,
        widths.1,
        size,
    )
    .map(|(corners, _)| corners)
}

fn intrinsics() -> ScreenIntrinsics {
    // 4:3, fov 60: m00 = (1/tan30)/aspect, m11 = 1/tan30.
    let f = 1.0 / 30f32.to_radians().tan();
    ScreenIntrinsics {
        width_px: 1024,
        height_px: 768,
        scale_x: (f / (4.0 / 3.0)) * 512.0,
        scale_y: f * 384.0,
        center_x: 512.0,
        center_y: 384.0,
        clip_near: 0.1,
        clip_far: 500.0,
        projection_effect: v2k_render::projection::ProjectionEffect::None,
    }
}

#[v2k_test_support::retail_test]
fn spider_stag_and_newant_submit_leg_quads_through_the_recovered_gate() {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(13, 1).unwrap();

    let intr = intrinsics();
    // Camera 15 units back on -Z looking toward +Z.
    let cam = [0.0_f32, 0.0, -15.0];
    let basis = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, -1.0]];

    for name in ["spider", "stag", "newant"] {
        let Some((_id, model)) = session.cache.global_model_ids().into_iter().find_map(|id| {
            session
                .cache
                .global_model(id)
                .filter(|m| m.name.as_deref() == Some(name))
                .map(|m| (id, m))
        }) else {
            panic!("{name} must exist in the loaded pools");
        };
        let materialized = model.materialize(&AnimVars::default());
        // A healthy draw resolves every vertex (view-pin policies applied);
        // simulate that so this test isolates the edge gate itself.
        // `vertex / 100` is already world; do not feed raw-local here.
        let resolved: Vec<Option<[f32; 3]>> = materialized
            .vertices
            .iter()
            .map(|v| {
                [
                    v[0] as f32 / 100.0,
                    v[1] as f32 / 100.0,
                    v[2] as f32 / 100.0,
                ]
            })
            .map(Some)
            .collect();
        let widths: Vec<(u16, u16)> = model
            .edges
            .iter()
            .map(|e| match e.style {
                ModelEdgeStyle::Sprite { sprite_id, .. } => session
                    .cache
                    .global_sprite(sprite_id)
                    .map(|(_, entry)| (entry.flags as u16, (entry.flags >> 16) as u16))
                    .unwrap_or((0, 0)),
                ModelEdgeStyle::Palette { .. } => (0, 0),
            })
            .collect();
        let materials: Vec<FaceMaterial> = vec![fallback_material(); model.edges.len()];

        let mut drops = (0usize, 0usize, 0usize, 0usize); // unresolved, endpoint, behind-eye, clip
        let mut quads: Vec<EdgeQuadSubmission> = Vec::new();
        for (index, edge) in model.edges.iter().enumerate() {
            let widths = widths[index];
            if widths == (0, 0) {
                drops.0 += 1;
                continue;
            }
            let ModelEdgeStyle::Sprite { size, .. } = edge.style else {
                continue;
            };
            let (Some(a), Some(b)) = (
                resolved.get(edge.vertices[0] as usize).copied().flatten(),
                resolved.get(edge.vertices[1] as usize).copied().flatten(),
            ) else {
                drops.1 += 1;
                continue;
            };
            let (Some(pa), Some(pb)) = (
                intr.project(a, cam, basis, SceneProjectionAuthority::default()),
                intr.project(b, cam, basis, SceneProjectionAuthority::default()),
            ) else {
                drops.2 += 1;
                continue;
            };
            match crate_gate(
                (pa.0, pa.1),
                (pb.0, pb.1),
                pa.2,
                pb.2,
                &intr,
                widths,
                i32::from(size),
            ) {
                Some(screen_corners) => {
                    quads.push(EdgeQuadSubmission {
                        screen_corners,
                        endpoint_depths_raw: [pa.2, pb.2],
                        mid_depth_raw: (pa.2 + pb.2) / 2,
                        material: fallback_material(),
                    });
                }
                None => drops.3 += 1,
            }
        }
        // Cross-check the production selector agrees with the staged count.
        let (via_selector, stats) = select_edge_quads_with_stats(
            &model.edges,
            &widths,
            &materials,
            &resolved,
            &intr,
            cam,
            basis,
        );
        assert_eq!(
            via_selector.quads, quads,
            "selector must keep FUN_00459000 screen corners, not unprojected world"
        );
        assert_eq!(stats.lines_submitted, via_selector.lines.len());
        let sprite_count = widths.iter().filter(|w| **w != (0, 0)).count();
        let palette_count = model
            .edges
            .iter()
            .filter(|edge| matches!(edge.style, ModelEdgeStyle::Palette { .. }))
            .count();
        println!(
            "{name}: edges={} sprite_widths={sprite_count} palette={palette_count} quads={} lines={} drops(unres,endp,eye,clip)={:?} {stats:?}",
            model.edges.len(),
            quads.len(),
            via_selector.lines.len(),
            drops
        );
        // Print the width pairs seen per sprite id for the record.
        if name == "spider" {
            for e in &model.edges {
                if let ModelEdgeStyle::Sprite { sprite_id, .. } = e.style {
                    if let Some((_, entry)) = session.cache.global_sprite(sprite_id) {
                        println!(
                            "  sprite {sprite_id} widths=({},{})",
                            entry.flags as u16,
                            (entry.flags >> 16) as u16
                        );
                    }
                }
            }
        }
        assert!(
            quads.len() >= 6,
            "{name} must submit its leg cluster through the recovered gate"
        );
        assert!(
            via_selector.lines.len() >= 1,
            "{name} must submit authored 0x02 palette hairlines, got {palette_count} palette records"
        );
    }
}

#[v2k_test_support::retail_test]
fn newant_palette_hairlines_are_hip_to_selector_or_paired_selectors() {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(13, 1).unwrap();
    let model = session
        .cache
        .global_model_ids()
        .into_iter()
        .find_map(|id| {
            session
                .cache
                .global_model(id)
                .filter(|m| m.name.as_deref() == Some("newant"))
        })
        .expect("newant");
    let materialized = model.materialize(&AnimVars::default());
    let mut hip_foot = 0usize;
    let mut paired_selectors = 0usize;
    let mut other = 0usize;
    for edge in model
        .edges
        .iter()
        .filter(|edge| matches!(edge.style, ModelEdgeStyle::Palette { .. }))
    {
        let t0 = materialized.vertex_type_flags[edge.vertices[0] as usize];
        let t1 = materialized.vertex_type_flags[edge.vertices[1] as usize];
        match (t0, t1) {
            (0, 14) | (14, 0) => hip_foot += 1,
            (14, 14) => paired_selectors += 1,
            _ => other += 1,
        }
        let v0 = materialized.vertices[edge.vertices[0] as usize];
        let v1 = materialized.vertices[edge.vertices[1] as usize];
        let dist = ((v0[0] - v1[0]).powi(2) + (v0[1] - v1[1]).powi(2) + (v0[2] - v1[2]).powi(2))
            .sqrt()
            / 256.0;
        assert!(
            dist < 1.0,
            "authored 0x02 span {dist} must stay insect-scale; live cables are a type-14 draw-space bug"
        );
    }
    assert_eq!(hip_foot, 6, "six hip-to-secondary 0x02 hairlines");
    assert_eq!(
        paired_selectors, 6,
        "six primary-to-secondary 0x02 hairlines"
    );
    assert_eq!(other, 2, "two antenna 0x02 hairlines");
}

#[v2k_test_support::retail_test]
fn targetter_terrain_crosshair_has_no_type13_vertices() {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    let model = session
        .cache
        .global_model(TARGETTER_TERRAIN_CROSSHAIR_MODEL_ID)
        .expect("global crosshair1");
    assert_eq!(model.name.as_deref(), Some("crosshair1"));
    assert!(
        model.vertex_type_flags.iter().all(|&flag| flag != 13),
        "terrain crosshair must opt into ModelOverlayKind, not invented type-13 vertices"
    );
}
