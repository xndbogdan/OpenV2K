//! Real-data guard for the spider's paired Sub-H external-frame selectors.

use v2k_game::session::GameSession;
use v2k_game::sub_h_external_frame::{selector_binding, SubHEndpoint};

#[v2k_test_support::retail_test]
fn spider_legs_are_sprite_ribbons_not_faces() {
    let data_dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&data_dir).expect("initialize retail-data session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-resolution common pool");
    let spider = session
        .cache
        .global_model(256)
        .expect("global spider model");
    let type14_faces = spider
        .triangles
        .iter()
        .filter(|tri| {
            tri.iter()
                .any(|&index| spider.vertex_type_flags[index as usize] == 14)
        })
        .count();
    assert_eq!(type14_faces, 0, "spider legs are not triangle faces");
    let sprite_985 = spider
        .edges
        .iter()
        .filter(|edge| {
            matches!(
                edge.style,
                v2k_formats::models::ModelEdgeStyle::Sprite { sprite_id: 985, .. }
            )
        })
        .count();
    let sprite_986 = spider
        .edges
        .iter()
        .filter(|edge| {
            matches!(
                edge.style,
                v2k_formats::models::ModelEdgeStyle::Sprite { sprite_id: 986, .. }
            )
        })
        .count();
    assert_eq!(sprite_985, 16, "eight legs × two 0x22 sprite-985 ribbons");
    assert_eq!(sprite_986, 8, "eight 0x22 sprite-986 support ribbons");
}

#[v2k_test_support::retail_test]
fn spider_authors_eight_sub_h_records_and_all_sixteen_paired_selectors() {
    let data_dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&data_dir).expect("initialize retail-data session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-resolution common pool");

    let spider_type = session
        .cache
        .global_entity_type(17)
        .expect("global entity type 17");
    assert!(
        spider_type.model_ids.contains(&256),
        "type 17 no longer references global spider model 256"
    );
    let descriptor = spider_type
        .sub_h_external_frame_descriptor()
        .expect("spider Sub-H external-frame descriptor");
    assert_eq!(descriptor.records.len(), 8);
    assert_eq!(descriptor.completion_sound_id, None);
    assert!(descriptor.records.iter().all(|record| {
        record.resolver_flags_raw == 0x4000_0000
            && record.phase_rate_raw == 0x0c00_0000
            && record.axis_mode_raw == 0
    }));
    assert_eq!(
        descriptor
            .records
            .iter()
            .map(|record| record.vertex_refs)
            .collect::<Vec<_>>(),
        [
            [0, 56, 64],
            [1, 57, 65],
            [52, 58, 66],
            [53, 59, 67],
            [54, 60, 68],
            [55, 61, 69],
            [150, 62, 70],
            [151, 63, 71],
        ]
    );
    assert_eq!(
        descriptor
            .records
            .iter()
            .map(|record| record.dependencies)
            .collect::<Vec<_>>(),
        [
            [1, 2, 3, 3],
            [0, 2, 3, 3],
            [0, 1, 3, 3],
            [0, 1, 2, 2],
            [5, 6, 7, 7],
            [4, 6, 7, 7],
            [4, 5, 7, 7],
            [4, 5, 6, 6],
        ]
    );

    let spider = session
        .cache
        .global_model(256)
        .expect("global spider model");
    assert_eq!(spider.name.as_deref(), Some("spider"));
    assert_eq!(spider.edges.len(), 40);
    assert!(
        spider.edges.iter().any(|edge| matches!(
            edge.style,
            v2k_formats::models::ModelEdgeStyle::Sprite { sprite_id: 985, .. }
        )),
        "spider legs use opcode 0x22 sprite 985"
    );
    assert_eq!(spider.slot_count, 154);
    assert_eq!(spider.records.len(), 77);
    for record in &descriptor.records {
        for &slot in &record.vertex_refs {
            let local = v2k_game::sub_h_external_frame::type0_slot_model_raw(&spider.records, slot)
                .expect("spider Sub-H refs are type-0 slots");
            if slot & 1 == 0 {
                let odd =
                    v2k_game::sub_h_external_frame::type0_slot_model_raw(&spider.records, slot | 1)
                        .expect("odd pair");
                assert_eq!(odd[0], local[0].wrapping_neg());
                assert_eq!(odd[1], local[1]);
                assert_eq!(odd[2], local[2]);
            }
        }
    }
    let runtime = v2k_game::sub_h_external_frame::SubHRuntimeState::new(8).unwrap();
    let points = v2k_game::sub_h_external_frame::resolve_selector_world_points(
        &runtime,
        &descriptor,
        &spider.records,
        [0, 0, 0],
        [0.0, 0.0, 0.0],
        None,
        |_, _| 0,
    )
    .expect("type-0 Sub-H geometry");
    let filled: Vec<_> = points.iter().flatten().collect();
    assert_eq!(filled.len(), 16);
    // Slot 64 is Endpoint C for selector 0: authored `[150, -75, 240]` is
    // already 8.8, so world is `/256`, not the menu `*256/100` conversion.
    let foot = points[0].expect("selector 0");
    assert!((foot[0] - 150.0 / 256.0).abs() < 1.0e-4);
    assert!((foot[1] + 75.0 / 256.0).abs() < 1.0e-4);
    assert!((foot[2] - 240.0 / 256.0).abs() < 1.0e-4);
    let wrapped_origin = [((200.0_f32 * 256.0).round() as i32) as i16, 0, 0];
    let wrapped = v2k_game::sub_h_external_frame::resolve_selector_world_points(
        &runtime,
        &descriptor,
        &spider.records,
        wrapped_origin,
        [200.0, 0.0, 0.0],
        None,
        |_, _| 0,
    )
    .expect("type-0 Sub-H geometry at wrapped origin");
    let wrapped_foot = wrapped[0].expect("selector 0");
    assert!((wrapped_foot[0] - (200.0 + 150.0 / 256.0)).abs() < 1.0e-4);
    assert!(wrapped_foot[0] > 199.0);
    assert!(
        spider.edges.iter().any(|edge| {
            spider.vertex_type_flags.get(edge.vertices[0] as usize) == Some(&13)
                && spider.vertex_type_flags.get(edge.vertices[1] as usize) == Some(&13)
        }),
        "spider authors type-13 support edges that bake type-14 selectors"
    );
    let mut selectors: Vec<i32> = spider
        .records
        .iter()
        .filter(|record| record[0] == 14)
        .map(|record| i32::from(record[1]))
        .collect();
    selectors.sort_unstable();
    assert_eq!(selectors, (0..16).collect::<Vec<_>>());

    for selector in 0..16 {
        let binding = selector_binding(selector, descriptor.records.len())
            .expect("authored spider selector must resolve");
        assert_eq!(binding.record_index, usize::try_from(selector % 8).unwrap());
        assert_eq!(
            binding.endpoint,
            if selector < 8 {
                SubHEndpoint::Primary
            } else {
                SubHEndpoint::Secondary
            }
        );
    }
}

#[v2k_test_support::retail_test]
fn newant_authors_six_sub_h_records_and_all_twelve_paired_selectors() {
    let data_dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&data_dir).expect("initialize retail-data session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-resolution common pool");

    let newant_type = session
        .cache
        .global_entity_type(47)
        .expect("global entity type 47");
    assert!(
        newant_type.model_ids.contains(&302),
        "type 47 no longer references global newant model 302"
    );
    let descriptor = newant_type
        .sub_h_external_frame_descriptor()
        .expect("newant Sub-H external-frame descriptor");
    assert_eq!(descriptor.records.len(), 6);
    assert_eq!(descriptor.completion_sound_id, None);

    let newant = session
        .cache
        .global_model(302)
        .expect("global newant model");
    assert_eq!(newant.name.as_deref(), Some("newant"));

    let runtime = v2k_game::sub_h_external_frame::SubHRuntimeState::new(6).unwrap();
    let points = v2k_game::sub_h_external_frame::resolve_selector_world_points(
        &runtime,
        &descriptor,
        &newant.records,
        [0, 0, 0],
        [0.0, 0.0, 0.0],
        None,
        |_, _| 0,
    )
    .expect("type-0 Sub-H geometry for newant");
    let filled: Vec<_> = points.iter().flatten().collect();
    assert_eq!(
        filled.len(),
        12,
        "six legs × two endpoints = 12 filled points"
    );

    let mut selectors: Vec<i32> = newant
        .records
        .iter()
        .filter(|record| record[0] == 14)
        .map(|record| i32::from(record[1]))
        .collect();
    selectors.sort_unstable();
    assert_eq!(
        selectors,
        (0..=12).collect::<Vec<_>>(),
        "newant authors 12 leg selectors 0..11 plus auxiliary selector 12"
    );

    for selector in 0..12 {
        let binding = selector_binding(selector, descriptor.records.len())
            .expect("authored newant leg selector must resolve");
        assert_eq!(binding.record_index, usize::try_from(selector % 6).unwrap());
        assert_eq!(
            binding.endpoint,
            if selector < 6 {
                SubHEndpoint::Primary
            } else {
                SubHEndpoint::Secondary
            }
        );
    }
    assert_eq!(
        selector_binding(12, descriptor.records.len()),
        None,
        "selector 12 is outside the 6-record paired range"
    );
}

#[v2k_test_support::retail_test]
fn stag_authors_six_sub_h_records_and_all_twelve_paired_selectors() {
    let data_dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&data_dir).expect("initialize retail-data session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-resolution common pool");

    let stag_type = session
        .cache
        .global_entity_type(26)
        .expect("global entity type 26");
    assert_eq!(stag_type.model_ids, [267; 4]);
    let descriptor = stag_type
        .sub_h_external_frame_descriptor()
        .expect("stag Sub-H external-frame descriptor");
    assert_eq!(descriptor.records.len(), 6);
    assert_eq!(descriptor.completion_sound_id, None);

    let stag = session.cache.global_model(267).expect("global stag model");
    assert_eq!(stag.name.as_deref(), Some("stag"));

    let runtime = v2k_game::sub_h_external_frame::SubHRuntimeState::new(6).unwrap();
    let points = v2k_game::sub_h_external_frame::resolve_selector_world_points(
        &runtime,
        &descriptor,
        &stag.records,
        [0, 0, 0],
        [0.0, 0.0, 0.0],
        None,
        |_, _| 0,
    )
    .expect("type-0 Sub-H geometry for stag");
    let filled: Vec<_> = points.iter().flatten().collect();
    assert_eq!(
        filled.len(),
        12,
        "six legs × two endpoints = 12 filled points"
    );

    let mut selectors: Vec<i32> = stag
        .records
        .iter()
        .filter(|record| record[0] == 14)
        .map(|record| i32::from(record[1]))
        .collect();
    selectors.sort_unstable();
    assert_eq!(
        selectors,
        (0..12).collect::<Vec<_>>(),
        "stag should author selectors 0..12"
    );

    for selector in 0..12 {
        let binding = selector_binding(selector, descriptor.records.len())
            .expect("authored stag selector must resolve");
        assert_eq!(binding.record_index, usize::try_from(selector % 6).unwrap());
        assert_eq!(
            binding.endpoint,
            if selector < 6 {
                SubHEndpoint::Primary
            } else {
                SubHEndpoint::Secondary
            }
        );
    }
}
