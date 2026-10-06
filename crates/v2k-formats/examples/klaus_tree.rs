//! Inspect the menu backdrop (`klaus`) inline-instance hierarchy.
//!
//! Run: cargo run -p v2k-formats --example klaus_tree

use std::collections::HashSet;
use std::path::Path;

use v2k_formats::models::{
    AnimVars, LinkedModelSlots, ModelEntry, ModelInstance, ResolvedModelSlot,
};

fn main() {
    // Retail install: V2K_RETAIL_DIR, else the repository's ignored `retail/`.
    let root = std::env::var_os("V2K_RETAIL_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../retail"));
    let ovl_path = root.join("Overlay/0X2XX.OVL");
    let data = std::fs::read(&ovl_path).expect("read OVL");
    let ovl = v2k_formats::ovl::OvlFile::parse(&data).expect("OVL parse");
    let models = v2k_formats::sections::parse_models(&ovl).expect("section 8 parse");

    for target in [
        "klaus",
        "ptersectklaus",
        "pteranasetheadklaus",
        "insectjawklaus",
        "insectwing1aklaus",
    ] {
        let Some((id, _)) = models
            .all_entries
            .iter()
            .enumerate()
            .find(|(_, e)| e.name.as_deref() == Some(target))
        else {
            println!("{target}: NOT FOUND");
            continue;
        };
        println!("\n== {target} local/global id {id} ==");
        print_records(&models.all_entries[id]);
        print_mount_and_instance_ops(&models.all_entries[id]);
        let mut seen = HashSet::new();
        dump_tree(&models.all_entries, id, 0, 6, &mut seen);
        if target == "klaus" {
            let mut stats = TreeStats::default();
            collect_linked_tree(&models.all_entries, id, None, 8, &mut stats);
            println!(
                "  linked materialized tree: models={} verts={} tris={} billboards={}",
                stats.models, stats.vertices, stats.triangles, stats.billboards
            );
        }
    }
}

fn print_records(entry: &v2k_formats::models::ModelEntry) {
    for (i, r) in entry.records.iter().enumerate() {
        if r[0] != 0 {
            println!(
                "  rec[{i}] slot {} tf={} a={} b={} c={}",
                i * 2,
                r[0],
                r[1],
                r[2],
                r[3]
            );
        }
    }
}

fn print_mount_and_instance_ops(entry: &v2k_formats::models::ModelEntry) {
    let mut pc = 0usize;
    while pc < entry.cmd_words.len() {
        let op = entry.cmd_words[pc];
        match op {
            0x00 => break,
            0x03 | 0x43 | 0x83 | 0xC3 | 0x07 | 0x47 | 0x87 | 0xC7 => pc += 6,
            0x04 | 0x44 | 0x84 | 0xC4 | 0x08 | 0x48 | 0x88 | 0xC8 => pc += 7,
            0x23 | 0xA3 | 0x27 | 0xA7 => pc += 9,
            0x24 | 0xA4 | 0x28 | 0xA8 => pc += 11,
            0x02 => pc += 4,
            0x22 => pc += 5,
            0x68 | 0xE8 | 0x78 | 0xB8 | 0xF8 => pc += 5,
            0x0B | 0x0C => pc += 3,
            0x13 => pc += 4,
            0x14 => {
                if pc + 1 >= entry.cmd_words.len() {
                    break;
                }
                pc = pc + 1 + entry.cmd_words[pc + 1] as usize;
            }
            0x2B | 0x2C => {
                if pc + 3 >= entry.cmd_words.len() {
                    break;
                }
                let a = entry.cmd_words[pc + 2];
                let b = entry.cmd_words[pc + 3];
                let take = if op == 0x2B { a != b } else { a == b };
                if take {
                    pc = pc + 1 + entry.cmd_words[pc + 1] as usize;
                } else {
                    pc += 4;
                }
            }
            0x0D | 0x1D | 0x2D | 0x3D | 0x4D | 0x5D | 0x6D | 0x7D | 0x8D | 0x9D | 0xAD | 0xBD
            | 0xCD | 0xDD | 0xED | 0xFD => pc += 4,
            0x06 | 0x26 | 0x38 => {
                let mut q = pc + 1;
                while q < entry.cmd_words.len() && entry.cmd_words[q] != 0xFFFF {
                    q += 1;
                }
                pc = q + 1;
            }
            0x46 | 0x15 => pc += 2,
            0xA6 => pc += 3,
            0xC6 => pc += 4,
            0x66 | 0x86 | 0xE6 | 0x0F | 0x10 => pc += 1,
            0x1C => pc += 3,
            0x3C => {
                if pc + 3 >= entry.cmd_words.len() {
                    break;
                }
                println!(
                    "  op 0x3c @{pc}: slots {}, {}, {}",
                    entry.cmd_words[pc + 1],
                    entry.cmd_words[pc + 2],
                    entry.cmd_words[pc + 3]
                );
                pc += 4;
            }
            0x5C => {
                if pc + 3 >= entry.cmd_words.len() {
                    break;
                }
                println!(
                    "  op 0x5c @{pc}: orient_code={:#06x} axis={} angle_operand={:#06x}",
                    entry.cmd_words[pc + 1],
                    entry.cmd_words[pc + 2],
                    entry.cmd_words[pc + 3]
                );
                pc += 4;
            }
            0x0E => {
                if pc + 4 >= entry.cmd_words.len() {
                    break;
                }
                let next = pc + 1 + (entry.cmd_words[pc + 3] / 2) as usize;
                let remap_end = next.min(entry.cmd_words.len());
                let remap = if pc + 4 < remap_end {
                    &entry.cmd_words[pc + 4..remap_end]
                } else {
                    &[]
                };
                println!(
                    "  op 0x0e @{pc}: code={:#06x} model={} skip_bytes={} attach={} remap={remap:?}",
                    entry.cmd_words[pc + 1],
                    entry.cmd_words[pc + 2],
                    entry.cmd_words[pc + 3],
                    entry.cmd_words[pc + 4]
                );
                pc = next;
            }
            _ => {
                println!("  unknown op {op:#06x} @{pc}");
                break;
            }
        }
    }
}

fn dump_tree(
    entries: &[v2k_formats::models::ModelEntry],
    id: usize,
    indent: usize,
    depth: u8,
    seen: &mut HashSet<usize>,
) {
    let Some(entry) = entries.get(id) else {
        println!("{:indent$}#{id}: MISSING", "", indent = indent);
        return;
    };
    let name = entry.name.as_deref().unwrap_or("<unnamed>");
    println!(
        "{:indent$}#{id} {name}: verts={} tris={} shadow={} edges={} billboards={} inst={}",
        "",
        entry.vertices.len(),
        entry.triangles.len(),
        entry.shadow_triangles.len(),
        entry.edges.len(),
        entry.billboards.len(),
        entry.instances.len(),
        indent = indent
    );
    if depth == 0 || !seen.insert(id) {
        return;
    }
    for (n, inst) in entry.instances.iter().enumerate() {
        let child = entries
            .get(inst.model_id as usize)
            .and_then(|e| e.name.as_deref())
            .unwrap_or("<missing>");
        println!(
            "{:indent$}inst[{n}] -> #{} {child} attach_slot={} attach_pos={:?}",
            "",
            inst.model_id,
            inst.attach_slot,
            inst.attach_pos,
            indent = indent + 2
        );
        println!(
            "{:indent$}linked_slots={:?}",
            "",
            inst.linked_slots,
            indent = indent + 4
        );
        for row in inst.orientation {
            println!(
                "{:indent$}[{:+.3} {:+.3} {:+.3}]",
                "",
                row[0],
                row[1],
                row[2],
                indent = indent + 4
            );
        }
        dump_tree(entries, inst.model_id as usize, indent + 4, depth - 1, seen);
    }
    seen.remove(&id);
}

#[derive(Default)]
struct TreeStats {
    models: usize,
    vertices: usize,
    triangles: usize,
    billboards: usize,
}

fn collect_linked_tree(
    entries: &[ModelEntry],
    id: usize,
    linked: Option<&LinkedModelSlots>,
    depth: u8,
    stats: &mut TreeStats,
) {
    let Some(entry) = entries.get(id) else {
        return;
    };
    let vars = AnimVars::default();
    let mat = entry.materialize_with_context(
        v2k_formats::models::ModelMaterializationContext::intrinsic(&vars, linked),
    );
    stats.models += 1;
    stats.vertices += mat.vertices.len();
    stats.triangles += mat.triangles.len();
    stats.billboards += mat.billboards.len();
    if depth == 0 {
        return;
    }
    for inst in &mat.instances {
        let child_linked = build_child_linked_slots(entry, linked, inst, &vars);
        collect_linked_tree(
            entries,
            inst.model_id as usize,
            Some(&child_linked),
            depth - 1,
            stats,
        );
    }
}

fn build_child_linked_slots(
    parent_model: &ModelEntry,
    parent_linked: Option<&LinkedModelSlots>,
    inst: &ModelInstance,
    vars: &AnimVars,
) -> LinkedModelSlots {
    let child_origin = inst.attach_pos.unwrap_or([0.0; 3]);
    inst.linked_slots
        .iter()
        .map(|&slot| {
            let even = parent_model
                .resolve_slot_with_context(
                    slot,
                    v2k_formats::models::ModelMaterializationContext::intrinsic(
                        vars,
                        parent_linked,
                    ),
                )
                .map(|p| ResolvedModelSlot {
                    world_point: None,
                    native_view_point: None,
                    position_raw: child_local_from_parent(
                        p.position_raw,
                        child_origin,
                        inst.orientation,
                    ),
                    clip: p.clip,
                    surface_origin: p.surface_origin,
                });
            let odd = parent_model
                .resolve_slot_with_context(
                    slot ^ 1,
                    v2k_formats::models::ModelMaterializationContext::intrinsic(
                        vars,
                        parent_linked,
                    ),
                )
                .map(|p| ResolvedModelSlot {
                    world_point: None,
                    native_view_point: None,
                    position_raw: child_local_from_parent(
                        p.position_raw,
                        child_origin,
                        inst.orientation,
                    ),
                    clip: p.clip,
                    surface_origin: p.surface_origin,
                });
            [even, odd]
        })
        .collect()
}

fn child_local_from_parent(
    parent_pos: [f64; 3],
    child_origin: [f64; 3],
    child_basis: [[f64; 3]; 3],
) -> [f64; 3] {
    let v = [
        parent_pos[0] - child_origin[0],
        parent_pos[1] - child_origin[1],
        parent_pos[2] - child_origin[2],
    ];
    [
        child_basis[0][0] * v[0] + child_basis[1][0] * v[1] + child_basis[2][0] * v[2],
        child_basis[0][1] * v[0] + child_basis[1][1] * v[1] + child_basis[2][1] * v[2],
        child_basis[0][2] * v[0] + child_basis[1][2] * v[1] + child_basis[2][2] * v[2],
    ]
}
