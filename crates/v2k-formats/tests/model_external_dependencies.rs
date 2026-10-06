//! Live external callbacks belong to the admitted authored dependency graph.
use std::cell::RefCell;
use std::collections::HashMap;
use v2k_formats::models::{
    AnimVars, LinkedModelSlots, ModelEntry, ModelMaterializationContext, ModelVertexKind,
    ModelVertexProjection, ModelVertexResolver, ResolvedModelSlot,
};

#[derive(Default)]
struct LiveOwner {
    calls: RefCell<Vec<u16>>,
    cache: RefCell<HashMap<u16, ResolvedModelSlot>>,
}
impl ModelVertexResolver for LiveOwner {
    fn owns_external_frame_slots(&self) -> bool {
        true
    }
    fn admit_face_raw(&self, _: [i32; 3], normal: [i32; 3]) -> bool {
        normal[0] <= 0
    }
    fn resolve_external_frame_raw(&self, slot: u16, _: [i16; 3]) -> Option<ResolvedModelSlot> {
        if let Some(value) = self.cache.borrow().get(&slot).copied() {
            return Some(value);
        }
        self.calls.borrow_mut().push(slot);
        let mut value = ResolvedModelSlot::clear([900.0, 800.0, 700.0]);
        value.world_point = Some([1.125, -2.5, 3.75]);
        self.cache.borrow_mut().insert(slot, value);
        Some(value)
    }
    fn resolve_vertex_raw(
        &self,
        kind: ModelVertexKind,
        mut source: ResolvedModelSlot,
    ) -> Option<ResolvedModelSlot> {
        let mut world = source.world_point?;
        world[1] = match kind {
            ModelVertexKind::Alias => 20.0,
            ModelVertexKind::ViewPin => 0.0,
        };
        source.world_point = Some(world);
        Some(source)
    }
}

fn world(materialized: &v2k_formats::models::MaterializedModel, index: usize) -> [f64; 3] {
    let ModelVertexProjection::WorldPoint(point) = materialized.vertex_projection[index] else {
        panic!("owned callback endpoint must survive materialization")
    };
    point
}

#[test]
fn live_recursive_aliases_follow_only_admitted_callbacks_and_direct_vertices_stay_world_owned() {
    let model = ModelEntry {
        records: vec![
            [14, 0, 0, 0],
            [13, 0, 0, 0],
            [12, 2, 0, 0],
            [13, 4, 0, 0],
            [0, 1, 2, 3],
            [14, 1, 0, 0],
            [0, 0, 0, 0],
            [14, 2, 0, 0],
        ],
        normal_pool: vec![[12, 1, 0, 0], [14, -1, 0, 0]],
        cmd_words: vec![
            0x22, 986, 4, 6, 8, 0x03, 32, 2, 10, 8, 12, 0x22, 680, 7, 0, 8, 0x03, 32, 4, 0, 8, 12,
            0,
        ],
        ..ModelEntry::default()
    };
    let vars = AnimVars::default();
    let owner = LiveOwner::default();
    let materialized = model.materialize_with_context(ModelMaterializationContext {
        vertex_resolver: Some(&owner),
        ..ModelMaterializationContext::intrinsic(&vars, None)
    });
    assert_eq!(
        *owner.calls.borrow(),
        [0],
        "backface selector1 and unused selector2 must never run"
    );
    assert_eq!(materialized.edges.len(), 2);
    assert_eq!(materialized.triangles.len(), 1);
    let v2k_formats::models::ModelFaceCull::LiveAdmitted(plane) = materialized.face_cull[0] else {
        panic!("live plane decision belongs to the source owner");
    };
    assert_eq!(plane.native_anchor_raw, Some([2, 0, 0]));
    assert_eq!(
        plane.anchor_raw,
        [2.0, 0.0, 0.0],
        "normal-only selector is read as raw source words, never resolved"
    );
    assert_eq!(
        world(&materialized, materialized.edges[0].vertices[0] as usize),
        [1.125, 0.0, 3.75]
    );
    assert_eq!(
        world(&materialized, materialized.edges[1].vertices[0] as usize),
        [1.125, -2.5, 3.75]
    );
    // Intrinsic/collision readers retain the authored parameter representation.
    let intrinsic = model.materialize(&vars);
    assert_eq!(intrinsic.triangles.len(), 2);
    assert_eq!(
        intrinsic.vertices[intrinsic.edges[1].vertices[0] as usize],
        [0.0; 3]
    );
    assert!(intrinsic
        .vertex_projection
        .iter()
        .all(|value| !matches!(value, ModelVertexProjection::WorldPoint(_))));
}

#[test]
fn linked_import_preserves_world_custody_through_recursive_surface_aliases() {
    let mut imported = ResolvedModelSlot::clear([-999.0, 222.0, 444.0]);
    imported.world_point = Some([1.125, -2.5, 3.75]);
    let linked: LinkedModelSlots = vec![[Some(imported); 2]];
    let model = ModelEntry {
        records: vec![[11, 0, 0, 0], [12, 0, 0, 0], [13, 2, 0, 0], [0, 1, 2, 3]],
        cmd_words: vec![0x22, 986, 4, 4, 6, 0],
        ..ModelEntry::default()
    };
    let vars = AnimVars::default();
    let owner = LiveOwner::default();
    let result = model.materialize_with_context(ModelMaterializationContext {
        vertex_resolver: Some(&owner),
        ..ModelMaterializationContext::intrinsic(&vars, Some(&linked))
    });
    assert_eq!(
        world(&result, result.edges[0].vertices[0] as usize),
        [1.125, 0.0, 3.75]
    );
    assert!(
        owner.calls.borrow().is_empty(),
        "imported parent callback is not called in the child"
    );
    assert_eq!(
        model.resolve_slot_with_context(
            0,
            ModelMaterializationContext::intrinsic(&vars, Some(&linked))
        ),
        Some(imported)
    );
}

#[test]
fn missing_live_external_owner_drops_dependent_primitive_without_parameter_fallback() {
    struct Missing;
    impl ModelVertexResolver for Missing {
        fn owns_external_frame_slots(&self) -> bool {
            true
        }
        fn resolve_vertex_raw(
            &self,
            _: ModelVertexKind,
            source: ResolvedModelSlot,
        ) -> Option<ResolvedModelSlot> {
            Some(source)
        }
    }
    let model = ModelEntry {
        records: vec![[14, 12, 0, 0], [13, 0, 0, 0], [0, 0, 0, 0]],
        cmd_words: vec![0x22, 986, 4, 2, 4, 0],
        ..ModelEntry::default()
    };
    let vars = AnimVars::default();
    assert!(model
        .materialize_with_context(ModelMaterializationContext {
            vertex_resolver: Some(&Missing),
            ..ModelMaterializationContext::intrinsic(&vars, None)
        })
        .edges
        .is_empty());
    assert_eq!(model.materialize(&vars).edges.len(), 1);
}
