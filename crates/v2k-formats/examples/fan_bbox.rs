//! Throwaway: materialize pl4engine (and pl4enginesurround) with default
//! AnimVars and print bbox extents to find the fan disc's spin axis (thinnest
//! extent = disc normal). Run: cargo run -p v2k-formats --example fan_bbox
use std::path::Path;
use v2k_formats::models::AnimVars;

fn main() {
    // Retail install: V2K_RETAIL_DIR, else the repository's ignored `retail/`.
    let root = std::env::var_os("V2K_RETAIL_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../retail"));
    // Section 8 is byte-identical across the display tiers. This explicit low
    // fixture is valid for geometry evidence; it is not a visual-art default.
    let ovl_path = root.join("Overlay/0X3XX.OVL");
    let data = std::fs::read(&ovl_path).expect("read OVL");
    let ovl = v2k_formats::ovl::OvlFile::parse(&data).expect("OVL parse");
    let models = v2k_formats::sections::parse_models(&ovl).expect("section 8 parse");

    for target in ["pl4engine", "pl4enginesurround", "player4"] {
        let Some(entry) = models
            .all_entries
            .iter()
            .find(|e| e.name.as_deref() == Some(target))
        else {
            println!("{target}: NOT FOUND");
            continue;
        };
        let mat = entry.materialize(&AnimVars::default());
        if mat.vertices.is_empty() {
            println!("{target}: no vertices");
            continue;
        }
        let mut mn = [f64::MAX; 3];
        let mut mx = [f64::MIN; 3];
        for v in &mat.vertices {
            for k in 0..3 {
                mn[k] = mn[k].min(v[k]);
                mx[k] = mx[k].max(v[k]);
            }
        }
        let ext = [mx[0] - mn[0], mx[1] - mn[1], mx[2] - mn[2]];
        let ctr = [
            (mx[0] + mn[0]) / 2.0,
            (mx[1] + mn[1]) / 2.0,
            (mx[2] + mn[2]) / 2.0,
        ];
        let thin = (0..3)
            .min_by(|&a, &b| ext[a].partial_cmp(&ext[b]).unwrap())
            .unwrap();
        let axis_name = ["X", "Y", "Z"][thin];
        println!(
            "{target}: verts={} tris={} inst={}",
            mat.vertices.len(),
            mat.triangles.len(),
            mat.instances.len()
        );
        println!(
            "  min=({:.1},{:.1},{:.1}) max=({:.1},{:.1},{:.1})",
            mn[0], mn[1], mn[2], mx[0], mx[1], mx[2]
        );
        println!(
            "  extent=({:.1},{:.1},{:.1}) center=({:.1},{:.1},{:.1})",
            ext[0], ext[1], ext[2], ctr[0], ctr[1], ctr[2]
        );
        println!("  THIN axis = {axis_name} (extent {:.1})", ext[thin]);
        for inst in &mat.instances {
            println!(
                "    inst -> id {} attach={:?}",
                inst.model_id, inst.attach_pos
            );
            let o = inst.orientation;
            for row in &o {
                println!("        [{:+.2} {:+.2} {:+.2}]", row[0], row[1], row[2]);
            }
            // Where does the child's local +Y (disc normal) point in this
            // model's frame? world_col = orientation * (0,1,0) = column 1.
            println!(
                "        child +Y maps to ({:+.2},{:+.2},{:+.2})",
                o[0][1], o[1][1], o[2][1]
            );
        }
    }
}
