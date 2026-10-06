use v2k_formats::models::{
    AnimVars, ModelEntry, ModelFaceCull, ModelMaterializationContext, ModelVertexKind,
    ModelVertexResolver, ResolvedModelSlot,
};

struct SlopedAlias;

impl ModelVertexResolver for SlopedAlias {
    fn resolve_vertex_raw(
        &self,
        kind: ModelVertexKind,
        slot: ResolvedModelSlot,
    ) -> Option<ResolvedModelSlot> {
        let source = slot.position_raw;
        Some(ResolvedModelSlot::clear(match kind {
            ModelVertexKind::Alias => [source[0], source[0] / 4.0, source[2]],
            ModelVertexKind::ViewPin => source,
        }))
    }
}

fn dependent_model() -> ModelEntry {
    // The authored spikes/fence chain: FUN_0046F1B0 resolves slot6 through
    // the active tf12 callback before adding the 128-high top offset.
    ModelEntry {
        records: vec![
            [0, 0, 0, 0],
            [0, 0, 128, -128],
            [0, 256, 256, 0],
            [12, 4, 0, 0],
            [6, 6, 2, 0],
            [13, 8, 0, 0],
            [5, 0, 0, 8],
        ],
        normal_pool: vec![[8, 0, 0, -32767]],
        cmd_words: vec![0x84, 1325, 2, 2, 8, 6, 0, 0],
        ..ModelEntry::default()
    }
}

#[test]
fn alias_context_precedes_generated_vertices_and_reflected_sampling() {
    let model = dependent_model();
    let vars = AnimVars::default();
    let intrinsic = ModelMaterializationContext::intrinsic(&vars, None);
    let context = ModelMaterializationContext {
        vertex_resolver: Some(&SlopedAlias),
        ..intrinsic
    };
    assert_eq!(
        model.resolve_slot_with_context(8, intrinsic),
        Some(ResolvedModelSlot::clear([256.0, 384.0, -128.0]))
    );
    assert_eq!(
        model.resolve_slot_with_context(8, context),
        Some(ResolvedModelSlot::clear([256.0, 192.0, -128.0]))
    );
    // Ground the odd source at negative X, never mirror an already grounded Y.
    assert_eq!(
        model.resolve_slot_with_context(9, context),
        Some(ResolvedModelSlot::clear([-256.0, 64.0, -128.0]))
    );
    assert_eq!(
        model.resolve_slot_with_context(12, context),
        Some(ResolvedModelSlot::clear([128.0, 96.0, -64.0]))
    );
    assert_eq!(
        model.resolve_slot_with_context(13, context),
        Some(ResolvedModelSlot::clear([-128.0, 32.0, -64.0]))
    );
    // The later type13 projection still receives the corrected source point.
    assert_eq!(
        model.resolve_slot_with_context(10, context),
        Some(ResolvedModelSlot::clear([256.0, 192.0, -128.0]))
    );
    assert_eq!(
        model.resolve_slot_with_context(11, context),
        Some(ResolvedModelSlot::clear([-256.0, 64.0, -128.0]))
    );
    let resolved = model.materialize_with_context(context);
    assert_eq!(resolved.vertices[1], [256.0, 192.0, -128.0]);
    let ModelFaceCull::Plane(plane) = resolved.face_cull[0] else {
        panic!("authored normal anchor")
    };
    assert_eq!(plane.anchor_raw, [256.0, 192.0, -128.0]);
    assert_eq!(resolved.face_uvs, model.materialize(&vars).face_uvs);
    assert_eq!(model.materialize(&vars).vertices[1], [256.0, 384.0, -128.0]);
}

#[test]
fn unavailable_alias_context_drops_its_dependent_face() {
    struct MissingSurface;
    impl ModelVertexResolver for MissingSurface {
        fn resolve_vertex_raw(
            &self,
            _: ModelVertexKind,
            _: ResolvedModelSlot,
        ) -> Option<ResolvedModelSlot> {
            None
        }
    }
    let vars = AnimVars::default();
    let model = dependent_model();
    let context = ModelMaterializationContext {
        vertex_resolver: Some(&MissingSurface),
        ..ModelMaterializationContext::intrinsic(&vars, None)
    };
    assert_eq!(model.resolve_slot_with_context(8, context), None);
    assert!(model.materialize_with_context(context).triangles.is_empty());
}
