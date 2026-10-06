//! Print the decoded geometry/material/UV data for the rotating main-menu props.
//!
//! This is intentionally a text audit: asymmetric artwork such as `optexit`
//! and `slopt` makes front/back texture-orientation regressions much easier to
//! spot than they are in an untextured OBJ export.
//!
//! Run every prop, or name a subset:
//! `cargo run -p v2k-formats --example audit_menu_props -- optexit slopt`

use std::path::Path;

use v2k_formats::models::face_material;

const PROPS: &[(&str, &str)] = &[
    ("1X3XX.OVL", "player4"),
    ("1X3XX.OVL", "optexit"),
    ("1X5XX.OVL", "screenop"),
    ("1X5XX.OVL", "screeno2"),
    ("1X5XX.OVL", "sfxopt"),
    ("1X5XX.OVL", "slopt"),
    ("1X5XX.OVL", "psjoypad"),
    ("1X5XX.OVL", "multipc"),
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Retail install: V2K_RETAIL_DIR, else the repository's ignored `retail/`.
    let root = std::env::var_os("V2K_RETAIL_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../retail"));
    let selected: Vec<String> = std::env::args().skip(1).collect();
    let mut current_ovl = String::new();
    let mut models = None;

    for &(ovl_name, model_name) in PROPS {
        if !selected.is_empty() && !selected.iter().any(|name| name == model_name) {
            continue;
        }
        if current_ovl != ovl_name {
            let data = std::fs::read(root.join("Overlay").join(ovl_name))?;
            let ovl = v2k_formats::ovl::OvlFile::parse(&data)?;
            models = Some(v2k_formats::sections::parse_models(&ovl)?);
            current_ovl = ovl_name.to_owned();
        }
        let model = models
            .as_ref()
            .unwrap()
            .all_entries
            .iter()
            .find(|entry| entry.name.as_deref() == Some(model_name))
            .ok_or_else(|| format!("{model_name} missing from {ovl_name}"))?;

        println!(
            "\n== {ovl_name}/{model_name}: {} vertices, {} triangles ==",
            model.vertices.len(),
            model.triangles.len()
        );
        print!("words:");
        for (index, word) in model.cmd_words.iter().enumerate() {
            if index % 16 == 0 {
                print!("\n  {index:03}:");
            }
            print!(" {word:04X}");
        }
        println!();

        for (index, triangle) in model.triangles.iter().enumerate() {
            let (material, sprite) = face_material(model.face_materials[index]);
            let positions = triangle.map(|vertex| model.vertices[usize::from(vertex)]);
            println!(
                "  tri {index:02}: v={triangle:?} p={positions:?} mat={material}{} uv={:?}",
                if sprite { " sprite" } else { " colour" },
                model.face_uvs[index]
            );
        }
    }

    Ok(())
}
