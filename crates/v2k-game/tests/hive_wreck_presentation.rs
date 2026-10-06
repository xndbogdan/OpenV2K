use v2k_game::{
    hive_wreck_presentation::{HiveWreckPresentationRequest, HIVE_WRECK_RING_MODEL_ID},
    session::GameSession,
};

#[v2k_test_support::retail_test]
fn retail_ring_resource_consumes_its_private_shrink_word() {
    let data_dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&data_dir).expect("init session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load high-resolution system resources");
    session.load_level_by_id(13, 1).expect("load Peasant World");
    let model = session
        .cache
        .global_model(HIVE_WRECK_RING_MODEL_ID)
        .expect("LAB_0041CB10 global model 243");
    let terrain = session.cache.terrain().expect("Peasant terrain");
    let input = HiveWreckPresentationRequest {
        controller_state: 0,
        marker_present: true,
        suction_timer_us: 6_000_000,
        anchor_raw: [-17_536, 0, -32_384],
    };
    let wide_ring = input.ring_submissions(terrain)[0];
    let narrow_ring = HiveWreckPresentationRequest {
        suction_timer_us: 3_000_000,
        ..input
    }
    .ring_submissions(terrain)[0];
    assert_eq!(wide_ring.shrink_output_raw, 0);
    assert_eq!(narrow_ring.shrink_output_raw, 65_471);
    let wide = model.materialize(&wide_ring.anim_vars(500));
    let narrow = model.materialize(&narrow_ring.anim_vars(500));
    assert!(
        !wide.vertices.is_empty(),
        "ring resource contains authored geometry"
    );
    assert_eq!(wide.vertices.len(), narrow.vertices.len());
    assert_ne!(
        wide.vertices, narrow.vertices,
        "the private callback word must change the ring geometry"
    );
    let radius = |vertices: &[[f64; 3]]| {
        vertices
            .iter()
            .map(|v| (v[0] * v[0] + v[2] * v[2]).sqrt())
            .fold(0.0_f64, f64::max)
    };
    assert!(
        radius(&narrow.vertices) < radius(&wide.vertices),
        "the recovered output draws the ring inward"
    );
}
