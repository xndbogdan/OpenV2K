//! Integration custody: native generators consume the canonical dependency walk.
use std::cell::RefCell;

use super::*;

#[derive(Default)]
struct NativeOwner {
    external_calls: RefCell<Vec<u16>>,
    leaf_calls: RefCell<Vec<u16>>,
    surface_calls: RefCell<Vec<[i32; 3]>>,
    failures: RefCell<Vec<ModelNativeVertexFailure>>,
}

fn external_point(slot: u16) -> [i32; 3] {
    let record = i32::from(slot >> 1);
    [
        (3 + record) * if slot & 1 == 0 { 1 } else { -1 },
        10 - record,
        100 + record,
    ]
}

impl ModelVertexResolver for NativeOwner {
    fn resolve_vertex_raw(
        &self,
        _: ModelVertexKind,
        source: ResolvedModelSlot,
    ) -> Option<ResolvedModelSlot> {
        let view = source.native_view_point?;
        self.surface_calls.borrow_mut().push(view);
        let mut result = ResolvedModelSlot::clear([999.0, -777.0, 555.0]);
        result.native_view_point = Some([view[0], view[1] - 137, view[2]]);
        result.world_point = Some([20.0, -30.0, 40.0]);
        result.clip = ModelSlotClip::SurfaceBand;
        result.surface_origin = ModelSurfaceOrigin::ViewPin;
        Some(result)
    }
    fn owns_native_view(&self) -> bool {
        true
    }
    fn owns_external_frame_slots(&self) -> bool {
        true
    }
    fn resolve_external_frame_raw(
        &self,
        slot: u16,
        parameters: [i16; 3],
    ) -> Option<ResolvedModelSlot> {
        self.external_calls.borrow_mut().push(slot);
        let mut result = ResolvedModelSlot::clear(parameters.map(f64::from));
        result.native_view_point = Some(external_point(slot));
        result.world_point = Some([1.0, 2.0, 3.0]);
        result.clip = ModelSlotClip::SurfaceBand;
        Some(result)
    }
    fn resolve_native_view_point(&self, slot: u16, _: &AnimVars) -> Option<[i32; 3]> {
        self.leaf_calls.borrow_mut().push(slot);
        Some([100 + i32::from(slot), 204, 404])
    }
    fn resolve_native_spatial_point(
        &self,
        generator: ModelNativeSpatialGenerator,
    ) -> Option<[i32; 3]> {
        Some(generator.evaluate([7, 8, 9]))
    }
    fn report_native_vertex_failure(&self, failure: ModelNativeVertexFailure) {
        self.failures.borrow_mut().push(failure);
    }
}

#[test]
fn native_midpoint_uses_projected_and_external_dependency_caches_once() {
    let records = [
        [14, 17, 18, 19],
        [13, 0, 0, 0],
        [0, 10, 20, 30],
        [5, 0, 2, 4],
        [13, 6, 0, 0],
    ];
    let vars = AnimVars::default();
    let owner = NativeOwner::default();
    let mut slots =
        SlotResolver::with_linked(&records, &vars, None).with_vertex_resolver(Some(&owner));
    let mut stats = StreamStats::default();
    let generated = slots.resolve(6, &mut stats).unwrap();
    assert_eq!(generated.native_view_point, Some([53, 38, 252]));
    assert_eq!(generated.clip, ModelSlotClip::Clear);
    assert_eq!(generated.world_point, None);
    assert_eq!(generated.surface_origin, ModelSurfaceOrigin::None);
    let consumer = slots.resolve(8, &mut stats).unwrap();
    assert_eq!(consumer.native_view_point, Some([53, -99, 252]));
    assert_eq!(*owner.surface_calls.borrow(), [[3, 10, 100], [53, 38, 252]]);
    assert_eq!(*owner.external_calls.borrow(), [0]);
    assert_eq!(*owner.leaf_calls.borrow(), [4]);
    assert!(owner.failures.borrow().is_empty());
}

#[test]
fn every_native_spatial_family_consumes_owned_external_caches_and_mirrors() {
    use ModelNativeSpatialGenerator as G;
    let cases = [
        ([2, 0, 0, 0], 2),
        ([5, 0, 0, 4], 5),
        ([6, 2, 0, 4], 6),
        ([7, 0, 0, 4], 7),
        ([8, 0x60, 0, 4], 8),
        ([9, 0x60, 0, 4], 9),
    ];
    for (record, kind) in cases {
        for odd in [0, 1] {
            let records = [
                [14, 900, 800, 700],
                [14, -90, -80, -70],
                [14, 90, 80, 70],
                [14, 700, 800, 900],
                record,
            ];
            let vars = AnimVars::default();
            let owner = NativeOwner::default();
            let mut slots =
                SlotResolver::with_linked(&records, &vars, None).with_vertex_resolver(Some(&owner));
            let mut stats = StreamStats::default();
            let result = slots.resolve(8 ^ odd, &mut stats).unwrap();
            let first = external_point(odd);
            let second = external_point(4 ^ odd);
            let generator = match kind {
                2 => G::Reflection { source: first },
                5 => G::Midpoint { first, second },
                6 => G::Parallelogram {
                    a: external_point(2 ^ odd),
                    b: first,
                    c: second,
                },
                7 => G::VectorSum { first, second },
                8 => G::Lerp {
                    first,
                    second,
                    phase: 0x8000,
                },
                9 => G::Bezier {
                    points: [
                        first,
                        second,
                        external_point(6 ^ odd),
                        external_point(2 ^ odd),
                    ],
                    phase: 0x8000,
                },
                _ => unreachable!(),
            };
            assert_eq!(
                result.native_view_point,
                Some(generator.evaluate([7, 8, 9])),
                "tf{kind} mirror{odd}"
            );
            assert_eq!(result.clip, ModelSlotClip::Clear);
            assert_eq!(result.world_point, None);
            assert!(
                owner.leaf_calls.borrow().is_empty(),
                "external callbacks already supplied VIEW"
            );
            let expected: Vec<_> = match kind {
                2 => vec![0],
                6 => vec![0, 4, 2],
                9 => vec![0, 4, 6, 2],
                _ => vec![0, 4],
            }
            .into_iter()
            .map(|slot| slot ^ odd)
            .collect();
            assert_eq!(
                *owner.external_calls.borrow(),
                expected,
                "retail dependency dispatch order"
            );
        }
    }
}

#[test]
fn unsupported_native_dependency_reports_first_reached_boundary_without_callback_replay() {
    use ModelNativeVertexBoundary as B;
    for (unsupported, expected) in [
        (
            [3, 0, 0, 0],
            B::UnsupportedGenerator {
                slot: 2,
                type_flag: 3,
            },
        ),
        (
            [99, 0, 0, 0],
            B::UnsupportedGenerator {
                slot: 2,
                type_flag: 99,
            },
        ),
        ([1, 0, 0, 0], B::ScreenMidpoint { slot: 2 }),
        (
            [10, 0, 0, 0],
            B::UnsupportedGenerator {
                slot: 2,
                type_flag: 10,
            },
        ),
        ([5, 0, 0, 99], B::SlotUnavailable { slot: 99 }),
        ([5, 0, 2, 0], B::SlotUnavailable { slot: 2 }),
    ] {
        let records = [[14, 1, 2, 3], unsupported, [5, 0, 2, 0], [13, 4, 0, 0]];
        let vars = AnimVars::default();
        let owner = NativeOwner::default();
        let mut slots =
            SlotResolver::with_linked(&records, &vars, None).with_vertex_resolver(Some(&owner));
        assert!(slots.resolve(6, &mut StreamStats::default()).is_none());
        assert_eq!(
            *owner.failures.borrow(),
            [ModelNativeVertexFailure {
                consumer_slot: 6,
                source_slot: 4,
                boundary: expected
            }]
        );
        assert!(owner.surface_calls.borrow().is_empty());
        assert!(
            owner.external_calls.borrow().len() <= 1,
            "diagnostic must not revisit a provider"
        );
    }
}

#[test]
fn incomplete_native_bezier_never_borrows_intrinsic_p0_fallback_cache() {
    let records = [[14, 1, 2, 3], [9, 0x60, 0, 99], [13, 2, 0, 0]];
    let vars = AnimVars::default();
    let owner = NativeOwner::default();
    let mut slots =
        SlotResolver::with_linked(&records, &vars, None).with_vertex_resolver(Some(&owner));
    let diagnostic = slots.resolve(2, &mut StreamStats::default()).unwrap();
    assert_eq!(diagnostic.position_raw, [1.0, 2.0, 3.0]);
    assert_eq!(diagnostic.native_view_point, None);
    assert!(slots.resolve(4, &mut StreamStats::default()).is_none());
    assert_eq!(
        owner.failures.borrow()[0].boundary,
        ModelNativeVertexBoundary::SlotUnavailable { slot: 99 }
    );
    assert!(owner.surface_calls.borrow().is_empty());
}
