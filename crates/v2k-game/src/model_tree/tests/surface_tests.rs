//! World callbacks must resolve before the real dragon's linked wing quads.

use super::*;
use v2k_formats::models::{ModelSlotClip, ModelSurfaceOrigin};

fn dragon_wing_session() -> Option<crate::session::GameSession> {
    let dir = v2k_test_support::retail_dir();
    let mut session = crate::session::GameSession::init(&dir).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(50, 1).unwrap();
    let vars = AnimVars::default();
    let first = session.cache.global_model(352).unwrap();
    assert_eq!(first.name.as_deref(), Some("gwing1a"));
    let first_geometry = first.materialize(&vars);
    assert_eq!(first_geometry.instances.len(), 1);
    assert_eq!(first_geometry.instances[0].model_id, 353);
    assert_eq!(first_geometry.instances[0].linked_slots, [2, 4, 10, 12]);
    assert_eq!(first.records[5][0], 13);
    assert_eq!(first.records[6][0], 13);
    assert_eq!(
        session.cache.global_model(353).unwrap().name.as_deref(),
        Some("gwing2a")
    );
    for model_id in [353, 354] {
        let model = session.cache.global_model(model_id).unwrap();
        // The original sprite1767 polygon imports the two parent endpoints
        // through tf11, interleaved with two local tf13 endpoints.
        assert!(
            model.cmd_words.windows(7).any(|words| {
                words[0] == 0x84 && words[1] == 1767 && words[3..7] == [8, 10, 14, 12]
            }),
            "model{model_id} authored wing quad"
        );
        assert_eq!(
            [8_usize, 10, 14, 12].map(|slot| model.records[slot >> 1][0]),
            [11, 13, 13, 11]
        );
    }
    let middle = session.cache.global_model(353).unwrap().materialize(&vars);
    assert_eq!(middle.instances.len(), 1);
    assert_eq!(middle.instances[0].model_id, 354);
    Some(session)
}

fn flat_wing_terrain(session: &mut crate::session::GameSession, height: i8, sea: i16) {
    let terrain = session.cache.level_terrain_mut().unwrap();
    terrain.header = [i32::from(sea) << 8, -73, 73, -73, 0];
    for cell in &mut terrain.cells {
        cell.height = height as u8;
    }
}

fn record_wings(
    cache: &ResourceCache,
    transform: ModelTransform,
    world: bool,
    cached: bool,
) -> RecordingRenderer {
    let colors = ModelMaterialCache::new();
    let mut renderer = RecordingRenderer::default();
    let mut tree = if world {
        ModelTreeRenderer::new_world(&mut renderer, cache, &colors, transform.scale, None, 77)
    } else {
        ModelTreeRenderer::new(
            &mut renderer,
            cache,
            &colors,
            transform.scale,
            None,
            ViewPinMode::Raw,
        )
    };
    if cached {
        tree.draw_static(352, transform.orientation, transform.position, 2, None);
    } else {
        tree.draw_linked(
            352,
            transform.orientation,
            transform.position,
            2,
            None,
            &AnimVars::default(),
        );
    }
    renderer
}

fn wing_quad(body: &RecordedBody) -> [usize; 4] {
    let ModelFaceVertices::Quad(indices) = body.face_vertices[0] else {
        panic!("the first authored wing face must retain its four corners");
    };
    let indices = indices.map(usize::from);
    assert_eq!(
        indices.map(|index| body.vertex_type_flags[index]),
        [11, 13, 13, 11]
    );
    indices
}

fn submitted_world_point(body: &RecordedBody, index: usize) -> [f64; 3] {
    let raw = body.vertices[index];
    std::array::from_fn(|axis| {
        f64::from(body.transform.position[axis])
            + (0..3)
                .map(|column| f64::from(body.transform.orientation[axis][column]) * raw[column])
                .sum::<f64>()
                * f64::from(body.transform.scale)
                / 100.0
    })
}

#[v2k_test_support::retail_test]
fn real_dragon_linked_wing_quads_resolve_to_world_ground_in_each_node_frame() {
    let Some(mut session) = dragon_wing_session() else {
        return;
    };
    flat_wing_terrain(&mut session, 8, -0x1800);
    let initial_vertices = (352..=354)
        .map(|id| {
            session
                .cache
                .global_model(id)
                .unwrap()
                .materialize(&AnimVars::default())
                .vertices
        })
        .collect::<Vec<_>>();
    // A full axis permutation tilts the local wing plane into world space;
    // its reflected variant independently exercises the opposite local-X side.
    for reflected in [false, true] {
        let orientation = [
            [0., 0., 1.],
            [if reflected { -1. } else { 1. }, 0., 0.],
            [0., 1., 0.],
        ];
        let transform = ModelTransform {
            orientation,
            position: [64., 10., 96.],
            scale: 100. / 256.,
        };
        let intrinsic = record_wings(&session.cache, transform, false, false);
        assert_eq!(intrinsic.bodies.len(), 3);
        for cached in [false, true] {
            let world = record_wings(&session.cache, transform, true, cached);
            assert_eq!(world.bodies.len(), 3);
            for (index, body) in world.bodies.iter().enumerate().skip(1) {
                assert_eq!(
                    body.surface_resolution,
                    ModelSurfaceResolution::ContextResolved
                );
                assert_eq!(body.view_pin, ViewPinMode::WorldSurface);
                assert!(body.has_world_surface);
                for corner in wing_quad(body) {
                    let point = submitted_world_point(body, corner);
                    assert!(
                        (point[1] - 1.).abs() < 1.0e-4,
                        "model{} reflected={reflected} cached={cached}, type{} world={point:?}",
                        352 + index,
                        body.vertex_type_flags[corner]
                    );
                    assert_eq!(body.vertex_clip[corner], ModelSlotClip::Clear);
                    assert_eq!(
                        body.vertex_surface_origin[corner],
                        ModelSurfaceOrigin::ViewPin
                    );
                }
                let raw = &intrinsic.bodies[index];
                assert_eq!(raw.surface_resolution, ModelSurfaceResolution::Intrinsic);
                assert!(wing_quad(raw).iter().all(|&corner| {
                    raw.vertex_surface_origin[corner] == ModelSurfaceOrigin::None
                }));
                assert!(
                    wing_quad(raw).iter().any(|&corner| {
                        (submitted_world_point(raw, corner)[1] - 1.).abs() > 0.25
                    }),
                    "intrinsic wing must remain above the terrain in model{}",
                    352 + index
                );
                assert_eq!(raw.face_vertices, body.face_vertices);
                assert_eq!(raw.face_uvs, body.face_uvs);
            }
        }
    }
    for (id, before) in (352..=354).zip(initial_vertices) {
        assert_eq!(
            session
                .cache
                .global_model(id)
                .unwrap()
                .materialize(&AnimVars::default())
                .vertices,
            before,
            "world context must not mutate intrinsic model{id}"
        );
    }
}

#[v2k_test_support::retail_test]
fn real_dragon_wing_links_keep_surface_band_clip_through_both_descendants() {
    let Some(mut session) = dragon_wing_session() else {
        return;
    };
    flat_wing_terrain(&mut session, -64, 0);
    // Keep every unprojected default-wing endpoint inside sea +/-150 raw
    // units while retaining a translated, rotated and reflected model frame.
    let transform = ModelTransform {
        orientation: [[0., 0., 1.], [0., 1., 0.], [1., 0., 0.]],
        position: [64., 0., 96.],
        scale: 100. / (256. * 64.),
    };
    let intrinsic = record_wings(&session.cache, transform, false, false);
    let world = record_wings(&session.cache, transform, true, false);
    assert_eq!(world.bodies.len(), 3);
    for (index, body) in world.bodies.iter().enumerate().skip(1) {
        assert_eq!(
            body.surface_resolution,
            ModelSurfaceResolution::ContextResolved
        );
        let raw = &intrinsic.bodies[index];
        for corner in wing_quad(body) {
            let point = submitted_world_point(raw, corner);
            assert!(
                point[1].abs() <= 150. / 256.,
                "model{} fixture outside sea band: {point:?}",
                352 + index
            );
            assert_eq!(
                body.vertex_clip[corner],
                ModelSlotClip::SurfaceBand,
                "model{} type{} must preserve rejection across the linked boundary",
                352 + index,
                body.vertex_type_flags[corner]
            );
            assert_eq!(
                body.vertex_surface_origin[corner],
                ModelSurfaceOrigin::ViewPin
            );
            assert_eq!(raw.vertex_clip[corner], ModelSlotClip::Clear);
            assert_eq!(raw.vertex_surface_origin[corner], ModelSurfaceOrigin::None);
            let rejected_point = submitted_world_point(body, corner);
            assert!(
                (0..3).all(|axis| (rejected_point[axis] - point[axis]).abs() < 1.0e-5),
                "sea-band rejection keeps source coordinates: {rejected_point:?} versus {point:?}"
            );
        }
    }
}
