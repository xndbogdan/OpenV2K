//! Recursive model vertices with presentation-owned contextual callbacks.

use std::collections::{HashMap, HashSet};

use super::{
    eval_var, retail_tf8_axis, retail_tf9_axis, AnimVars, ModelNativeSpatialGenerator, StreamStats,
};

/// Semantic clipping contributed by a vertex callback. Ordinary view-frustum
/// clipping belongs to the renderer and is not inherited by generators.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ModelSlotClip {
    #[default]
    Clear,
    /// World tf13's inclusive sea +/-0x96 rejection bit (retail 0x40).
    SurfaceBand,
}

/// Presentation provenance for the existing world-surface depth adapter.
/// Authored type flags remain unchanged when tf11 imports a projected endpoint.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ModelSurfaceOrigin {
    #[default]
    None,
    ViewPin,
}

/// One resolved vertex in the current model's raw-local frame. A rejected
/// vertex still has coordinates: arithmetic generators consume them and write
/// fresh clip flags, while tf11 copies the parent vertex including its clip.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedModelSlot {
    pub position_raw: [f64; 3],
    /// Owned callback endpoint in true draw-world units. Tf11 preserves it;
    /// spatial arithmetic writes a fresh point and clears this provenance.
    pub world_point: Option<[f64; 3]>,
    /// Authenticated current-node VIEW cache, before any GL compatibility
    /// projection. Tf11 copies it; spatial arithmetic recomputes or clears it.
    pub native_view_point: Option<[i32; 3]>,
    pub clip: ModelSlotClip,
    pub surface_origin: ModelSurfaceOrigin,
}

impl ResolvedModelSlot {
    pub const fn clear(position_raw: [f64; 3]) -> Self {
        Self {
            position_raw,
            world_point: None,
            native_view_point: None,
            clip: ModelSlotClip::Clear,
            surface_origin: ModelSurfaceOrigin::None,
        }
    }
}

/// Completed endpoint cache read by one reached 0x02/0x22 command.
/// The raw doubled slot identifies the command-time source, independently of
/// the final materialized vertex pool and any later register writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelNativeEdgeEndpoint {
    pub slot: u16,
    pub view_raw: [i32; 3],
    pub clip: ModelSlotClip,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModelNativeEdgeBoundary {
    SlotUnavailable {
        slot: u16,
    },
    ViewUnavailable {
        slot: u16,
    },
    /// Tf1 averages projected source pixels, not ordinary VIEW coordinates.
    ScreenMidpoint {
        slot: u16,
    },
}

/// A reached native edge could not authenticate its projection inputs.
/// This receipt survives independently of geometry: dropping the failed edge
/// must not lose its diagnostic or resolve an unread endpoint during remapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelNativeEdgeFailure {
    pub source_edge_index: usize,
    pub source_slots: [u16; 2],
    pub boundary: ModelNativeEdgeBoundary,
}

/// The first reached dependency that lacks source-owned native VIEW coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModelNativeVertexBoundary {
    SlotUnavailable { slot: u16 },
    ViewUnavailable { slot: u16, type_flag: i16 },
    ScreenMidpoint { slot: u16 },
    UnsupportedGenerator { slot: u16, type_flag: i16 },
    ExternalFrameUnavailable { slot: u16 },
}

/// A world tf12/tf13 consumer was skipped because its native source is unowned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModelNativeVertexFailure {
    pub consumer_slot: u16,
    pub source_slot: u16,
    pub boundary: ModelNativeVertexBoundary,
}

/// An owned 0x22 constructor stops before allocating any primitive when a
/// reached endpoint carries bit0x40. This is a source rejection, not missing
/// endpoint custody; the unvisited endpoint must not be synthesized or replayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelEdgeConstructorClip {
    SurfaceBand,
    NearView { depth_raw: i32 },
}

/// Per-command projection custody. Compatibility is a genuine producer
/// mode; an owned native producer never falls back when an endpoint is missing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModelEdgeEndpointSnapshot {
    #[default]
    Compatibility,
    Native {
        endpoints: [ModelNativeEdgeEndpoint; 2],
    },
    SourceClipped {
        slot: u16,
        reason: ModelEdgeConstructorClip,
    },
    MissingNative {
        boundary: ModelNativeEdgeBoundary,
    },
}

/// Parent vertices imported by tf11. The outer index is the op-0x0E remap
/// table index; each pair contains the even/odd variants already converted to
/// child raw-local coordinates. None means an unavailable source, not clipping.
pub type LinkedModelSlots = Vec<[Option<ResolvedModelSlot>; 2]>;

/// Context-sensitive vertex families installed by the active render table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelVertexKind {
    Alias,
    ViewPin,
}

/// Presentation-owned replacement for tf12/tf13. Both input and output are
/// raw-local coordinates. The source has resolved its generators and mirror
/// bit; each callback computes a fresh clip state from those coordinates.
/// Intrinsic/collision materialization supplies no callback.
pub trait ModelVertexResolver {
    fn resolve_vertex_raw(
        &self,
        kind: ModelVertexKind,
        source: ResolvedModelSlot,
    ) -> Option<ResolvedModelSlot>;

    /// An installed live external owner admits its referenced slots only after
    /// the authored normal test and before corner projection (45A4E0).
    fn owns_external_frame_slots(&self) -> bool {
        false
    }

    /// A source-owned current-node VIEW frame is installed. This declaration
    /// distinguishes a missing native generator from a compatibility producer.
    fn owns_native_view(&self) -> bool {
        false
    }

    /// Exact current-node VIEW origin for the reached 66 queue constructor.
    /// An owned frame with a missing origin must never use a float pose.
    fn native_node_origin_view(&self) -> Option<[i32; 3]> {
        None
    }

    /// Claimed-native submissions fail closed at the first unowned endpoint.
    /// A deliberate scene adapter may retain compatibility geometry and its
    /// callback traversal even when this producer also carries native VIEW.
    fn requires_native_edge_endpoints(&self) -> bool {
        self.owns_native_view()
    }

    fn admit_face_raw(&self, _anchor: [i32; 3], _normal: [i32; 3]) -> bool {
        true
    }

    /// Decorate a reached plain or external leaf with source VIEW words at the
    /// interpreter's actual register snapshot. Generated points use the resolved
    /// dependencies below, so callbacks and linked caches are never re-traversed.
    fn resolve_native_view_point(&self, _slot: u16, _vars: &AnimVars) -> Option<[i32; 3]> {
        None
    }

    /// Evaluate spatial arithmetic from the canonical walker's completed VIEW
    /// caches. Local coordinates are retained separately for intrinsic consumers.
    fn resolve_native_spatial_point(
        &self,
        _generator: ModelNativeSpatialGenerator,
    ) -> Option<[i32; 3]> {
        None
    }

    fn report_native_vertex_failure(&self, _failure: ModelNativeVertexFailure) {}

    /// Optional owner of tf14's authored callback parameters. Ordinary intrinsic
    /// decoding retains its raw-parameter fallback when unimplemented.
    fn resolve_external_frame_raw(
        &self,
        _slot: u16,
        _parameters: [i16; 3],
    ) -> Option<ResolvedModelSlot> {
        None
    }
}

// ── Slot resolution (type_flag transform table, 0x4D4A78) ──────────────────

/// Resolve slot refs to model-space positions per the type_flag transform
/// table. Cycles are broken with a busy set, so a self-referential generator
/// resolves to `None` instead of recursing forever.
pub(super) struct SlotResolver<'a> {
    pub(super) records: &'a [[i16; 4]],
    pub(super) vars: &'a AnimVars,
    pub(super) linked: Option<&'a LinkedModelSlots>,
    pub(super) vertex_resolver: Option<&'a dyn ModelVertexResolver>,
    pub(super) cache: HashMap<u16, Option<ResolvedModelSlot>>,
    pub(super) busy: HashSet<u16>,
}

impl<'a> SlotResolver<'a> {
    #[cfg(test)]
    pub(super) fn new(records: &'a [[i16; 4]], vars: &'a AnimVars) -> Self {
        Self::with_linked(records, vars, None)
    }

    pub(super) fn with_linked(
        records: &'a [[i16; 4]],
        vars: &'a AnimVars,
        linked: Option<&'a LinkedModelSlots>,
    ) -> Self {
        Self {
            records,
            vars,
            linked,
            vertex_resolver: None,
            cache: HashMap::new(),
            busy: HashSet::new(),
        }
    }

    pub(super) fn with_vertex_resolver(
        mut self,
        resolver: Option<&'a dyn ModelVertexResolver>,
    ) -> Self {
        self.vertex_resolver = resolver;
        self
    }

    pub(super) fn resolve(
        &mut self,
        slot: u16,
        stats: &mut StreamStats,
    ) -> Option<ResolvedModelSlot> {
        if let Some(v) = self.cache.get(&slot) {
            return *v;
        }
        let vi = (slot >> 1) as usize;
        if vi >= self.records.len() || self.busy.contains(&slot) {
            return None;
        }
        self.busy.insert(slot);
        let r = self.records[vi];
        let v = self.compute(slot, r[0], r[1], r[2], r[3], stats);
        self.busy.remove(&slot);
        let v = v.map(|mut value| {
            if value.native_view_point.is_none() && matches!(r[0], 0 | 4 | 14) {
                value.native_view_point = self
                    .vertex_resolver
                    .and_then(|owner| owner.resolve_native_view_point(slot, self.vars));
            }
            value
        });
        self.cache.insert(slot, v);
        v
    }

    pub(super) fn live_face_admitted(&self, nref: u16, normals: &[[i16; 4]]) -> bool {
        let Some(owner) = self
            .vertex_resolver
            .filter(|owner| owner.owns_external_frame_slots())
        else {
            return true;
        };
        let Some(index) = (nref >> 1).checked_sub(1).map(usize::from) else {
            return true;
        };
        let Some(normal) = normals.get(index) else {
            return false;
        };
        let slot = normal[0] as u16 ^ (nref & 1);
        let Some(anchor) = self.records.get(usize::from(slot >> 1)) else {
            return false;
        };
        let mirror = if slot & 1 == 0 { 1 } else { -1 };
        owner.admit_face_raw(
            [
                i32::from(anchor[1]) * mirror,
                i32::from(anchor[2]),
                i32::from(anchor[3]),
            ],
            [
                i32::from(normal[1]) * if nref & 1 == 0 { 1 } else { -1 },
                i32::from(normal[2]),
                i32::from(normal[3]),
            ],
        )
    }

    /// Read a source's coordinates without inheriting its clip state. Refs
    /// are XOR'd with the requesting slot's mirror bit before resolution.
    fn rref(&mut self, r: i32, odd: u16, stats: &mut StreamStats) -> Option<[f64; 3]> {
        self.resolve(((r as u32 & 0xFFFF) as u16) ^ odd, stats)
            .map(|slot| slot.position_raw)
    }

    fn spatial_slot(
        &self,
        position: [f64; 3],
        generator: Option<ModelNativeSpatialGenerator>,
    ) -> ResolvedModelSlot {
        let mut result = ResolvedModelSlot::clear(position);
        result.native_view_point = generator.and_then(|generator| {
            self.vertex_resolver?
                .resolve_native_spatial_point(generator)
        });
        result
    }

    /// Diagnose only already-reached dependencies: this must never execute a
    /// provider, resolve an unvisited corner, or replay the source graph.
    fn native_boundary(&self, slot: u16, seen: &mut HashSet<u16>) -> ModelNativeVertexBoundary {
        use ModelNativeVertexBoundary as Boundary;
        if !seen.insert(slot) {
            return Boundary::SlotUnavailable { slot };
        }
        let Some(&[tf, a, b, c]) = self.records.get(usize::from(slot >> 1)) else {
            return Boundary::SlotUnavailable { slot };
        };
        match tf {
            1 => return Boundary::ScreenMidpoint { slot },
            3 | 10 => {
                return Boundary::UnsupportedGenerator {
                    slot,
                    type_flag: tf,
                }
            }
            14 => return Boundary::ExternalFrameUnavailable { slot },
            0 | 2 | 4..=9 | 11..=13 => {}
            _ => {
                return Boundary::UnsupportedGenerator {
                    slot,
                    type_flag: tf,
                }
            }
        }
        let refs: Vec<u16> = match tf {
            2 => vec![c as u16],
            5 | 7 | 8 => vec![b as u16, c as u16],
            6 => vec![b as u16, c as u16, a as u16],
            9 => vec![
                b as u16,
                c as u16,
                (c as u16).wrapping_add(2),
                (b as u16).wrapping_add(2),
            ],
            12 | 13 => vec![a as u16],
            _ => vec![],
        };
        for source in refs {
            let source = source ^ (slot & 1);
            if self
                .cache
                .get(&source)
                .and_then(|value| *value)
                .and_then(|value| value.native_view_point)
                .is_none()
            {
                return self.native_boundary(source, seen);
            }
        }
        if self.cache.get(&slot).is_none_or(Option::is_none) {
            Boundary::SlotUnavailable { slot }
        } else {
            Boundary::ViewUnavailable {
                slot,
                type_flag: tf,
            }
        }
    }

    fn compute(
        &mut self,
        slot: u16,
        tf: i16,
        a: i16,
        b: i16,
        c: i16,
        stats: &mut StreamStats,
    ) -> Option<ResolvedModelSlot> {
        let odd = slot & 1;
        let count = |stats: &mut StreamStats, name: &'static str| {
            *stats.generated.entry(name).or_insert(0) += 1;
        };

        let position =
            match tf {
                // Plain vertex; mirror slot negates X.
                0 => Some([
                    if odd != 0 { -(a as f64) } else { a as f64 },
                    b as f64,
                    c as f64,
                ]),
                // Plain vertex; mirror slot pinned to the symmetry plane (X=0).
                4 => Some([if odd != 0 { 0.0 } else { a as f64 }, b as f64, c as f64]),
                // 46DC00 preserves source 0x40 rejection for screen midpoints.
                // The XYZ midpoint remains the existing diagnostic proxy; actual
                // screen coordinates/min depth use ModelVertexProjection.
                1 => {
                    count(stats, "midpoint");
                    let va = self.resolve(b as u16 ^ odd, stats)?;
                    let vb = self.resolve(c as u16 ^ odd, stats)?;
                    return Some(ResolvedModelSlot {
                        world_point: None,
                        native_view_point: None,
                        position_raw: std::array::from_fn(|axis| {
                            (va.position_raw[axis] + vb.position_raw[axis]) / 2.0
                        }),
                        clip: if va.clip == ModelSlotClip::SurfaceBand
                            || vb.clip == ModelSlotClip::SurfaceBand
                        {
                            ModelSlotClip::SurfaceBand
                        } else {
                            ModelSlotClip::Clear
                        },
                        surface_origin: ModelSurfaceOrigin::None,
                    });
                }
                // 46E760 consumes completed VIEW caches and writes fresh clip flags.
                5 => {
                    count(stats, "midpoint");
                    let first = self.resolve(b as u16 ^ odd, stats)?;
                    let second = self.resolve(c as u16 ^ odd, stats)?;
                    return Some(self.spatial_slot(
                        std::array::from_fn(|axis| {
                            (first.position_raw[axis] + second.position_raw[axis]) / 2.0
                        }),
                        first.native_view_point.zip(second.native_view_point).map(
                            |(first, second)| ModelNativeSpatialGenerator::Midpoint {
                                first,
                                second,
                            },
                        ),
                    ));
                }
                // 46DDB0 reflects the source through the current node VIEW origin.
                2 => {
                    count(stats, "point_reflect");
                    let source = self.resolve(c as u16 ^ odd, stats)?;
                    return Some(
                        self.spatial_slot(
                            source.position_raw.map(|value| -value),
                            source
                                .native_view_point
                                .map(|source| ModelNativeSpatialGenerator::Reflection { source }),
                        ),
                    );
                }
                // Slot c + per-frame random jitter; static base position.
                3 => {
                    count(stats, "jitter");
                    self.rref(c as i32, odd, stats)
                }
                // 46F1B0 visits B,C,A, then completes A+B-C in saved VIEW words.
                6 => {
                    count(stats, "parallelogram");
                    let vb = self.resolve(b as u16 ^ odd, stats)?;
                    let vc = self.resolve(c as u16 ^ odd, stats)?;
                    let va = self.resolve(a as u16 ^ odd, stats)?;
                    let native = (|| {
                        Some(ModelNativeSpatialGenerator::Parallelogram {
                            a: va.native_view_point?,
                            b: vb.native_view_point?,
                            c: vc.native_view_point?,
                        })
                    })();
                    return Some(self.spatial_slot(
                        std::array::from_fn(|axis| {
                            va.position_raw[axis] + vb.position_raw[axis] - vc.position_raw[axis]
                        }),
                        native,
                    ));
                }
                // 46F530 subtracts the current node VIEW origin after B+C.
                7 => {
                    count(stats, "vector_sum");
                    let first = self.resolve(b as u16 ^ odd, stats)?;
                    let second = self.resolve(c as u16 ^ odd, stats)?;
                    return Some(self.spatial_slot(
                        std::array::from_fn(|axis| {
                            first.position_raw[axis] + second.position_raw[axis]
                        }),
                        first.native_view_point.zip(second.native_view_point).map(
                            |(first, second)| ModelNativeSpatialGenerator::VectorSum {
                                first,
                                second,
                            },
                        ),
                    ));
                }
                // 46F7D0 consumes completed VIEW caches at the reached register state.
                8 => {
                    count(stats, "lerp");
                    let first = self.resolve(b as u16 ^ odd, stats)?;
                    let second = self.resolve(c as u16 ^ odd, stats)?;
                    let phase = eval_var(a as i32, self.vars);
                    return Some(self.spatial_slot(
                        std::array::from_fn(|axis| {
                            retail_tf8_axis(
                                first.position_raw[axis],
                                second.position_raw[axis],
                                phase,
                            )
                        }),
                        first.native_view_point.zip(second.native_view_point).map(
                            |(first, second)| ModelNativeSpatialGenerator::Lerp {
                                first,
                                second,
                                phase,
                            },
                        ),
                    ));
                }
                // 46FB80 visits B,C,C+2,B+2, then evaluates the current phase.
                9 => {
                    count(stats, "bezier");
                    let points = [
                        self.resolve(b as u16 ^ odd, stats),
                        self.resolve(c as u16 ^ odd, stats),
                        self.resolve((c as u16).wrapping_add(2) ^ odd, stats),
                        self.resolve((b as u16).wrapping_add(2) ^ odd, stats),
                    ];
                    let [Some(p0), Some(p1), Some(p2), Some(p3)] = points else {
                        // Intrinsic diagnostic fallback only: no native cache is
                        // borrowed from P0 for an incomplete control-point graph.
                        return points[0].map(|point| ResolvedModelSlot::clear(point.position_raw));
                    };
                    let phase = eval_var(a as i32, self.vars);
                    let native = (|| {
                        Some(ModelNativeSpatialGenerator::Bezier {
                            points: [
                                p0.native_view_point?,
                                p1.native_view_point?,
                                p2.native_view_point?,
                                p3.native_view_point?,
                            ],
                            phase,
                        })
                    })();
                    return Some(self.spatial_slot(
                        std::array::from_fn(|axis| {
                            retail_tf9_axis(
                                p0.position_raw[axis],
                                p1.position_raw[axis],
                                p2.position_raw[axis],
                                p3.position_raw[axis],
                                phase,
                            )
                        }),
                        native,
                    ));
                }
                // Track follower: re-evaluate the generator record of slot
                // (a & 0xFF); record types 0x13/0x14 are linear tracks
                // (statically: their midpoint).
                10 => {
                    count(stats, "track_follow");
                    let target = ((a as i32 & 0xFF) ^ odd as i32) as u16;
                    let vi = (target >> 1) as usize;
                    if vi >= self.records.len() {
                        return None;
                    }
                    let r = self.records[vi];
                    if r[0] == 0x13 || r[0] == 0x14 {
                        let va = self.rref(r[2] as i32, odd, stats)?;
                        let vb = self.rref(r[3] as i32, odd, stats)?;
                        return Some(ResolvedModelSlot::clear([
                            (va[0] + vb[0]) / 2.0,
                            (va[1] + vb[1]) / 2.0,
                            (va[2] + vb[2]) / 2.0,
                        ]));
                    }
                    self.resolve(target, stats).map(|slot| slot.position_raw)
                }
                // Vertex imported from the linked model at ctx+0xC0 (set at runtime
                // by op 0x0E instancing). Without a linked context, return None so
                // the face is DROPPED rather than spiking to the origin — matching
                // the static v3 reference extraction.
                11 => {
                    count(stats, "linked_model");
                    let index = a;
                    if index < 0 {
                        return None;
                    }
                    return self
                        .linked
                        .and_then(|linked| linked.get(index as usize))
                        .and_then(|pair| pair[odd as usize]);
                }
                // World tf12 and tf13 consume resolved source XYZ but replace
                // its clip state (4340B0/4349C0). Intrinsic decoding keeps XYZ.
                12 | 13 => {
                    let kind = if tf == 12 {
                        count(stats, "alias");
                        ModelVertexKind::Alias
                    } else {
                        count(stats, "sky_pin");
                        ModelVertexKind::ViewPin
                    };
                    let source_slot = a as u16 ^ odd;
                    let source = self.resolve(source_slot, stats);
                    if let Some(owner) = self
                        .vertex_resolver
                        .filter(|owner| owner.owns_native_view())
                    {
                        if source.and_then(|value| value.native_view_point).is_none() {
                            owner.report_native_vertex_failure(ModelNativeVertexFailure {
                                consumer_slot: slot,
                                source_slot,
                                boundary: self.native_boundary(source_slot, &mut HashSet::new()),
                            });
                            return None;
                        }
                    }
                    let source = source?;
                    return match self.vertex_resolver {
                        Some(resolver) => resolver.resolve_vertex_raw(kind, source),
                        None => Some(ResolvedModelSlot::clear(source.position_raw)),
                    };
                }
                // Vertex in an external attachment frame (callback ctx+0x5C);
                // record coords are coordinates in that frame.
                14 => {
                    count(stats, "external_frame");
                    if let Some(resolver) = self.vertex_resolver {
                        let point = resolver.resolve_external_frame_raw(slot, [a, b, c]);
                        if point.is_some() || resolver.owns_external_frame_slots() {
                            return point;
                        }
                    }
                    Some([a as f64, b as f64, c as f64])
                }
                _ => {
                    *stats.unknown_type_flags.entry(tf).or_insert(0) += 1;
                    None
                }
            };
        // Arithmetic families (including tf10) consume coordinates, then
        // recompute their own clip byte. Tf11 copies the full slot; tf1 has
        // its separate source-rejection policy above.
        position.map(ResolvedModelSlot::clear)
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod native_tests;
