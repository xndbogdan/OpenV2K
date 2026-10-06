//! Validate the canonical Section-8 interpreter across the retail OVL corpus.
//!
//! This intentionally validates parser invariants and known default-frame
//! retail-corpus totals
//! instead of comparing against the obsolete Python OBJ snapshots that
//! predated live materialization and generated-vertex fixes.
//!
//! Run from anywhere: `cargo run -p v2k-formats --example validate_sec8`

use std::path::Path;

fn main() {
    // Retail install: V2K_RETAIL_DIR, else the repository's ignored `retail/`.
    let root = std::env::var_os("V2K_RETAIL_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../retail"));
    let overlay_dir = root.join("Overlay");
    let mut paths: Vec<_> = std::fs::read_dir(&overlay_dir)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", overlay_dir.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("ovl"))
        })
        .collect();
    paths.sort();

    let (mut files, mut entries, mut triangles, mut shadows) = (0usize, 0usize, 0usize, 0usize);
    let (mut edges, mut billboards, mut instances) = (0usize, 0usize, 0usize);
    let mut valid = true;

    for path in paths {
        let data = std::fs::read(&path).expect("OVL read");
        let ovl = v2k_formats::ovl::OvlFile::parse(&data).expect("OVL parse");
        if !ovl.has_data(v2k_formats::sections::idx::SPRITE_METADATA) {
            continue;
        }
        let models = match v2k_formats::sections::parse_models(&ovl) {
            Ok(models) if !models.all_entries.is_empty() => models,
            Ok(_) | Err(_) => continue,
        };
        files += 1;

        if !models.stats.is_clean() {
            valid = false;
            eprintln!("{}: stream anomalies: {:?}", path.display(), models.stats);
        }

        for model in &models.all_entries {
            let vertex_count = model.vertices.len();
            let bad_triangle = model
                .triangles
                .iter()
                .chain(&model.shadow_triangles)
                .flatten()
                .any(|&index| index as usize >= vertex_count);
            let bad_edge = model
                .edges
                .iter()
                .flat_map(|edge| edge.vertices)
                .any(|index| index as usize >= vertex_count);
            let parallel_face_data = model.normals.len() == model.triangles.len()
                && model.face_materials.len() == model.triangles.len()
                && model.face_uvs.len() == model.triangles.len()
                && model.face_corner_normals.len() == model.triangles.len();
            if bad_triangle || bad_edge || !parallel_face_data {
                valid = false;
                eprintln!(
                    "{} model {} failed geometry invariants",
                    path.display(),
                    model.index,
                );
            }
        }

        entries += models.all_entries.len();
        triangles += models.total_triangles();
        shadows += models.total_shadow_triangles();
        edges += models.total_edges();
        billboards += models.total_billboards();
        instances += models
            .all_entries
            .iter()
            .map(|model| model.instances.len())
            .sum::<usize>();
    }

    println!(
        "{files} OVLs: {entries} models, {triangles} triangles ({shadows} shadow), \
         {edges} edges, {billboards} billboards, {instances} instances"
    );
    let retail_totals_match = files == 36
        && entries == 5_344
        && triangles == 124_100
        && shadows == 0
        && edges == 3_152
        && billboards == 912;
    if valid && retail_totals_match {
        println!("VALIDATION PASSED");
    } else {
        eprintln!("VALIDATION FAILED");
        std::process::exit(1);
    }
}
