//! Painter-command coverage of the normal presentation-tier Klaus hierarchy.

use v2k_formats::models::{
    AnimVars, LinkedModelSlots, ModelEntry, ModelPainterDepthKey, ModelPainterOp,
    ModelPainterSorting, ResolvedModelSlot,
};

fn klaus_models(tier: u8) -> Vec<ModelEntry> {
    let path = v2k_test_support::retail_dir().join(format!("Overlay/{tier}X2XX.OVL"));
    let ovl = v2k_formats::ovl::OvlFile::parse(&std::fs::read(path).unwrap()).unwrap();
    v2k_formats::sections::parse_models(&ovl)
        .unwrap()
        .all_entries
}

fn visit(
    models: &[ModelEntry],
    id: usize,
    vars: &AnimVars,
    linked: Option<&LinkedModelSlots>,
    depth: usize,
    masks: &mut Vec<(usize, i32)>,
) {
    assert!(depth < 20, "unexpected Klaus recursion");
    let model = models[id].materialize_with_context(
        v2k_formats::models::ModelMaterializationContext::intrinsic(vars, linked),
    );
    let mut triangles = vec![0; model.triangles.len()];
    let mut instances = vec![0; model.instances.len()];
    let mut scopes = Vec::new();
    for op in &model.painter_program {
        match op {
            ModelPainterOp::Face {
                triangle_range,
                depth_key,
            } => {
                assert_ne!(
                    *depth_key,
                    ModelPainterDepthKey::Unresolved,
                    "face key model {id}"
                );
                assert!((1..=2).contains(&triangle_range.len()));
                for index in triangle_range.clone() {
                    triangles[index] += 1;
                }
                if let Some(Some(offset)) = scopes.last() {
                    assert!(model.face_materials[triangle_range.clone()]
                        .iter()
                        .all(|&material| material == 0x20));
                    masks.push((id, *offset));
                }
            }
            ModelPainterOp::BeginGroup {
                depth_key, sorting, ..
            } => {
                assert_ne!(
                    *depth_key,
                    ModelPainterDepthKey::Unresolved,
                    "group key model {id}"
                );
                let offset = match depth_key {
                    ModelPainterDepthKey::Vertex { offset_raw, .. }
                        if matches!((id, *offset_raw), (8, 1500) | (9, 2250)) =>
                    {
                        assert_eq!(*sorting, ModelPainterSorting::Unsorted);
                        Some(*offset_raw)
                    }
                    _ => None,
                };
                scopes.push(offset);
            }
            ModelPainterOp::EndGroup => {
                assert!(scopes.pop().is_some(), "unmatched E6 in model {id}");
            }
            ModelPainterOp::Instance { instance_index } => {
                instances[*instance_index] += 1;
                let instance = &model.instances[*instance_index];
                let mut child_vars = vars.clone();
                child_vars.registers = instance.registers;
                // Retail 67410 passes the remapped parent slots through the
                // instance frame. The matte imports intentionally lie well
                // outside the creature, so omitting linkage deletes them.
                let origin = instance.attach_pos.expect("Klaus attachment");
                let child_linked: LinkedModelSlots = instance
                    .linked_slots
                    .iter()
                    .map(|&slot| {
                        std::array::from_fn(|mirror| {
                            let point = models[id].resolve_slot_with_context(
                                slot ^ mirror as u16,
                                v2k_formats::models::ModelMaterializationContext::intrinsic(
                                    &child_vars,
                                    linked,
                                ),
                            )?;
                            let delta: [f64; 3] =
                                std::array::from_fn(|axis| point.position_raw[axis] - origin[axis]);
                            Some(ResolvedModelSlot {
                                world_point: None,
                                native_view_point: None,
                                position_raw: std::array::from_fn(|axis| {
                                    (0..3)
                                        .map(|row| instance.orientation[row][axis] * delta[row])
                                        .sum()
                                }),
                                clip: point.clip,
                                surface_origin: point.surface_origin,
                            })
                        })
                    })
                    .collect();
                // 67410 copies only the first four parent registers into
                // the child command context; linkage above uses the full
                // parent snapshot when resolving exported points.
                child_vars.registers[4..].fill(0);
                visit(
                    models,
                    usize::from(instance.model_id),
                    &child_vars,
                    Some(&child_linked),
                    depth + 1,
                    masks,
                );
            }
            ModelPainterOp::Edge { .. } | ModelPainterOp::Billboard { .. } => {
                panic!("unexpected non-face primitive in Klaus model {id}");
            }
        }
    }
    assert!(scopes.is_empty(), "unclosed painter group in model {id}");
    assert!(
        triangles.iter().all(|&count| count == 1),
        "face coverage model {id}"
    );
    assert!(
        instances.iter().all(|&count| count == 1),
        "instance coverage model {id}"
    );
}

#[v2k_test_support::retail_test]
fn klaus_reached_hierarchy_keeps_every_primitive_and_balanced_mask_groups() {
    let models = klaus_models(1);
    assert_eq!(models[1].name.as_deref(), Some("klaus"));
    let mut masks = Vec::new();
    for morph in [0, 0x4000, 0x8000, 0xb000, 0xc000, 0xffff] {
        for state in [0, 1] {
            let mut vars = AnimVars::default();
            vars.dynamic[1] = morph;
            vars.dynamic[2] = state;
            visit(&models, 1, &vars, None, 0, &mut masks);
        }
    }
    assert!(masks.contains(&(8, 1500)), "head mask scope absent");
    assert!(masks.contains(&(9, 2250)), "jaw mask scope absent");
}

#[v2k_test_support::retail_test]
fn klaus_support_links_and_entire_model_program_are_invariant_across_tiers() {
    let reference = klaus_models(1);
    // Raw slots 2/3 and 4/5 are the X-mirrored pairs of these records.
    // The remaining root anchors and complete descendant programs are
    // identical too, so a verified matte-only link policy is tier-invariant.
    assert_eq!(
        reference[1].records[1..3],
        [[0, -768, 640, -1536], [0, -768, -1280, -1536]]
    );
    for tier in [0, 2, 3] {
        let models = klaus_models(tier);
        for id in 1..=12 {
            let expected = &reference[id];
            let actual = &models[id];
            assert_eq!(actual.name, expected.name, "tier{tier} model{id}");
            assert_eq!(
                actual.records, expected.records,
                "tier{tier} model{id} vertices"
            );
            assert_eq!(
                actual.normal_pool, expected.normal_pool,
                "tier{tier} model{id} normals"
            );
            assert_eq!(
                actual.cmd_words, expected.cmd_words,
                "tier{tier} model{id} program"
            );
            assert_eq!(
                (
                    actual.flags,
                    actual.slot_count,
                    actual.face_val,
                    actual.radius
                ),
                (
                    expected.flags,
                    expected.slot_count,
                    expected.face_val,
                    expected.radius
                ),
                "tier{tier} model{id} render header"
            );
        }
    }
}
