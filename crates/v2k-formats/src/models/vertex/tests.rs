use std::cell::RefCell;

use super::*;
use crate::models::{
    ModelEntry, ModelFaceVertices, ModelMaterializationContext, ModelVertexProjection,
};

struct SurfaceFixture {
    view_pin_clip: ModelSlotClip,
    sources: RefCell<Vec<(ModelVertexKind, [f64; 3])>>,
}

impl SurfaceFixture {
    fn new(view_pin_clip: ModelSlotClip) -> Self {
        Self {
            view_pin_clip,
            sources: RefCell::new(Vec::new()),
        }
    }
}

impl ModelVertexResolver for SurfaceFixture {
    fn resolve_vertex_raw(
        &self,
        kind: ModelVertexKind,
        source: ResolvedModelSlot,
    ) -> Option<ResolvedModelSlot> {
        let source_raw = source.position_raw;
        self.sources.borrow_mut().push((kind, source_raw));
        Some(match kind {
            ModelVertexKind::Alias => {
                ResolvedModelSlot::clear([source_raw[0], source_raw[0] / 4.0, source_raw[2]])
            }
            ModelVertexKind::ViewPin => ResolvedModelSlot {
                world_point: None,
                native_view_point: None,
                // A controlled surface callback, independent of actual terrain
                // sampling. Retail's rejected branch retains source XYZ.
                position_raw: match self.view_pin_clip {
                    ModelSlotClip::Clear => [
                        source_raw[0] + source_raw[1] / 4.0,
                        0.0,
                        source_raw[2] + source_raw[1] / 4.0,
                    ],
                    ModelSlotClip::SurfaceBand => source_raw,
                },
                clip: self.view_pin_clip,
                surface_origin: ModelSurfaceOrigin::ViewPin,
            },
        })
    }
}

#[test]
fn contextual_projection_precedes_generators_and_samples_each_mirror() {
    let vars = AnimVars::default();
    let surface = SurfaceFixture::new(ModelSlotClip::Clear);
    let model = ModelEntry {
        records: vec![
            [0, 256, 512, 128], // 0/1: source
            [12, 0, 0, 0],      // 2/3: alias; mirrored terrain samples differ
            [13, 2, 0, 0],      // 4/5: projected endpoint
            [0, 0, 128, 0],     // 6/7: independent displacement
            [6, 4, 6, 0],       // 8/9: projected endpoint + displacement - source
        ],
        ..ModelEntry::default()
    };
    let intrinsic = ModelMaterializationContext::intrinsic(&vars, None);
    let context = ModelMaterializationContext {
        vertex_resolver: Some(&surface),
        ..intrinsic
    };

    assert_eq!(
        model.resolve_slot_with_context(8, context),
        Some(ResolvedModelSlot::clear([16.0, -384.0, 16.0]))
    );
    assert_eq!(
        model.resolve_slot_with_context(9, context),
        Some(ResolvedModelSlot::clear([-16.0, -384.0, -16.0]))
    );
    assert_eq!(
        *surface.sources.borrow(),
        [
            (ModelVertexKind::Alias, [256.0, 512.0, 128.0]),
            (ModelVertexKind::ViewPin, [256.0, 64.0, 128.0]),
            (ModelVertexKind::Alias, [-256.0, 512.0, 128.0]),
            (ModelVertexKind::ViewPin, [-256.0, -64.0, 128.0]),
        ]
    );
    assert_eq!(
        model.resolve_slot_with_context(8, intrinsic),
        Some(ResolvedModelSlot::clear([0.0, 128.0, 0.0]))
    );
}

#[test]
fn linked_import_preserves_rejected_xyz_and_tf6_can_consume_it() {
    let vars = AnimVars::default();
    let surface = SurfaceFixture::new(ModelSlotClip::SurfaceBand);
    let parent = ModelEntry {
        records: vec![[0, 40, 96, 80], [13, 0, 0, 0]],
        ..ModelEntry::default()
    };
    let parent_context = ModelMaterializationContext {
        vertex_resolver: Some(&surface),
        ..ModelMaterializationContext::intrinsic(&vars, None)
    };
    let parent_slots = [
        parent.resolve_slot_with_context(2, parent_context),
        parent.resolve_slot_with_context(3, parent_context),
    ];
    assert_eq!(parent_slots[0].unwrap().position_raw, [40.0, 96.0, 80.0]);
    assert_eq!(parent_slots[1].unwrap().position_raw, [-40.0, 96.0, 80.0]);
    assert!(parent_slots
        .iter()
        .all(|slot| slot.unwrap().clip == ModelSlotClip::SurfaceBand));
    assert!(parent_slots
        .iter()
        .all(|slot| slot.unwrap().surface_origin == ModelSurfaceOrigin::ViewPin));

    let linked = vec![parent_slots];
    let child = ModelEntry {
        records: vec![
            [11, 0, 0, 0],
            [0, 16, 32, 48],
            [0, 0, 0, 0],
            [6, 0, 2, 4],
            [12, 0, 0, 0],
            [13, 0, 0, 0],
        ],
        ..ModelEntry::default()
    };
    let context = ModelMaterializationContext::intrinsic(&vars, Some(&linked));
    assert_eq!(child.resolve_slot_with_context(0, context), parent_slots[0]);
    assert_eq!(child.resolve_slot_with_context(1, context), parent_slots[1]);
    assert_eq!(
        child.resolve_slot_with_context(6, context),
        Some(ResolvedModelSlot::clear([56.0, 128.0, 128.0]))
    );
    // The alias/default projection reads XYZ without copying source clipping.
    for slot in [8, 10] {
        assert_eq!(
            child.resolve_slot_with_context(slot, context),
            Some(ResolvedModelSlot::clear([40.0, 96.0, 80.0]))
        );
    }
    let fresh_surface = SurfaceFixture::new(ModelSlotClip::Clear);
    let projected = ModelMaterializationContext {
        vertex_resolver: Some(&fresh_surface),
        ..context
    };
    assert_eq!(
        child.resolve_slot_with_context(10, projected),
        Some(ResolvedModelSlot {
            world_point: None,
            native_view_point: None,
            position_raw: [64.0, 0.0, 104.0],
            clip: ModelSlotClip::Clear,
            surface_origin: ModelSurfaceOrigin::ViewPin,
        })
    );
    assert_eq!(
        child.resolve_slot_with_context(8, projected),
        Some(ResolvedModelSlot::clear([40.0, 10.0, 80.0]))
    );
}

#[test]
fn arithmetic_generators_reset_rejection_but_screen_midpoint_preserves_it() {
    let mut vars = AnimVars::default();
    vars.registers[0] = 0x8000;
    let linked = [
        [8.0, 16.0, 24.0],
        [24.0, 32.0, 40.0],
        [40.0, 48.0, 56.0],
        [56.0, 64.0, 72.0],
    ]
    .map(|position_raw| {
        [Some(ResolvedModelSlot {
            world_point: None,
            native_view_point: None,
            position_raw,
            clip: ModelSlotClip::SurfaceBand,
            surface_origin: ModelSurfaceOrigin::ViewPin,
        }); 2]
    })
    .to_vec();
    for (generator, expected) in [
        ([2, 0, 0, 0], [-8.0, -16.0, -24.0]),
        ([3, 0, 0, 0], [8.0, 16.0, 24.0]),
        ([5, 0, 0, 2], [16.0, 24.0, 32.0]),
        ([6, 0, 2, 4], [-8.0, 0.0, 8.0]),
        ([7, 0, 0, 2], [32.0, 48.0, 64.0]),
        ([8, 0xC0, 0, 2], [16.0, 24.0, 32.0]),
        // The cubic's last negative half-step truncates to an eight-unit
        // contribution (46FB80), rather than the ideal floating midpoint.
        ([9, 0xC0, 0, 4], [36.0, 44.0, 52.0]),
    ] {
        let records = [
            [11, 0, 0, 0],
            [11, 1, 0, 0],
            [11, 2, 0, 0],
            [11, 3, 0, 0],
            generator,
        ];
        let mut resolver = SlotResolver::with_linked(&records, &vars, Some(&linked));
        assert_eq!(
            resolver.resolve(8, &mut StreamStats::default()),
            Some(ResolvedModelSlot::clear(expected)),
            "generator tf{} must consume clipped XYZ and write its own flags",
            generator[0]
        );
    }
    let records = [
        [11, 0, 0, 0],
        [11, 1, 0, 0],
        [0x13, 0xC0, 0, 2],
        [10, 4, 0, 0],
        [1, 0, 0, 2],
    ];
    let mut resolver = SlotResolver::with_linked(&records, &vars, Some(&linked));
    assert_eq!(
        resolver.resolve(6, &mut StreamStats::default()),
        Some(ResolvedModelSlot::clear([16.0, 24.0, 32.0]))
    );
    assert_eq!(
        resolver
            .resolve(8, &mut StreamStats::default())
            .unwrap()
            .clip,
        ModelSlotClip::SurfaceBand
    );
    assert_eq!(
        resolver
            .resolve(8, &mut StreamStats::default())
            .unwrap()
            .surface_origin,
        ModelSurfaceOrigin::None
    );
}

#[test]
fn surface_rejection_retains_the_whole_quad_and_parallel_vertex_metadata() {
    let vars = AnimVars::default();
    let surface = SurfaceFixture::new(ModelSlotClip::SurfaceBand);
    let model = ModelEntry {
        records: vec![[0, 0, 0, 0], [0, 64, 0, 0], [0, 64, 96, 0], [13, 4, 0, 0]],
        cmd_words: vec![0x84, 17, 0, 0, 2, 4, 6, 0],
        ..ModelEntry::default()
    };
    let context = ModelMaterializationContext {
        vertex_resolver: Some(&surface),
        ..ModelMaterializationContext::intrinsic(&vars, None)
    };
    let materialized = model.materialize_with_context(context);
    assert_eq!(materialized.triangles.len(), 2);
    assert_eq!(
        materialized.face_vertices,
        [ModelFaceVertices::Quad([0, 1, 2, 3]); 2]
    );
    assert_eq!(materialized.vertex_type_flags, [0, 0, 0, 13]);
    assert_eq!(
        materialized.vertex_projection,
        [ModelVertexProjection::Position; 4]
    );
    assert_eq!(
        materialized.vertex_clip,
        [
            ModelSlotClip::Clear,
            ModelSlotClip::Clear,
            ModelSlotClip::Clear,
            ModelSlotClip::SurfaceBand
        ]
    );
    assert_eq!(
        materialized.vertex_surface_origin,
        [
            ModelSurfaceOrigin::None,
            ModelSurfaceOrigin::None,
            ModelSurfaceOrigin::None,
            ModelSurfaceOrigin::ViewPin
        ]
    );
    assert_eq!(materialized.vertices[3], [64.0, 96.0, 0.0]);
    assert_eq!(materialized.face_uvs, model.materialize(&vars).face_uvs);
    assert!(model
        .materialize(&vars)
        .vertex_clip
        .iter()
        .all(|clip| *clip == ModelSlotClip::Clear));
    assert!(model
        .materialize(&vars)
        .vertex_surface_origin
        .iter()
        .all(|origin| *origin == ModelSurfaceOrigin::None));
}
