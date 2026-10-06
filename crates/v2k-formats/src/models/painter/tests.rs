use super::{ModelPainterDepthKey as Key, ModelPainterOp as Op, ModelPainterSorting as Sorting};
use crate::models::{AnimVars, ModelEntry};

fn entry(words: &[u16]) -> ModelEntry {
    ModelEntry {
        records: vec![
            [0, 10, 0, 90],
            [0, 20, 10, 10],
            [0, 30, 0, 50],
            [0, 40, 10, 200],
        ],
        cmd_words: words.to_vec(),
        ..ModelEntry::default()
    }
}

fn vertex(position_raw: [f64; 3], offset_raw: i32) -> Key {
    Key::Vertex {
        position_raw,
        offset_raw,
    }
}

#[test]
fn quads_and_mirrors_keep_source_polygon_keys_and_atomic_ranges() {
    let model = entry(&[
        0x15, 0, // Sorted group.
        0x08, 7, 0, 0, 2, 4, 6, // Mirrored quad, two distinct painter items.
        0x03, 8, 0, 2, 4, 6, 0xE6, 0,
    ])
    .materialize(&AnimVars::default());
    assert_eq!(
        model.painter_program,
        vec![
            Op::BeginGroup {
                depth_key: vertex([10.0, 0.0, 90.0], 0),
                sorting: Sorting::Sorted,
                referenced_slots: vec![0],
            },
            Op::Face {
                triangle_range: 0..2,
                depth_key: vertex([10.0, 0.0, 90.0], 0)
            },
            Op::Face {
                triangle_range: 2..4,
                depth_key: vertex([-10.0, 0.0, 90.0], 0)
            },
            Op::Face {
                triangle_range: 4..5,
                depth_key: vertex([20.0, 10.0, 10.0], 0)
            },
            Op::EndGroup,
        ]
    );
    assert_eq!(model.face_materials, [7, 7, 7, 7, 8]);
}

#[test]
fn instances_retain_their_place_inside_nested_queue_scopes() {
    let model = entry(&[
        0x15, 0, 0xC6, 2, 0xFE3E, 0xFFFF, // -450; unsigned low, signed high.
        0x03, 7, 0, 0, 2, 4, 0x0E, 6, 9, 8, 0, 0xE6, 0x46,
        4, // This is unsorted, unlike 0x15.
        0x02, 8, 0, 2, 0x68, 4, 9, 16, 0, 0xE6, 0xE6, 0,
    ])
    .materialize(&AnimVars::default());
    assert_eq!(
        model.painter_program,
        vec![
            Op::BeginGroup {
                depth_key: vertex([10.0, 0.0, 90.0], 0),
                sorting: Sorting::Sorted,
                referenced_slots: vec![0],
            },
            Op::BeginGroup {
                depth_key: vertex([20.0, 10.0, 10.0], -450),
                sorting: Sorting::Unsorted,
                referenced_slots: vec![2],
            },
            Op::Face {
                triangle_range: 0..1,
                depth_key: vertex([10.0, 0.0, 90.0], 0)
            },
            Op::Instance { instance_index: 0 },
            Op::EndGroup,
            Op::BeginGroup {
                depth_key: vertex([30.0, 0.0, 50.0], 0),
                sorting: Sorting::Unsorted,
                referenced_slots: vec![4],
            },
            Op::Edge {
                edge_index: 0,
                depth_key: vertex([10.0, 0.0, 90.0], 0)
            },
            Op::Billboard {
                billboard_index: 0,
                depth_key: vertex([30.0, 0.0, 50.0], 0)
            },
            Op::EndGroup,
            Op::EndGroup,
        ]
    );
    assert_eq!(model.instances[0].model_id, 9);
}

#[test]
fn group_anchor_uses_command_registers_before_later_writes() {
    let model = ModelEntry {
        records: vec![[0, 0, 0, 0], [0, 0, 0, 200], [8, 0xC0, 0, 2]],
        cmd_words: vec![
            0x0D, 0, 0x8000, 0, 0xC6, 4, 1500, 0, 0xE6, 0x0D, 0, 0, 0, 0xC6, 4, 2250, 0, 0xE6, 0,
        ],
        ..ModelEntry::default()
    }
    .materialize(&AnimVars::default());
    assert_eq!(
        model.painter_program[0],
        Op::BeginGroup {
            depth_key: vertex([0.0, 0.0, 100.0], 1500),
            sorting: Sorting::Unsorted,
            referenced_slots: vec![4],
        }
    );
    assert_eq!(
        model.painter_program[2],
        Op::BeginGroup {
            depth_key: vertex([0.0, 0.0, 0.0], 2250),
            sorting: Sorting::Unsorted,
            referenced_slots: vec![4],
        }
    );
}

#[test]
fn failed_slot_resolution_preserves_scopes_and_remaps_surviving_ranges() {
    let model = entry(&[
        0xC6, 100, 2250, 0, 0x03, 7, 0, 100, 2, 4, // Unresolvable face is absent.
        0x04, 8, 0, 0, 2, 4, 6, 0xE6, 0,
    ])
    .materialize(&AnimVars::default());
    assert_eq!(
        model.painter_program,
        vec![
            Op::BeginGroup {
                depth_key: Key::Unresolved,
                sorting: Sorting::Unsorted,
                referenced_slots: vec![100],
            },
            Op::Face {
                triangle_range: 0..2,
                depth_key: vertex([10.0, 0.0, 90.0], 0)
            },
            Op::EndGroup,
        ]
    );
}

#[test]
fn list_origin_and_signed_fixed_groups_keep_distinct_depth_policies() {
    let model = entry(&[
        0x06, 0, 2, 0xFFFF, 0xE6, 0x26, 4, 6, 0xFFFF, 0xE6, 0x66, 0xE6, 0x86, 0xE6, 0xA6, 0,
        0x8000, 0xE6, 0,
    ])
    .materialize(&AnimVars::default());
    let keys: Vec<_> = model
        .painter_program
        .iter()
        .filter_map(|op| match op {
            Op::BeginGroup {
                depth_key, sorting, ..
            } => {
                assert_eq!(*sorting, Sorting::Unsorted);
                Some(depth_key.clone())
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        keys,
        vec![
            Key::Maximum(vec![[10.0, 0.0, 90.0], [20.0, 10.0, 10.0]]),
            Key::Minimum(vec![[30.0, 0.0, 50.0], [40.0, 10.0, 200.0]]),
            vertex([0.0; 3], 0),
            Key::Fixed(-1),
            Key::Fixed(i32::MIN),
        ]
    );
    let references: Vec<_> = model
        .painter_program
        .iter()
        .filter_map(|op| match op {
            Op::BeginGroup {
                referenced_slots, ..
            } => Some(referenced_slots.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        references,
        vec![vec![0, 2], vec![4, 6], vec![], vec![], vec![]]
    );
}

#[test]
fn group_raw_slot_provenance_follows_dynamic_branch_and_ignores_debug_scan() {
    let entry = ModelEntry {
        records: vec![[14, 1, 0, 0], [14, 1, 0, 0]],
        cmd_words: vec![
            0x2B, 8, 0x80, 1, 0xC6, 0, 0xFFFF, 0xFFFF, 0xE6, 0x46, 2, 0xE6, 0x38, 0, 2, 0xFFFF, 0,
        ],
        ..ModelEntry::default()
    };
    for (dynamic, expected) in [(0, vec![vec![2]]), (1, vec![vec![0], vec![2]])] {
        let mut vars = AnimVars::default();
        vars.dynamic[0] = dynamic;
        let model = entry.materialize(&vars);
        let references: Vec<_> = model
            .painter_program
            .iter()
            .filter_map(|op| match op {
                Op::BeginGroup {
                    referenced_slots, ..
                } => Some(referenced_slots.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(references, expected);
        // Equal resolved coordinates do not erase distinct raw slot identity.
        assert!(model.painter_program.iter().all(|op| match op {
            Op::BeginGroup {
                depth_key: Key::Vertex { position_raw, .. },
                ..
            } => *position_raw == [1., 0., 0.],
            Op::EndGroup => true,
            _ => false,
        }));
    }
}

struct NativeSnapshotResolver {
    seen: std::cell::RefCell<Vec<(u16, u16)>>,
    phases: std::cell::RefCell<Vec<i32>>,
    missing: Option<u16>,
}

impl crate::models::ModelVertexResolver for NativeSnapshotResolver {
    fn resolve_vertex_raw(
        &self,
        _: crate::models::ModelVertexKind,
        _: crate::models::ResolvedModelSlot,
    ) -> Option<crate::models::ResolvedModelSlot> {
        None
    }
    fn owns_native_view(&self) -> bool {
        true
    }
    fn resolve_native_view_point(&self, slot: u16, vars: &AnimVars) -> Option<[i32; 3]> {
        self.seen.borrow_mut().push((slot, vars.registers[2]));
        if self.missing == Some(slot) {
            return None;
        }
        let odd = slot & 1;
        match slot & !1 {
            0 => Some([if odd == 0 { -120 } else { 120 }, 20, 512]),
            2 => Some([if odd == 0 { 140 } else { -140 }, 80, 768]),
            _ => None,
        }
    }
    fn resolve_native_spatial_point(
        &self,
        generator: crate::models::ModelNativeSpatialGenerator,
    ) -> Option<[i32; 3]> {
        if let crate::models::ModelNativeSpatialGenerator::Lerp { phase, .. } = generator {
            self.phases.borrow_mut().push(phase);
        }
        Some(generator.evaluate([0; 3]))
    }
}

#[test]
fn edge_snapshots_retain_r2_at_each_command_instead_of_final_vars() {
    use crate::models::{ModelEdgeEndpointSnapshot, ModelMaterializationContext};
    let model = ModelEntry {
        records: vec![[0, -120, 20, 512], [0, 140, 80, 768], [8, 0xC2, 0, 2]],
        cmd_words: vec![
            0x0D, 2, 1, 0, 0x22, 680, 3, 4, 2, 0x0D, 2, 0x8000, 0, 0x02, 32, 4, 2, 0,
        ],
        ..ModelEntry::default()
    };
    let vars = AnimVars::default();
    let owner = NativeSnapshotResolver {
        seen: Default::default(),
        phases: Default::default(),
        missing: None,
    };
    let geometry = model.materialize_with_context(ModelMaterializationContext {
        vertex_resolver: Some(&owner),
        ..ModelMaterializationContext::intrinsic(&vars, None)
    });
    let [ModelEdgeEndpointSnapshot::Native { endpoints: first }, ModelEdgeEndpointSnapshot::Native { endpoints: second }] =
        geometry.edge_endpoint_snapshots.as_slice()
    else {
        panic!("owned endpoints");
    };
    assert_eq!(first[0].view_raw, [-120, 20, 512]);
    assert_eq!(second[0].view_raw, [10, 50, 640]);
    assert_eq!(first[1].view_raw, second[1].view_raw);
    assert_eq!(first[0].slot, 4);
    assert_ne!(first[0].view_raw, second[0].view_raw);
    assert_eq!(
        geometry.vertices[usize::from(geometry.edges[0].vertices[0])],
        geometry.vertices[usize::from(geometry.edges[1].vertices[0])],
        "legacy final-vars vertex remap cannot be the receipt"
    );
    assert_eq!(owner.seen.borrow()[0], (0, 1));
    assert!(owner.phases.borrow().contains(&0x8000));
}

#[test]
fn native_missing_and_mirrored_edge_receipts_stay_aligned_after_geometry_remap() {
    use crate::models::{
        ModelEdgeEndpointSnapshot as Snapshot, ModelMaterializationContext,
        ModelNativeEdgeBoundary as Boundary,
    };
    let model = ModelEntry {
        records: vec![[0, -120, 20, 512], [0, 140, 80, 768]],
        cmd_words: vec![0x02, 32, 99, 2, 0x22, 680, 7, 1, 3, 0x02, 32, 0, 2, 0],
        ..ModelEntry::default()
    };
    let vars = AnimVars::default();
    let owner = NativeSnapshotResolver {
        seen: Default::default(),
        phases: Default::default(),
        missing: Some(2),
    };
    let geometry = model.materialize_with_context(ModelMaterializationContext {
        vertex_resolver: Some(&owner),
        ..ModelMaterializationContext::intrinsic(&vars, None)
    });
    assert_eq!(geometry.edges.len(), 1);
    let [Snapshot::Native { endpoints }] = geometry.edge_endpoint_snapshots.as_slice() else {
        panic!(
            "owned mirrored edge must stay aligned: {:?}",
            geometry.edge_endpoint_snapshots
        );
    };
    assert_eq!(
        geometry.native_edge_failures,
        [
            crate::models::ModelNativeEdgeFailure {
                source_edge_index: 0,
                source_slots: [99, 2],
                boundary: Boundary::SlotUnavailable { slot: 99 }
            },
            crate::models::ModelNativeEdgeFailure {
                source_edge_index: 2,
                source_slots: [0, 2],
                boundary: Boundary::ViewUnavailable { slot: 2 }
            },
        ]
    );
    assert_eq!(endpoints[0].slot, 1);
    assert_eq!(endpoints[0].view_raw, [120, 20, 512]);
    assert_eq!(endpoints[1].view_raw, [-140, 80, 768]);
    assert!(matches!(
        geometry.painter_program[0],
        Op::Edge { edge_index: 0, .. }
    ));
    assert_eq!(
        geometry.painter_program.len(),
        1,
        "failed edges never become painter allocations"
    );
}

#[test]
fn native_screen_midpoint_is_an_explicit_endpoint_boundary() {
    use crate::models::{
        ModelEdgeEndpointSnapshot as Snapshot, ModelMaterializationContext,
        ModelNativeEdgeBoundary as Boundary,
    };
    let model = ModelEntry {
        records: vec![[0, -120, 20, 512], [0, 140, 80, 768], [1, 0, 0, 2]],
        cmd_words: vec![0x02, 32, 4, 2, 0],
        ..ModelEntry::default()
    };
    let vars = AnimVars::default();
    let owner = NativeSnapshotResolver {
        seen: Default::default(),
        phases: Default::default(),
        missing: None,
    };
    let geometry = model.materialize_with_context(ModelMaterializationContext {
        vertex_resolver: Some(&owner),
        ..ModelMaterializationContext::intrinsic(&vars, None)
    });
    assert!(geometry.edges.is_empty() && geometry.edge_endpoint_snapshots.is_empty());
    assert_eq!(
        geometry.native_edge_failures,
        [crate::models::ModelNativeEdgeFailure {
            source_edge_index: 0,
            source_slots: [4, 2],
            boundary: Boundary::ScreenMidpoint { slot: 4 },
        }]
    );
    assert_eq!(
        model.materialize(&vars).edge_endpoint_snapshots,
        [Snapshot::Compatibility],
        "intrinsic producer is an explicit compatibility mode"
    );
}

struct ConstructorOrderResolver {
    seen: std::cell::RefCell<Vec<(u16, u16)>>,
    clipped_slot: u16,
    surface_band: bool,
    depth_raw: i32,
    native: bool,
}

impl crate::models::ModelVertexResolver for ConstructorOrderResolver {
    fn resolve_vertex_raw(
        &self,
        _: crate::models::ModelVertexKind,
        _: crate::models::ResolvedModelSlot,
    ) -> Option<crate::models::ResolvedModelSlot> {
        None
    }
    fn owns_external_frame_slots(&self) -> bool {
        true
    }
    fn owns_native_view(&self) -> bool {
        self.native
    }
    fn resolve_external_frame_raw(
        &self,
        slot: u16,
        _: [i16; 3],
    ) -> Option<crate::models::ResolvedModelSlot> {
        let mut point = crate::models::ResolvedModelSlot::clear([10.0, 20.0, 512.0]);
        if self.surface_band && slot == self.clipped_slot {
            point.clip = crate::models::ModelSlotClip::SurfaceBand;
        }
        Some(point)
    }
    fn resolve_native_view_point(&self, slot: u16, vars: &AnimVars) -> Option<[i32; 3]> {
        // This is the reached H/E/provider side effect. The final materializer
        // must not cause the first visit to an endpoint rejected by A22/B22.
        self.seen.borrow_mut().push((slot, vars.registers[2]));
        self.native.then_some([
            10,
            20,
            if slot == self.clipped_slot {
                self.depth_raw
            } else {
                512
            },
        ])
    }
}

fn constructor_order_owner(
    slot: u16,
    surface_band: bool,
    depth_raw: i32,
    native: bool,
) -> ConstructorOrderResolver {
    ConstructorOrderResolver {
        seen: Default::default(),
        clipped_slot: slot,
        surface_band,
        depth_raw,
        native,
    }
}

#[test]
fn sprite_first_bit40_stops_before_second_callback_and_final_remap() {
    use crate::models::{
        ModelEdgeConstructorClip as Clip, ModelEdgeEndpointSnapshot as Snapshot,
        ModelMaterializationContext,
    };
    let model = ModelEntry {
        records: vec![[14, 0, 0, 0], [14, 1, 0, 0]],
        cmd_words: vec![0x22, 986, 4, 0, 2, 0],
        ..ModelEntry::default()
    };
    let vars = AnimVars::default();
    for (surface_band, depth_raw, native) in
        [(true, 512, true), (false, 63, true), (true, 512, false)]
    {
        let owner = constructor_order_owner(0, surface_band, depth_raw, native);
        let geometry = model.materialize_with_context(ModelMaterializationContext {
            vertex_resolver: Some(&owner),
            ..ModelMaterializationContext::intrinsic(&vars, None)
        });
        assert_eq!(
            &*owner.seen.borrow(),
            &[(0, 0)],
            "unvisited endpoint2 cannot enter H/E/provider at final remap"
        );
        assert!(geometry.edges.is_empty());
        assert!(geometry.painter_program.is_empty());
        // The raw receipt is a source rejection, never missing custody or a
        // fabricated completed pair. Inspect the same constructor helper.
        let mut resolver = super::super::SlotResolver::new(&model.records, &vars)
            .with_vertex_resolver(Some(&owner));
        let snapshot = super::edge_endpoint_snapshot(
            &mut resolver,
            [0, 2],
            true,
            &mut crate::models::StreamStats::default(),
        );
        let reason = if surface_band {
            Clip::SurfaceBand
        } else {
            Clip::NearView { depth_raw }
        };
        assert_eq!(snapshot, Snapshot::SourceClipped { slot: 0, reason });
    }
}

#[test]
fn palette_bit40_still_visits_both_endpoints_before_lut() {
    use crate::models::{ModelEdgeEndpointSnapshot as Snapshot, ModelMaterializationContext};
    let model = ModelEntry {
        records: vec![[14, 0, 0, 0], [14, 1, 0, 0]],
        cmd_words: vec![0x02, 32, 0, 2, 0],
        ..ModelEntry::default()
    };
    let vars = AnimVars::default();
    for (surface_band, depth_raw) in [(true, 512), (false, 63)] {
        let owner = constructor_order_owner(0, surface_band, depth_raw, true);
        let geometry = model.materialize_with_context(ModelMaterializationContext {
            vertex_resolver: Some(&owner),
            ..ModelMaterializationContext::intrinsic(&vars, None)
        });
        assert_eq!(&owner.seen.borrow()[..2], &[(0, 0), (2, 0)]);
        assert!(matches!(
            geometry.edge_endpoint_snapshots.as_slice(),
            [Snapshot::Native { .. }]
        ));
    }
}

#[test]
fn sprite_second_bit40_visits_in_order_then_removes_unallocated_primitive() {
    use crate::models::ModelMaterializationContext;
    let model = ModelEntry {
        records: vec![[14, 0, 0, 0], [14, 1, 0, 0]],
        cmd_words: vec![0x22, 986, 4, 0, 2, 0],
        ..ModelEntry::default()
    };
    let vars = AnimVars::default();
    let owner = constructor_order_owner(2, false, 63, true);
    let geometry = model.materialize_with_context(ModelMaterializationContext {
        vertex_resolver: Some(&owner),
        ..ModelMaterializationContext::intrinsic(&vars, None)
    });
    assert_eq!(&*owner.seen.borrow(), &[(0, 0), (2, 0)]);
    assert!(geometry.edges.is_empty() && geometry.painter_program.is_empty());
}

#[test]
fn later_reached_command_owns_the_first_visit_of_skipped_sprite_endpoint() {
    use crate::models::ModelMaterializationContext;
    let model = ModelEntry {
        records: vec![[14, 0, 0, 0], [14, 1, 0, 0]],
        cmd_words: vec![0x22, 986, 4, 0, 2, 0x0D, 2, 0x83, 0, 0x46, 2, 0],
        ..ModelEntry::default()
    };
    let mut vars = AnimVars::default();
    vars.dynamic[3] = 77;
    let owner = constructor_order_owner(0, false, 63, true);
    let geometry = model.materialize_with_context(ModelMaterializationContext {
        vertex_resolver: Some(&owner),
        ..ModelMaterializationContext::intrinsic(&vars, None)
    });
    assert_eq!(&*owner.seen.borrow(), &[(0, 0), (2, 77)]);
    assert!(geometry.edges.is_empty());
    assert!(matches!(
        geometry.painter_program.as_slice(),
        [Op::BeginGroup { .. }]
    ));
}

struct MissingEndpointResolver {
    seen: std::cell::RefCell<Vec<(u16, u16)>>,
    missing_slot: u16,
    required: bool,
}

impl crate::models::ModelVertexResolver for MissingEndpointResolver {
    fn resolve_vertex_raw(
        &self,
        _: crate::models::ModelVertexKind,
        _: crate::models::ResolvedModelSlot,
    ) -> Option<crate::models::ResolvedModelSlot> {
        None
    }
    fn owns_native_view(&self) -> bool {
        true
    }
    fn owns_external_frame_slots(&self) -> bool {
        true
    }
    fn requires_native_edge_endpoints(&self) -> bool {
        self.required
    }
    fn resolve_external_frame_raw(
        &self,
        _: u16,
        _: [i16; 3],
    ) -> Option<crate::models::ResolvedModelSlot> {
        Some(crate::models::ResolvedModelSlot::clear([10., 20., 512.]))
    }
    fn resolve_native_view_point(&self, slot: u16, vars: &AnimVars) -> Option<[i32; 3]> {
        self.seen.borrow_mut().push((slot, vars.registers[2]));
        (slot != self.missing_slot).then_some([10, 20, 512])
    }
}

#[test]
fn required_missing_first_endpoint_does_not_enter_second_provider_at_remap() {
    use crate::models::{
        ModelMaterializationContext, ModelNativeEdgeBoundary as Boundary, ModelNativeEdgeFailure,
    };
    let vars = AnimVars::default();
    for command in [vec![0x02, 32, 0, 2, 0], vec![0x22, 986, 4, 0, 2, 0]] {
        let model = ModelEntry {
            records: vec![[14, 0, 0, 0], [14, 1, 0, 0]],
            cmd_words: command,
            ..ModelEntry::default()
        };
        let owner = MissingEndpointResolver {
            seen: Default::default(),
            missing_slot: 0,
            required: true,
        };
        let geometry = model.materialize_with_context(ModelMaterializationContext {
            vertex_resolver: Some(&owner),
            ..ModelMaterializationContext::intrinsic(&vars, None)
        });
        assert_eq!(&*owner.seen.borrow(), &[(0, 0)]);
        assert!(
            geometry.edges.is_empty()
                && geometry.edge_endpoint_snapshots.is_empty()
                && geometry.painter_program.is_empty()
        );
        assert_eq!(
            geometry.native_edge_failures,
            [ModelNativeEdgeFailure {
                source_edge_index: 0,
                source_slots: [0, 2],
                boundary: Boundary::ViewUnavailable { slot: 0 },
            }],
            "the failed command survives without fabricated geometry"
        );
    }
}

#[test]
fn required_missing_second_endpoint_keeps_reached_prefix_and_failure() {
    use crate::models::{ModelMaterializationContext, ModelNativeEdgeBoundary as Boundary};
    let model = ModelEntry {
        records: vec![[14, 0, 0, 0], [14, 1, 0, 0]],
        cmd_words: vec![0x22, 986, 4, 0, 2, 0],
        ..ModelEntry::default()
    };
    let vars = AnimVars::default();
    let owner = MissingEndpointResolver {
        seen: Default::default(),
        missing_slot: 2,
        required: true,
    };
    let geometry = model.materialize_with_context(ModelMaterializationContext {
        vertex_resolver: Some(&owner),
        ..ModelMaterializationContext::intrinsic(&vars, None)
    });
    assert_eq!(&*owner.seen.borrow(), &[(0, 0), (2, 0)]);
    assert!(geometry.edges.is_empty());
    assert_eq!(
        geometry.native_edge_failures[0].boundary,
        Boundary::ViewUnavailable { slot: 2 }
    );
}

#[test]
fn deliberate_compatibility_scene_visits_both_and_retains_missing_receipt() {
    use crate::models::{
        ModelEdgeEndpointSnapshot as Snapshot, ModelMaterializationContext,
        ModelNativeEdgeBoundary as Boundary,
    };
    let model = ModelEntry {
        records: vec![[14, 0, 0, 0], [14, 1, 0, 0]],
        cmd_words: vec![0x22, 986, 4, 0, 2, 0],
        ..ModelEntry::default()
    };
    let vars = AnimVars::default();
    let owner = MissingEndpointResolver {
        seen: Default::default(),
        missing_slot: 0,
        required: false,
    };
    let geometry = model.materialize_with_context(ModelMaterializationContext {
        vertex_resolver: Some(&owner),
        ..ModelMaterializationContext::intrinsic(&vars, None)
    });
    assert_eq!(
        &owner.seen.borrow()[..2],
        &[(0, 0), (2, 0)],
        "compatibility continuation occurs at command-time, before final remap"
    );
    assert_eq!(geometry.edges.len(), 1);
    assert!(geometry.native_edge_failures.is_empty());
    assert_eq!(
        geometry.edge_endpoint_snapshots,
        [Snapshot::MissingNative {
            boundary: Boundary::ViewUnavailable { slot: 0 }
        }]
    );
}

#[test]
fn later_command_reaches_missing_edges_unvisited_endpoint_under_its_own_registers() {
    use crate::models::ModelMaterializationContext;
    let model = ModelEntry {
        records: vec![[14, 0, 0, 0], [14, 1, 0, 0]],
        cmd_words: vec![0x22, 986, 4, 0, 2, 0x0D, 2, 0x83, 0, 0x46, 2, 0],
        ..ModelEntry::default()
    };
    let mut vars = AnimVars::default();
    vars.dynamic[3] = 77;
    let owner = MissingEndpointResolver {
        seen: Default::default(),
        missing_slot: 0,
        required: true,
    };
    let geometry = model.materialize_with_context(ModelMaterializationContext {
        vertex_resolver: Some(&owner),
        ..ModelMaterializationContext::intrinsic(&vars, None)
    });
    assert_eq!(&*owner.seen.borrow(), &[(0, 0), (2, 77)]);
    assert_eq!(geometry.native_edge_failures.len(), 1);
    assert!(matches!(
        geometry.painter_program.as_slice(),
        [Op::BeginGroup { .. }]
    ));
}
