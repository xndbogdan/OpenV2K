//! Authored insect shadow dependencies share the live Sub-H endpoint owner.
use super::*;
use crate::sub_h_external_frame::{SubHPresentation, SubHRuntimeState};
use v2k_formats::models::{ModelEdgeStyle, ModelVertexProjection};

fn retail_shadow_session() -> crate::session::GameSession {
    let data = v2k_test_support::retail_dir();
    assert!(
        data.join("PRELOAD.DAT").exists(),
        "retail shadow corpus is required"
    );
    let mut session = crate::session::GameSession::init(&data).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(13, 1).unwrap();
    let terrain = session.cache.level_terrain_mut().unwrap();
    terrain.header = [-0x1800 << 8, 0, 73, 0, 0];
    for cell in &mut terrain.cells {
        cell.height = 0;
    }
    session
}

fn submitted_world(body: &RecordedBody, index: usize) -> [f64; 3] {
    let ModelVertexProjection::WorldPoint(point) = body.vertex_projection[index] else {
        panic!(
            "live callback point must retain world ownership, type{}",
            body.vertex_type_flags[index]
        );
    };
    point
}

#[v2k_test_support::retail_test]
fn retail_newant_spider_and_stag_leg_shadows_follow_their_own_primary_feet() {
    let session = retail_shadow_session();
    let colors = ModelMaterialCache::new();
    // This is a controlled warm-cache consumer regression. The runtime's
    // settled endpoints are deliberately independent from model parameters;
    // authentic construction and all-state rendering are separate live tests.
    for (type_id, model_id, count, size) in [(47, 302, 6, 4), (17, 256, 8, 10), (26, 267, 6, 8)] {
        let model = session.cache.global_model(model_id).unwrap();
        let descriptor = session
            .cache
            .global_entity_type(type_id)
            .unwrap()
            .sub_h_external_frame_descriptor()
            .unwrap();
        assert_eq!(descriptor.records.len(), count);
        for offset in [0_i16, 256] {
            let mut runtime = SubHRuntimeState::new(count).unwrap();
            runtime.set_enabled(true);
            for (index, record) in runtime.records_mut().iter_mut().enumerate() {
                record.flags_raw = 7;
                record.primary_raw = [
                    offset + 128 + index as i16 * 128,
                    384,
                    -128 - index as i16 * 128,
                ];
                record.secondary_raw = [
                    offset + 256 + index as i16 * 128,
                    640,
                    -256 - index as i16 * 128,
                ];
            }
            let before = runtime.clone();
            let mut renderer = RecordingRenderer::default();
            // The draw basis is deliberately non-orthonormal. World custody
            // must survive; transpose-as-inverse storage is not the endpoint.
            let orientation = [[0.98, 0.02, 0.0], [0.0, 0.97, 0.01], [0.03, 0.0, 0.99]];
            ModelTreeRenderer::new_world(
                &mut renderer,
                &session.cache,
                &colors,
                100.0 / 256.0,
                None,
                0,
            )
            .with_view(ModelTreeView {
                position: [0.0, 4.0, 8.0],
                forward: [0.0, 0.0, -1.0],
            })
            .with_sub_h_presentation(SubHPresentation {
                runtime: &mut runtime,
                descriptor: &descriptor,
                model_records: &model.records,
                retail_tick: 0,
                origin_raw: [0, 512, 0],
                origin_world: [0.0, 2.0, 0.0],
                body_axes_q31: Some([[0x7FFF0000, 0, 0], [0, 0x7FFF0000, 0], [0, 0, 0x7FFF0000]]),
                native_context: None,
                fallback: None,
                emitter: None,
            })
            .draw_linked(
                model_id,
                orientation,
                [0.0, 2.0, 0.0],
                2,
                None,
                &AnimVars::default(),
            );
            let body = &renderer.bodies[0];
            let shadows = body
                .edges
                .iter()
                .filter(|edge| {
                    edge.style
                        == ModelEdgeStyle::Sprite {
                            sprite_id: 986,
                            size,
                        }
                })
                .collect::<Vec<_>>();
            assert_eq!(
                shadows.len(),
                count,
                "type{type_id} authored leg shadow count/size"
            );
            let mut covered = std::collections::BTreeSet::new();
            for edge in shadows {
                let endpoint = edge.vertices[1] as usize;
                assert_eq!(body.vertex_type_flags[endpoint], 13);
                let shadow = submitted_world(body, endpoint);
                let record = before
                    .records()
                    .iter()
                    .position(|record| {
                        shadow[0] == f64::from(record.primary_raw[0]) / 256.0
                            && shadow[2] == f64::from(record.primary_raw[2]) / 256.0
                    })
                    .expect("shadow X/Z must come from this actor's primary foot");
                assert_eq!(shadow[1], 0.0, "flat authored terrain projection");
                covered.insert(record);
            }
            assert_eq!(
                covered.len(),
                count,
                "every authored primary foot has its own shadow"
            );
            for (index, kind) in body.vertex_type_flags.iter().enumerate() {
                if *kind != 14 {
                    continue;
                }
                let point = submitted_world(body, index);
                assert!(
                    before.records().iter().any(|record| [
                        record.primary_raw,
                        record.secondary_raw
                    ]
                    .into_iter()
                    .any(|raw| point == raw.map(|word| f64::from(word) / 256.0))),
                    "direct leg retains native world endpoint"
                );
            }
            assert_eq!(
                runtime, before,
                "warm callback reads must not alter phase caches"
            );
        }
    }
}

#[test]
fn recursive_live_shadow_warms_only_the_submitted_limb_and_backfaces_do_not_resolve() {
    use v2k_formats::collision::{SubHExternalFrameDescriptor, SubHExternalFrameRecord};
    let model = model_entry(
        0,
        "lazy_shadow",
        vec![
            [14, 0, 0, 0],
            [13, 0, 0, 0],
            [13, 2, 0, 0],
            [14, 1, 0, 0],
            [0, 0, 0, 0],
            [0, 0, 500, 0],
            [0, 100, 0, 0],
        ],
        vec![0x22, 986, 4, 4, 8, 0x03, 32, 2, 6, 8, 12, 0],
    );
    let mut model = model;
    model.normal_pool = vec![[8, 0, 0, 32767]];
    let cache = synthetic_cache_with_terrain(
        vec![model],
        Some(TerrainGrid {
            header: [-0x1800 << 8, 0, 73, 0, 0],
            cells: vec![
                TerrainCell {
                    height: 0,
                    attribute: 0,
                    terrain_type: 0
                };
                GRID_SIZE * GRID_SIZE
            ],
        }),
    );
    let descriptor = SubHExternalFrameDescriptor {
        completion_sound_id: None,
        records: vec![
            SubHExternalFrameRecord {
                resolver_flags_raw: 0,
                phase_rate_raw: 0,
                vertex_refs: [8, 10, 12],
                axis_mode_raw: 0,
                dependencies: [0; 4]
            };
            2
        ],
    };
    let mut runtime = SubHRuntimeState::new(2).unwrap();
    runtime.set_enabled(false);
    let colors = ModelMaterialCache::new();
    let mut renderer = RecordingRenderer::default();
    ModelTreeRenderer::new_world(&mut renderer, &cache, &colors, 100.0 / 256.0, None, 0)
        .with_view(ModelTreeView {
            position: [0.0, 0.0, -8.0],
            forward: [0.0, 0.0, 1.0],
        })
        .with_sub_h_presentation(SubHPresentation {
            runtime: &mut runtime,
            descriptor: &descriptor,
            model_records: &cache.global_model(0).unwrap().records,
            retail_tick: 0,
            origin_raw: [0; 3],
            origin_world: [0.0; 3],
            body_axes_q31: None,
            native_context: None,
            fallback: None,
            emitter: None,
        })
        .draw_linked(
            0,
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            [0.0; 3],
            0,
            None,
            &AnimVars::default(),
        );
    assert_eq!(
        runtime.records()[0].flags_raw,
        3,
        "referenced tf13 chain executes D360 primary"
    );
    assert_eq!(
        runtime.records()[1],
        Default::default(),
        "normal-rejected limb is untouched"
    );
    assert_eq!(renderer.bodies[0].edges.len(), 1);
    assert!(renderer.bodies[0].triangles.is_empty());
}

#[test]
fn native_live_normal_gate_uses_wrapping_raw_plane_and_strict_sign() {
    let callback = |_| panic!("a normal gate cannot invoke a vertex callback");
    let mut resolver = WorldModelVertexResolver {
        surface: None,
        native_context: None,
        native_edges_required: false,
        model: &ModelEntry::default(),
        transform: ModelTransform {
            orientation: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            position: [0.0; 3],
            scale: 100.0 / 256.0,
        },
        // This float camera deliberately contradicts the native owner.
        view: Some(ModelTreeView {
            position: [100.0, 100.0, 100.0],
            forward: [0.0, 0.0, -1.0],
        }),
        native_view: Some(ModelViewSelection::Retail {
            local_origin_from_camera_raw: [0; 3],
        }),
        external_owner: Some(&callback),
        external_points: RefCell::new(HashMap::new()),
        native_vertex_failures: RefCell::new(Vec::new()),
    };
    assert!(
        !resolver.admit_face_raw([0; 3], [1, 0, 0]),
        "native equality rejects"
    );
    resolver.native_view = Some(ModelViewSelection::Retail {
        local_origin_from_camera_raw: [-1, 0, 0],
    });
    assert!(resolver.admit_face_raw([0; 3], [1, 0, 0]));
    resolver.native_view = Some(ModelViewSelection::Retail {
        local_origin_from_camera_raw: [i32::MAX, 0, 0],
    });
    assert!(
        resolver.admit_face_raw([0; 3], [2, 0, 0]),
        "source IMUL narrows before sign test"
    );
    assert!(
        resolver.admit_face_raw([1, 0, 0], [1, 0, 0]),
        "source origin+anchor ADD wraps before multiply"
    );
    assert!(resolver.external_points.borrow().is_empty());
}

#[test]
fn queued_live_callbacks_execute_before_flush_and_are_never_revisited() {
    use v2k_formats::collision::{SubHExternalFrameDescriptor, SubHExternalFrameRecord};
    let model = model_entry(
        0,
        "queued_live_shadow",
        vec![
            [14, 0, 0, 0],
            [13, 0, 0, 0],
            [0, 0, 0, 0],
            [0, 0, 500, 0],
            [0, 100, 0, 0],
        ],
        vec![0x22, 986, 4, 2, 4, 0],
    );
    let cache = synthetic_cache_with_terrain(
        vec![model],
        Some(TerrainGrid {
            header: [-0x1800 << 8, 0, 73, 0, 0],
            cells: vec![
                TerrainCell {
                    height: 0,
                    attribute: 0,
                    terrain_type: 0
                };
                GRID_SIZE * GRID_SIZE
            ],
        }),
    );
    let descriptor = SubHExternalFrameDescriptor {
        completion_sound_id: None,
        records: vec![SubHExternalFrameRecord {
            resolver_flags_raw: 0,
            phase_rate_raw: 0,
            vertex_refs: [4, 6, 8],
            axis_mode_raw: 0,
            dependencies: [0; 4],
        }],
    };
    let mut runtime = SubHRuntimeState::new(1).unwrap();
    runtime.set_enabled(false);
    let colors = ModelMaterialCache::new();
    let mut renderer = RecordingRenderer::default();
    let mut buffer = ModelTreeSubmissionBuffer::default();
    ModelTreeRenderer::new_world(&mut renderer, &cache, &colors, 100.0 / 256.0, None, 0)
        .with_submission_buffer(&mut buffer)
        .with_view(ModelTreeView {
            position: [0.0, 0.0, -8.0],
            forward: [0.0, 0.0, 1.0],
        })
        .with_sub_h_presentation(SubHPresentation {
            runtime: &mut runtime,
            descriptor: &descriptor,
            model_records: &cache.global_model(0).unwrap().records,
            retail_tick: 0,
            origin_raw: [0; 3],
            origin_world: [0.0; 3],
            body_axes_q31: None,
            native_context: None,
            fallback: None,
            emitter: None,
        })
        .draw_linked(
            0,
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            [0.0; 3],
            0,
            None,
            &AnimVars::default(),
        );
    assert!(
        renderer.bodies.is_empty(),
        "producer stage runs callbacks without GL submission"
    );
    assert_eq!(
        runtime.records()[0].flags_raw,
        3,
        "D360 ran at source draw time"
    );
    // A new actor update would invalidate the live cache. Flushing captured
    // commands cannot consult it again or re-run the actor callback.
    runtime.records_mut()[0].flags_raw = 0;
    buffer.flush(
        &mut renderer,
        cache
            .terrain()
            .map(|terrain| WorldSurfaceProjection::new(terrain, 0)),
    );
    assert_eq!(runtime.records()[0].flags_raw, 0);
    assert!(buffer.is_empty());
    assert_eq!(renderer.bodies.len(), 1);
    let endpoint = renderer.bodies[0].edges[0].vertices[0] as usize;
    assert_eq!(
        submitted_world(&renderer.bodies[0], endpoint),
        [100.0 / 256.0, 0.0, 0.0]
    );
}

#[test]
fn native_shadow_consumes_the_cached_view_quantization_without_changing_direct_world_custody() {
    use crate::native_model_frame::{NativeModelFrame, NativeWorldViewport};
    let viewport = NativeWorldViewport {
        origin_raw: [0; 3],
        axes_q31: [[i32::MAX, 0, 0], [0, i32::MAX, 0], [0, 0, i32::MAX]],
        identity: false,
    };
    let model = ModelEntry {
        records: vec![[14, 0, 0, 0], [13, 0, 0, 0]],
        ..ModelEntry::default()
    };
    let terrain = TerrainGrid {
        header: [-0x1800 << 8, 0, 73, 0, 0],
        cells: vec![
            TerrainCell {
                height: 0,
                attribute: 0,
                terrain_type: 0
            };
            GRID_SIZE * GRID_SIZE
        ],
    };
    let callback = |_| {
        Some(external_frame::ActorExternalSlotPoint {
            draw_world: [1.0 / 256.0, 100.0 / 256.0, 1.0 / 256.0],
            native_world_raw: [1, 100, 1],
        })
    };
    let resolver = WorldModelVertexResolver {
        surface: Some(WorldSurfaceProjection::new(&terrain, 0)),
        transform: ModelTransform {
            orientation: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            position: [0.0; 3],
            scale: 100.0 / 256.0,
        },
        view: Some(ModelTreeView {
            position: [0.0; 3],
            forward: [0.0, 0.0, -1.0],
        }),
        native_view: None,
        native_context: Some((
            NativeModelFrame::from_actor(viewport, [0; 3], viewport.axes_q31),
            viewport,
        )),
        native_edges_required: true,
        model: &model,
        external_owner: Some(&callback),
        external_points: RefCell::new(HashMap::new()),
        native_vertex_failures: RefCell::new(Vec::new()),
    };
    let vars = AnimVars::default();
    let context = ModelMaterializationContext {
        vertex_resolver: Some(&resolver),
        ..ModelMaterializationContext::intrinsic(&vars, None)
    };
    let foot = model.resolve_slot_with_context(0, context).unwrap();
    assert_eq!(
        foot.native_view_point,
        Some([0, 99, 0]),
        "6ECF0 separately shifts each positive Q31 product"
    );
    assert_eq!(
        foot.world_point,
        Some([1.0 / 256.0, 100.0 / 256.0, 1.0 / 256.0]),
        "direct external endpoint remains authoritative"
    );
    let shadow = model.resolve_slot_with_context(2, context).unwrap();
    assert_eq!(shadow.native_view_point, Some([0; 3]));
    assert_eq!(
        shadow.world_point,
        Some([0.0; 3]),
        "4349C0 consumes the cached VIEW rather than the original world words"
    );
}

#[test]
fn native_callback_rebases_all_words_to_viewport_image_before_q31_projection() {
    use crate::native_model_frame::{NativeModelFrame, NativeWorldViewport};
    let viewport = NativeWorldViewport {
        origin_raw: [32760, -32760, 65536 + 32760],
        axes_q31: [[i32::MAX, 0, 0], [0, i32::MAX, 0], [0, 0, i32::MAX]],
        identity: true,
    };
    let model = ModelEntry {
        records: vec![[14, 0, 0, 0]],
        ..ModelEntry::default()
    };
    let callback = |_| {
        Some(external_frame::ActorExternalSlotPoint {
            draw_world: [128.03125, -128.03125, 384.03125],
            native_world_raw: [-32760, 32760, -32760],
        })
    };
    let resolver = WorldModelVertexResolver {
        surface: None,
        transform: ModelTransform {
            orientation: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            position: [0.0; 3],
            scale: 100.0 / 256.0,
        },
        view: None,
        native_view: None,
        native_context: Some((
            NativeModelFrame::from_actor(viewport, [0; 3], viewport.axes_q31),
            viewport,
        )),
        native_edges_required: true,
        model: &model,
        external_owner: Some(&callback),
        external_points: RefCell::new(HashMap::new()),
        native_vertex_failures: RefCell::new(Vec::new()),
    };
    let point = resolver.resolve_external_frame_raw(0, [0; 3]).unwrap();
    assert_eq!(
        point.native_view_point,
        Some([16, -16, 16]),
        "40D37A..40D3B0 WORD sub/MOVSX/full origin add is viewport-owned"
    );
}

#[test]
fn source_shadow_native_view_metadata_survives_linked_imports() {
    let mut source = ResolvedModelSlot::clear([999.0; 3]);
    source.world_point = Some([1.0, 2.0, 3.0]);
    source.native_view_point = Some([17, -29, 31]);
    let linked = vec![[Some(source); 2]];
    let model = ModelEntry {
        records: vec![[11, 0, 0, 0]],
        ..ModelEntry::default()
    };
    assert_eq!(
        model.resolve_slot_with_context(
            0,
            ModelMaterializationContext::intrinsic(&AnimVars::default(), Some(&linked))
        ),
        Some(source)
    );
}

#[test]
fn world_shadow_wave_policy_is_installed_from_authored_descriptor_84() {
    let colors = ModelMaterialCache::new();
    let mut renderer = RecordingRenderer::default();
    for raw_policy in [0_u32, 1, 0x80000000] {
        let mut bytes = vec![0_u8; 0xD0];
        bytes[0x84..0x88].copy_from_slice(&raw_policy.to_le_bytes());
        let level = v2k_formats::levels::parse_level(&bytes).unwrap();
        let terrain = TerrainGrid {
            header: [0, -4, 4, 8, 0],
            cells: vec![
                TerrainCell {
                    height: (-10_i8) as u8,
                    attribute: 0,
                    terrain_type: 0
                };
                GRID_SIZE * GRID_SIZE
            ],
        };
        let cache = synthetic_cache_with_terrain_and_level(vec![], Some(terrain), Some(level));
        let tree =
            ModelTreeRenderer::new_world(&mut renderer, &cache, &colors, 100.0 / 256.0, None, 1000);
        let surface = tree.world_surface_projection().unwrap();
        assert_eq!(
            surface.waves_enabled(),
            raw_policy != 0,
            "433BD0 preserves the full dword truth value"
        );
        let viewport = crate::native_model_frame::NativeWorldViewport {
            origin_raw: [0; 3],
            axes_q31: [[i32::MAX, 0, 0], [0, i32::MAX, 0], [0, 0, i32::MAX]],
            identity: true,
        };
        let projected = crate::native_world_surface::project_native_world_surface(
            [100, 300, 200],
            viewport,
            surface,
        );
        assert_eq!(
            projected.view_raw,
            if raw_policy == 0 {
                [175, 0, 50]
            } else {
                [176, -12, 47]
            },
            "original-machine wet controls selected by the authored flag"
        );
    }
}

#[test]
fn actual_newant_antenna_tail_retains_zero_cosine_before_final_sine_registers() {
    use crate::native_model_frame::{NativeModelFrame, NativeWorldViewport};
    use v2k_formats::models::ModelEdgeEndpointSnapshot;
    let viewport = NativeWorldViewport {
        origin_raw: [-15097, -243, 28884],
        axes_q31: [
            [2105482641, 0, 422129991],
            [-61663396, 2124313119, 307562155],
            [-417665851, -313697527, 2082951585],
        ],
        identity: false,
    };
    let frame = NativeModelFrame::from_actor(
        viewport,
        [-17501, -551, 31307],
        [
            [1621229568, -462225408, -1329135616],
            [398852096, 2095841280, -242089984],
            [1349451776, -64225280, 1668481024],
        ],
    );
    assert_eq!(frame.origin_view_raw, [-1881, 111, 2861]);
    assert_eq!(
        frame.axes_view_q31,
        [
            [1328253516, -694149098, -1536988840],
            [343463718, 2027103133, -618542007],
            [1651031509, 136678681, 1365268872]
        ]
    );
    let mut records = vec![[0; 4]; 66];
    records[26] = [0, 24, 28, 112];
    records[62] = [0, 42, 35, 154];
    records[63] = [0, 42, 49, 140];
    records[64] = [8, 194, 124, 126];
    records[65] = [8, 194, 125, 127];
    // Exact authored model302 words667..705. Other draw commands are not
    // executed by this bounded source-tail control.
    let model = ModelEntry {
        records,
        cmd_words: vec![
            0x5D, 1, 0x80, 0x0B, 0xED, 2, 0x8000, 0xC1, 0x0D, 2, 0x8000, 0xC2, 0x02, 32, 52, 128,
            0x22, 680, 3, 52, 128, 0xDD, 2, 0x8000, 0xC1, 0x0D, 2, 0x8000, 0xC2, 0x02, 32, 53, 130,
            0x22, 680, 3, 53, 130, 0,
        ],
        ..ModelEntry::default()
    };
    let mut vars = AnimVars::default();
    vars.dynamic[0] = 952;
    let resolver = WorldModelVertexResolver {
        surface: None,
        transform: ModelTransform {
            orientation: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            position: [0.; 3],
            scale: 100.0 / 256.0,
        },
        view: Some(ModelTreeView {
            position: viewport.origin_raw.map(|v| v as f32 / 256.0),
            forward: viewport.axes_q31[2].map(|v| -(v as f32 / 2147483648.0)),
        }),
        native_view: Some(frame.view_selection()),
        native_context: Some((frame.clone(), viewport)),
        native_edges_required: true,
        model: &model,
        external_owner: None,
        external_points: RefCell::new(HashMap::new()),
        native_vertex_failures: RefCell::new(Vec::new()),
    };
    let geometry = model.materialize_with_context(ModelMaterializationContext {
        vertex_resolver: Some(&resolver),
        ..ModelMaterializationContext::intrinsic(&vars, None)
    });
    let receipts = geometry
        .edge_endpoint_snapshots
        .iter()
        .map(|snapshot| match snapshot {
            ModelEdgeEndpointSnapshot::Native { endpoints } => *endpoints,
            other => panic!("source-owned antenna receipt: {other:?}"),
        })
        .collect::<Vec<_>>();
    assert_eq!(receipts.len(), 4);
    assert_eq!(receipts[0][1].slot, 128);
    assert_eq!(
        receipts[0][1].view_raw,
        [-1733, 139, 2916],
        "raw r2=0 at original cosine antenna command"
    );
    assert_eq!(
        receipts[1], receipts[0],
        "palette and ribbon read the same warmed source cache"
    );
    assert_eq!(receipts[2][1].slot, 130);
    assert_eq!(
        receipts[2][1].view_raw,
        [-1789, 173, 2972],
        "raw r2=0x8000 at original sine antenna command"
    );
    assert_eq!(receipts[3], receipts[2]);
    let mut final_vars = vars.clone();
    final_vars.registers[2] = 0x8000;
    assert_ne!(
        frame.resolve_model_slot(&model, &final_vars, 128),
        Some(receipts[0][1].view_raw),
        "final-vars replay cannot own the first native antenna endpoint"
    );
}
#[test]
fn native_missing_surface_receipt_survives_empty_geometry_and_preserves_other_faces() {
    use crate::native_model_frame::{NativeModelFrame, NativeWorldViewport};
    use v2k_formats::collision::SubHExternalFrameDescriptor;
    use v2k_formats::models::ModelNativeVertexBoundary;
    let axes = [[i32::MAX, 0, 0], [0, i32::MAX, 0], [0, 0, i32::MAX]];
    let viewport = NativeWorldViewport {
        origin_raw: [0; 3],
        axes_q31: [[i32::MAX, 0, 0], [0, i32::MAX, 0], [0, 0, -i32::MAX]],
        identity: false,
    };
    let descriptor = SubHExternalFrameDescriptor {
        completion_sound_id: None,
        records: vec![],
    };
    for ordinary_face in [false, true] {
        let mut commands = if ordinary_face {
            vec![0x03, 2, 0, 0, 2, 4]
        } else {
            vec![]
        };
        commands.extend([0x03, 2, 0, 8, 2, 4, 0]);
        let model = model_entry(
            0,
            "missing-surface-dependency",
            vec![
                [0, 0, 0, 0],
                [0, 100, 0, 0],
                [0, 0, 100, 0],
                [3, 0, 0, 0],
                [13, 6, 0, 0],
            ],
            commands,
        );
        let terrain = TerrainGrid {
            header: [-0x1800 << 8, 0, 0, 0, 0],
            cells: vec![
                TerrainCell {
                    height: 0,
                    attribute: 0,
                    terrain_type: 0
                };
                GRID_SIZE * GRID_SIZE
            ],
        };
        let cache = synthetic_cache_with_terrain(vec![model], Some(terrain));
        let colors = ModelMaterialCache::new();
        let mut renderer = RecordingRenderer::default();
        let mut buffer = ModelTreeSubmissionBuffer::default();
        let mut runtime = SubHRuntimeState::new(0).unwrap();
        let position = [0.0, 0.0, -10.0];
        let mut tree =
            ModelTreeRenderer::new_world(&mut renderer, &cache, &colors, 100.0 / 256.0, None, 0)
                .with_view(ModelTreeView {
                    position: [0.0; 3],
                    forward: [0.0, 0.0, -1.0],
                })
                .with_submission_buffer(&mut buffer)
                .with_sub_h_presentation(SubHPresentation {
                    runtime: &mut runtime,
                    descriptor: &descriptor,
                    model_records: &cache.global_model(0).unwrap().records,
                    retail_tick: 0,
                    origin_raw: [0, 0, -2560],
                    origin_world: position,
                    body_axes_q31: Some(axes),
                    native_context: Some((
                        NativeModelFrame::from_actor(viewport, [0, 0, -2560], axes),
                        viewport,
                    )),
                    fallback: None,
                    emitter: None,
                });
        tree.draw_linked(
            0,
            [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            position,
            0,
            None,
            &AnimVars::default(),
        );
        assert_eq!(
            tree.native_vertex_failures(),
            &[ModelTreeNativeVertexFailure {
                model_id: 0,
                dependency: ModelNativeVertexFailure {
                    consumer_slot: 8,
                    source_slot: 6,
                    boundary: ModelNativeVertexBoundary::UnsupportedGenerator {
                        slot: 6,
                        type_flag: 3
                    },
                },
            }]
        );
        drop(tree);
        buffer.flush(&mut renderer, None);
        let triangles: usize = renderer
            .bodies
            .iter()
            .map(|body| body.triangles.len())
            .sum();
        assert_eq!(triangles, usize::from(ordinary_face));
        assert!(renderer.bodies.iter().all(|body| body.edges.is_empty()));
        assert!(renderer.billboards.is_empty());
        if ordinary_face {
            for &slot in renderer.bodies[0].triangles.iter().flatten() {
                assert_eq!(renderer.bodies[0].vertex_type_flags[usize::from(slot)], 0);
            }
        }
    }
}
