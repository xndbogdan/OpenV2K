//! Deferred model-queue commands, before any camera or presentation policy.

use std::collections::{HashMap, HashSet};
use std::ops::Range;

use super::{
    AnimVars, LinkedModelSlots, ModelEdgeConstructorClip, ModelEdgeEndpointSnapshot,
    ModelNativeEdgeBoundary, ModelNativeEdgeEndpoint, ModelVertexResolver, RawStream,
    ResolvedModelSlot, SlotResolver, StreamStats,
};

/// How the children of one deferred queue record are drained.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelPainterSorting {
    /// `FUN_00494AB0`: descending signed depth, preserving emission order
    /// for equal keys, then drain (`FUN_00494930`).
    Sorted,
    /// `FUN_00494B60`: drain directly in command order (`FUN_00494A50`).
    Unsorted,
}

/// Retail signed camera-depth key, independent of geometric depth testing.
///
/// Points are raw model-local positions resolved at the command's register
/// snapshot. The submission transforms them to camera space; offsets are
/// added to camera Z afterwards, in raw model units. An empty maximum list
/// has key `-i32::MAX`, and an empty minimum list has key `i32::MAX`.
#[derive(Debug, Clone, PartialEq)]
pub enum ModelPainterDepthKey {
    /// Command-time signed VIEW depth from the presentation's current frame.
    /// Offsets have already been added with native dword wrapping.
    Native(i32),
    Vertex {
        position_raw: [f64; 3],
        offset_raw: i32,
    },
    Fixed(i32),
    Minimum(Vec<[f64; 3]>),
    Maximum(Vec<[f64; 3]>),
    /// Invalid/unresolved source slot; never invent a reference depth.
    Unresolved,
}

/// One reached model-stream operation in retail emission order.
///
/// `Instance` expands inline, inheriting the current queue stack. Group
/// boundaries are not implicitly added or removed at model entry/return.
/// A face range is one original polygon (or its separate mirror), so its
/// triangulation must remain atomic when its enclosing queue is sorted.
#[derive(Debug, Clone, PartialEq)]
pub enum ModelPainterOp {
    Face {
        triangle_range: Range<usize>,
        depth_key: ModelPainterDepthKey,
    },
    Edge {
        edge_index: usize,
        depth_key: ModelPainterDepthKey,
    },
    Billboard {
        billboard_index: usize,
        depth_key: ModelPainterDepthKey,
    },
    Instance {
        instance_index: usize,
    },
    BeginGroup {
        depth_key: ModelPainterDepthKey,
        sorting: ModelPainterSorting,
        /// Raw doubled model slots resolved by the reached group command,
        /// in source order. Literal/origin keys have no slot dependencies.
        /// Retained independently of coordinates for external-frame consumers.
        referenced_slots: Vec<u16>,
    },
    EndGroup,
}

pub(super) fn signed_offset(low: u16, high: u16) -> i32 {
    i32::from_le_bytes([low as u8, (low >> 8) as u8, high as u8, (high >> 8) as u8])
}

/// Reuse point lookups and allocation capacity between primitive commands.
/// Register writes invalidate these painter snapshots without changing the
/// independent geometry resolver's existing interpretation.
#[derive(Default)]
pub(super) struct PainterSlotCache {
    points: HashMap<u16, Option<ResolvedModelSlot>>,
    busy: HashSet<u16>,
}

impl PainterSlotCache {
    pub(super) fn clear(&mut self) {
        self.points.clear();
    }

    pub(super) fn with_resolver<T>(
        &mut self,
        records: &[[i16; 4]],
        vars: &AnimVars,
        linked: Option<&LinkedModelSlots>,
        vertex_resolver: Option<&dyn ModelVertexResolver>,
        resolve: impl FnOnce(&mut SlotResolver<'_>) -> T,
    ) -> T {
        let mut resolver = SlotResolver {
            records,
            vars,
            linked,
            vertex_resolver,
            cache: std::mem::take(&mut self.points),
            busy: std::mem::take(&mut self.busy),
        };
        let output = resolve(&mut resolver);
        self.points = resolver.cache;
        self.busy = resolver.busy;
        output
    }
}

pub(super) fn vertex_key(
    resolver: &mut SlotResolver<'_>,
    slot: u16,
    offset_raw: i32,
    stats: &mut StreamStats,
) -> ModelPainterDepthKey {
    let native = resolver
        .vertex_resolver
        .is_some_and(|owner| owner.owns_native_view());
    match resolver.resolve(slot, stats) {
        Some(slot) if native => slot
            .native_view_point
            .map(|point| ModelPainterDepthKey::Native(point[2].wrapping_add(offset_raw)))
            .unwrap_or(ModelPainterDepthKey::Unresolved),
        Some(slot) => ModelPainterDepthKey::Vertex {
            position_raw: slot.position_raw,
            offset_raw,
        },
        None => ModelPainterDepthKey::Unresolved,
    }
}

fn edge_endpoint_snapshot(
    resolver: &mut SlotResolver<'_>,
    slots: [u16; 2],
    sprite_constructor: bool,
    stats: &mut StreamStats,
) -> ModelEdgeEndpointSnapshot {
    let Some(owner) = resolver.vertex_resolver else {
        return ModelEdgeEndpointSnapshot::Compatibility;
    };
    let native = owner.owns_native_view();
    if !native && !owner.owns_external_frame_slots() {
        return ModelEdgeEndpointSnapshot::Compatibility;
    }
    let required = owner.requires_native_edge_endpoints();
    let mut missing = None;
    let mut endpoints = [ModelNativeEdgeEndpoint {
        slot: 0,
        view_raw: [0; 3],
        clip: super::ModelSlotClip::Clear,
    }; 2];
    // A22/B22 (459000/459550) resolve and test each endpoint in order. Bit40
    // on endpoint1 returns before endpoint2's H/E/provider callback. A02/B02
    // (458C60/458E20) instead resolve both, then apply the combined clip LUT.
    for index in 0..2 {
        let slot = slots[index];
        let point = resolver.resolve(slot, stats);
        if sprite_constructor && missing.is_none() {
            if let Some(point) = point {
                if point.clip == super::ModelSlotClip::SurfaceBand {
                    return ModelEdgeEndpointSnapshot::SourceClipped {
                        slot,
                        reason: ModelEdgeConstructorClip::SurfaceBand,
                    };
                }
                // Tf1 requires its completed projected-source dependency and
                // cannot borrow an ordinary native VIEW depth for this test.
                let screen_midpoint = resolver
                    .records
                    .get(usize::from(slot >> 1))
                    .is_some_and(|record| record[0] == 1);
                if !screen_midpoint {
                    if let Some(view) = point.native_view_point {
                        if view[2] < 0x40 {
                            return ModelEdgeEndpointSnapshot::SourceClipped {
                                slot,
                                reason: ModelEdgeConstructorClip::NearView { depth_raw: view[2] },
                            };
                        }
                    }
                }
            }
        }
        if native {
            let boundary = if point.is_none() {
                Some(ModelNativeEdgeBoundary::SlotUnavailable { slot })
            } else if resolver
                .records
                .get(usize::from(slot >> 1))
                .is_some_and(|record| record[0] == 1)
            {
                Some(ModelNativeEdgeBoundary::ScreenMidpoint { slot })
            } else if point.is_some_and(|point| point.native_view_point.is_none()) {
                Some(ModelNativeEdgeBoundary::ViewUnavailable { slot })
            } else {
                None
            };
            if let Some(boundary) = boundary {
                if required {
                    return ModelEdgeEndpointSnapshot::MissingNative { boundary };
                }
                // A deliberate compatibility scene keeps its legacy geometry
                // and visits both endpoints at this command's register state.
                // The missing receipt never becomes native parity evidence.
                missing.get_or_insert(boundary);
                continue;
            }
            let point = point.expect("boundary checked");
            endpoints[index] = ModelNativeEdgeEndpoint {
                slot,
                view_raw: point.native_view_point.expect("boundary checked"),
                clip: point.clip,
            };
        }
    }
    if let Some(boundary) = missing {
        ModelEdgeEndpointSnapshot::MissingNative { boundary }
    } else if native {
        ModelEdgeEndpointSnapshot::Native { endpoints }
    } else {
        ModelEdgeEndpointSnapshot::Compatibility
    }
}

/// Near-pass flat/Gouraud face handlers (59AC0, 5CFB0, 60CC0, 62A00
/// and their textured/mirrored families) use the first authored corner's Z.
/// Mirrored polygons receive their own first-corner key. Lines (458C60 /
/// 459000) likewise use the first endpoint, and 67840/67C60 use the anchor.
pub(super) fn record_primitives(
    out: &mut RawStream,
    start: (usize, usize, usize, usize),
    opcode: u16,
    resolver: &mut SlotResolver<'_>,
    normal_pool: &[[i16; 4]],
    stats: &mut StreamStats,
) {
    let triangle_count = out.tris.len() - start.0;
    if triangle_count != 0 {
        let mirrors = matches!(opcode & 0x3F, 0x07 | 0x08 | 0x27 | 0x28);
        let polygon_count = if mirrors { 2 } else { 1 };
        let polygon_triangles = triangle_count / polygon_count;
        for polygon in 0..polygon_count {
            let begin = start.0 + polygon * polygon_triangles;
            let face = &out.tris[begin];
            let admitted = resolver.live_face_admitted(face.normal, normal_pool);
            if admitted
                && resolver
                    .vertex_resolver
                    .is_some_and(|owner| owner.owns_external_frame_slots())
            {
                let corners: &[u16] = match &face.source_vertices {
                    super::ModelFaceVertices::Triangle(corners) => corners,
                    super::ModelFaceVertices::Quad(corners) => corners,
                };
                for &slot in corners {
                    resolver.resolve(slot, stats);
                }
            }
            let depth_key = if admitted {
                vertex_key(resolver, face.refs[0], 0, stats)
            } else {
                ModelPainterDepthKey::Unresolved
            };
            out.painter_program.push(ModelPainterOp::Face {
                triangle_range: begin..begin + polygon_triangles,
                depth_key,
            });
        }
    }
    for edge_index in start.1..out.edges.len() {
        let slots = [out.edges[edge_index].r1, out.edges[edge_index].r2];
        let sprite_constructor = matches!(
            out.edges[edge_index].style,
            super::ModelEdgeStyle::Sprite { .. }
        );
        out.edges[edge_index].requires_native_endpoints =
            resolver.vertex_resolver.is_some_and(|owner| {
                owner.owns_native_view() && owner.requires_native_edge_endpoints()
            });
        out.edges[edge_index].endpoint_snapshot =
            edge_endpoint_snapshot(resolver, slots, sprite_constructor, stats);
        let depth_key = vertex_key(resolver, out.edges[edge_index].r1, 0, stats);
        out.painter_program.push(ModelPainterOp::Edge {
            edge_index,
            depth_key,
        });
    }
    for billboard_index in start.2..out.billboards.len() {
        let depth_key = vertex_key(resolver, out.billboards[billboard_index].0, 0, stats);
        out.painter_program.push(ModelPainterOp::Billboard {
            billboard_index,
            depth_key,
        });
    }
    for instance_index in start.3..out.instances.len() {
        out.painter_program
            .push(ModelPainterOp::Instance { instance_index });
    }
}

/// Slot resolution can reject malformed primitives. Keep the surviving
/// materialized ranges/indices aligned without dropping surrounding scopes.
pub(super) fn remap_program(
    program: &[ModelPainterOp],
    triangle_prefix: &[usize],
    edge_remap: &[Option<usize>],
    billboard_remap: &[Option<usize>],
) -> Vec<ModelPainterOp> {
    program
        .iter()
        .filter_map(|op| match op {
            ModelPainterOp::Face {
                triangle_range,
                depth_key,
            } => {
                let start = triangle_prefix[triangle_range.start];
                let end = triangle_prefix[triangle_range.end];
                (start != end).then(|| ModelPainterOp::Face {
                    triangle_range: start..end,
                    depth_key: depth_key.clone(),
                })
            }
            ModelPainterOp::Edge {
                edge_index,
                depth_key,
            } => edge_remap[*edge_index].map(|edge_index| ModelPainterOp::Edge {
                edge_index,
                depth_key: depth_key.clone(),
            }),
            ModelPainterOp::Billboard {
                billboard_index,
                depth_key,
            } => {
                billboard_remap[*billboard_index].map(|billboard_index| ModelPainterOp::Billboard {
                    billboard_index,
                    depth_key: depth_key.clone(),
                })
            }
            _ => Some(op.clone()),
        })
        .collect()
}

#[cfg(test)]
mod tests;

/// 4669E0/466A70 fold current VIEW cache words, including native empty sentinels.
pub(super) fn extrema_key(
    resolver: &mut SlotResolver<'_>,
    slots: &[u16],
    maximum: bool,
    stats: &mut StreamStats,
) -> ModelPainterDepthKey {
    if resolver
        .vertex_resolver
        .is_some_and(|owner| owner.owns_native_view())
    {
        let mut depth = if maximum { -i32::MAX } else { i32::MAX };
        for &slot in slots {
            let Some(point) = resolver
                .resolve(slot, stats)
                .and_then(|slot| slot.native_view_point)
            else {
                return ModelPainterDepthKey::Unresolved;
            };
            depth = if maximum {
                depth.max(point[2])
            } else {
                depth.min(point[2])
            };
        }
        ModelPainterDepthKey::Native(depth)
    } else {
        let points = slots
            .iter()
            .map(|&slot| {
                resolver
                    .resolve(slot, stats)
                    .map(|value| value.position_raw)
            })
            .collect::<Option<Vec<_>>>();
        match points {
            Some(points) if maximum => ModelPainterDepthKey::Maximum(points),
            Some(points) => ModelPainterDepthKey::Minimum(points),
            None => ModelPainterDepthKey::Unresolved,
        }
    }
}

#[cfg(test)]
mod native_depth_tests {
    use super::super::{interpret, ModelMaterializationContext, ModelVertexKind};
    use super::*;

    struct NativeOwner {
        missing: Option<u16>,
    }
    impl ModelVertexResolver for NativeOwner {
        fn resolve_vertex_raw(
            &self,
            _: ModelVertexKind,
            _: ResolvedModelSlot,
        ) -> Option<ResolvedModelSlot> {
            None
        }
        fn owns_native_view(&self) -> bool {
            true
        }
        fn native_node_origin_view(&self) -> Option<[i32; 3]> {
            Some([999, 888, 123])
        }
        fn resolve_native_view_point(&self, slot: u16, _: &AnimVars) -> Option<[i32; 3]> {
            if self.missing == Some(slot) {
                return None;
            }
            Some([
                111,
                222,
                [-7, -100, 300, 200, 700, 600, 900, 800][usize::from(slot)],
            ])
        }
    }

    fn program(words: &[u16], missing: Option<u16>) -> Vec<ModelPainterOp> {
        let records = [[0, 10, 20, 30]; 4];
        let vars = AnimVars::default();
        let owner = NativeOwner { missing };
        interpret(
            words,
            &records,
            &[],
            ModelMaterializationContext {
                vars: &vars,
                linked: None,
                vertex_resolver: Some(&owner),
                view_selection: super::super::ModelViewSelection::IntrinsicAllBranches,
            },
            &mut StreamStats::default(),
        )
        .0
        .painter_program
    }

    #[test]
    fn native_painter_groups_match_original_pe_signed_view_keys_and_empty_sentinels() {
        // Original executable 4669E0/466A70/466B00/466B60/466BC0/466C20
        // controls: execute-native-model-primitive-receipts.py groups().
        for (words, expected) in [
            (vec![0x06, 0, 2, 4, 0xFFFF, 0xE6, 0], 700),
            (vec![0x26, 0, 2, 4, 0xFFFF, 0xE6, 0], -7),
            (vec![0x06, 0xFFFF, 0xE6, 0], -i32::MAX),
            (vec![0x26, 0xFFFF, 0xE6, 0], i32::MAX),
            (vec![0x46, 2, 0xE6, 0], 300),
            (vec![0x66, 0xE6, 0], 123),
            (vec![0xC6, 0, 0xFFFF, 0x7FFF, 0xE6, 0], 2147483640),
            (vec![0xC6, 2, 0xFFFF, 0x7FFF, 0xE6, 0], -2147483349),
            (vec![0x15, 2, 0xE6, 0], 300),
        ] {
            let operations = program(&words, None);
            assert!(
                matches!(&operations[0], ModelPainterOp::BeginGroup { depth_key: ModelPainterDepthKey::Native(depth), .. } if *depth == expected)
            );
        }
        let operations = program(&[0x86, 0xE6, 0xA6, 0xFFFE, 0xFFFF, 0xE6, 0], None);
        assert!(matches!(
            &operations[0],
            ModelPainterOp::BeginGroup {
                depth_key: ModelPainterDepthKey::Fixed(-1),
                ..
            }
        ));
        assert!(matches!(
            &operations[2],
            ModelPainterOp::BeginGroup {
                depth_key: ModelPainterDepthKey::Fixed(-2),
                ..
            }
        ));
    }

    #[test]
    fn native_painter_primitive_first_keys_survive_unequal_endpoints_and_mirrors() {
        // First VIEW endpoint, rather than midpoint/last or raw-local XYZ.
        let operations = program(
            &[
                0x07, 10, 0, 0, 2, 4, 0x22, 986, 4, 2, 4, 0x02, 10, 4, 6, 0x78, 6, 986, 4, 0, 0,
            ],
            None,
        );
        let depths = operations
            .iter()
            .map(|op| match op {
                ModelPainterOp::Face { depth_key, .. }
                | ModelPainterOp::Edge { depth_key, .. }
                | ModelPainterOp::Billboard { depth_key, .. } => depth_key.clone(),
                _ => panic!("unexpected group"),
            })
            .collect::<Vec<_>>();
        assert_eq!(
            depths,
            [-7, -100, 300, 700, 900].map(ModelPainterDepthKey::Native)
        );
    }

    #[test]
    fn missing_native_painter_key_never_falls_back_to_resolved_local_position() {
        let operations = program(
            &[0x03, 10, 0, 0, 2, 4, 0x06, 0, 2, 4, 0xFFFF, 0xE6, 0],
            Some(0),
        );
        assert!(matches!(
            &operations[0],
            ModelPainterOp::Face {
                depth_key: ModelPainterDepthKey::Unresolved,
                ..
            }
        ));
        assert!(matches!(
            &operations[1],
            ModelPainterOp::BeginGroup {
                depth_key: ModelPainterDepthKey::Unresolved,
                ..
            }
        ));
    }
}
