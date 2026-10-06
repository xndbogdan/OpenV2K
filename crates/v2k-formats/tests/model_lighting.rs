//! Normal-tier gameplay faces must retain retail's uniform lighting family.

use v2k_formats::models::{AnimVars, ModelEntry, ModelFaceShading};

fn gameplay_models() -> Vec<ModelEntry> {
    let path = v2k_test_support::retail_dir().join("Overlay/1X3XX.OVL");
    let overlay = v2k_formats::ovl::OvlFile::parse(&std::fs::read(path).unwrap()).unwrap();
    v2k_formats::sections::parse_models(&overlay)
        .unwrap()
        .all_entries
}

#[v2k_test_support::retail_test]
fn authored_gameplay_hulls_keep_uniform_lighting_and_unlit_details_distinct() {
    let models = gameplay_models();
    for name in ["player4", "college", "factory2", "lifter"] {
        let entry = models
            .iter()
            .find(|entry| entry.name.as_deref() == Some(name))
            .unwrap();
        let model = entry.materialize(&AnimVars::default());
        assert_eq!(entry.face_shading, model.face_shading, "cached/live {name}");
        let uniform = model
            .face_shading
            .iter()
            .filter(|&&mode| mode == ModelFaceShading::FlatLit)
            .count();
        let unlit = model
            .face_shading
            .iter()
            .filter(|&&mode| mode == ModelFaceShading::Flat)
            .count();
        assert!(uniform > 0, "{name} contains C3/C4/C7/C8 faces");
        match name {
            "college" => assert_eq!(uniform, model.triangles.len(), "Main Base is uniformly lit"),
            "player4" | "factory2" => {
                assert!(
                    unlit > 0,
                    "{name} also contains authored 84/88 unlit details"
                );
                assert!(
                    uniform > unlit,
                    "{name} hull must not inherit the unlit detail policy"
                );
            }
            _ => {}
        }
        for (index, mode) in model.face_shading.iter().enumerate() {
            if *mode == ModelFaceShading::FlatLit {
                assert_eq!(
                    model.face_corner_normals[index], [model.normals[index]; 3],
                    "{name} face {index}"
                );
            }
        }
    }
}
