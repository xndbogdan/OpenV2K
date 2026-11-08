//! Tf3 (`FUN_0046DF70`, `FUN_0046E020`, `FUN_0046E0E0`): one process word
//! per VIEW axis, once a node draw, added to the source slot's VIEW point.

use super::*;
use std::cell::Cell;

fn process_word(state: &Cell<u32>) -> u16 {
    let mut word = state.get();
    let sample = crate::retail_rng::retail_random_u16(&mut word);
    state.set(word);
    sample
}

fn menu_camera() -> Camera {
    let mut camera = Camera::new(4.0 / 3.0);
    camera.position = [0.0; 3];
    camera.yaw = 0.0;
    camera.pitch = 0.0;
    camera
}

#[test]
fn jitter_draws_x_y_z_once_and_offsets_view_and_local_points() {
    let state = Cell::new(0x1234_5678);
    let next = || process_word(&state);
    let camera = menu_camera();
    // A quarter turn about Y: local +X faces world -Z, the camera's depth.
    let transform = ModelTransform {
        orientation: [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [-1.0, 0.0, 0.0]],
        position: [1.0, 2.0, -30.0],
        scale: 1.0,
    };
    let owner = ModelJitterOwner::new(
        5,
        Some(ModelTreeProcessRandom::new(&next, &camera)),
        transform,
    );
    let mut source = ResolvedModelSlot::clear([10.0, 20.0, 30.0]);
    source.native_view_point = Some([100, 200, 300]);
    let request = |command_visit| ModelVertexJitter {
        slot: 40,
        shift_operand: 0x011A,
        source,
        command_visit,
    };

    let mut expected_state = state.get();
    let offset: [i32; 3] = std::array::from_fn(|_| {
        v2k_formats::models::retail_tf3_offset(
            crate::retail_rng::retail_random_u16(&mut expected_state),
            26,
        )
    });
    let jittered = owner.resolve(request(true)).unwrap();
    assert_eq!(state.get(), expected_state, "three words, X then Y then Z");
    assert_eq!(
        jittered.native_view_point,
        Some([100 + offset[0], 200 + offset[1], 300 + offset[2]])
    );
    // VIEW X is world +X = local +Z; VIEW Y is up; VIEW depth is world -Z =
    // local +X.
    let local = [
        f64::from(offset[2]),
        f64::from(offset[1]),
        f64::from(offset[0]),
    ];
    for axis in 0..3 {
        let moved = jittered.position_raw[axis] - source.position_raw[axis];
        assert!((moved - local[axis]).abs() < 1e-3, "{axis}: {moved}");
    }

    // The slot's cache keeps the words; the geometry pass reads it.
    assert_eq!(owner.resolve(request(true)), Some(jittered));
    assert_eq!(owner.resolve(request(false)), Some(jittered));
    assert_eq!(state.get(), expected_state);
}

#[test]
fn jitter_never_draws_after_the_walk_or_without_the_generator() {
    let state = Cell::new(7);
    let next = || process_word(&state);
    let camera = menu_camera();
    let transform = ModelTransform {
        orientation: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        position: [0.0; 3],
        scale: 1.0,
    };
    let request = |command_visit| ModelVertexJitter {
        slot: 2,
        shift_operand: 25,
        source: ResolvedModelSlot::clear([1.0, 2.0, 3.0]),
        command_visit,
    };
    let lent = ModelJitterOwner::new(
        1,
        Some(ModelTreeProcessRandom::new(&next, &camera)),
        transform,
    );
    assert_eq!(lent.resolve(request(false)), None);
    assert_eq!(state.get(), 7);
    let unlent = ModelJitterOwner::new(1, None, transform);
    assert_eq!(unlent.resolve(request(true)), None);
}

/// Multipc's slot 40 (record `[3, 26, 0, 38]`) is the frontend's only tf3
/// vertex: three `0x22` sparks share it, and every frontend draw takes three
/// words (the lab's `0046E076/91/A8` or `0046E136/51/68` triple a pass).
#[v2k_test_support::retail_test]
fn multipc_sparks_take_three_process_words_a_draw() {
    let dir = v2k_test_support::retail_dir();
    let session = crate::session::GameSession::init(&dir).unwrap();
    let colors = ModelMaterialCache::new();
    let model_id = 323;
    let model = session.cache.global_model(model_id).unwrap();
    assert_eq!(model.name.as_deref(), Some("multipc"));
    assert_eq!(model.records[20], [3, 26, 0, 38]);
    let camera = menu_camera();
    let orientation = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let position = [0.0, -3.2, -30.0];
    let state = Cell::new(0x9CC1_EC61);
    let next = || process_word(&state);
    let draw = |random: Option<ModelTreeProcessRandom<'_>>| {
        let mut renderer = RecordingRenderer::default();
        let mut tree = ModelTreeRenderer::new(
            &mut renderer,
            &session.cache,
            &colors,
            1.0,
            Some(ModelSceneLight::NEUTRAL),
            ViewPinMode::Disabled,
        )
        .with_view((&camera).into())
        .with_near_clip(ModelNearClip::RetailFrontend);
        if let Some(random) = random {
            tree = tree.with_process_random(random);
        }
        tree.draw_linked(
            model_id,
            orientation,
            position,
            2,
            None,
            &AnimVars::default(),
        );
        drop(tree);
        let body = renderer
            .bodies
            .into_iter()
            .find(|body| !body.edges.is_empty())
            .expect("multipc submits its sparks");
        let shared = body.edges[0].vertices[0];
        assert_eq!(body.vertex_type_flags[shared as usize], 3);
        assert!(body.edges.iter().all(|edge| edge.vertices[0] == shared));
        assert_eq!(body.edges.len(), 3);
        body.vertices[shared as usize]
    };

    let still = draw(None);
    let mut expected_state = state.get();
    let words: [u16; 3] =
        std::array::from_fn(|_| crate::retail_rng::retail_random_u16(&mut expected_state));
    let jittered = draw(Some(ModelTreeProcessRandom::new(&next, &camera)));
    assert_eq!(state.get(), expected_state, "one slot, three words");
    let offset = words.map(|word| f64::from(v2k_formats::models::retail_tf3_offset(word, 26)));
    // Identity body under the menu camera: VIEW depth is local -Z.
    let moved = [
        jittered[0] - still[0],
        jittered[1] - still[1],
        jittered[2] - still[2],
    ];
    for (axis, expected) in [offset[0], offset[1], -offset[2]].into_iter().enumerate() {
        assert!((moved[axis] - expected).abs() < 1e-3, "{axis}: {moved:?}");
    }
    assert!(offset.iter().all(|value| (-32.0..32.0).contains(value)));
}
