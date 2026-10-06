//! Preserve original polygon corners through triangulation and animation.

use v2k_formats::models::{
    parse_model_subblocks, AnimVars, ModelEntry, ModelFaceVertices, ModelPainterOp,
    ModelVertexProjection,
};

fn parse_entry(records: &[[i16; 4]], words: &[u16]) -> ModelEntry {
    let mut data = Vec::new();
    data.extend_from_slice(&(words.len() as u16).to_le_bytes());
    data.extend_from_slice(&[0, 0x40]);
    data.extend_from_slice(&(records.len() as u16 * 2).to_le_bytes());
    data.extend_from_slice(&2u16.to_le_bytes());
    data.extend_from_slice(&[0; 4]);
    for record in records {
        for value in record {
            data.extend_from_slice(&value.to_le_bytes());
        }
    }
    for word in words {
        data.extend_from_slice(&word.to_le_bytes());
    }
    data.resize((data.len() + 3) & !3, 0);
    let header = 0x0001_0000 | data.len() as u32;
    parse_model_subblocks(&data, header)
        .unwrap()
        .all_entries
        .remove(0)
}

const CORNERS: [[i16; 4]; 4] = [
    [0, 10, 0, -100],
    [0, 110, 0, 100],
    [0, 110, 100, 100],
    [0, 10, 100, 100],
];

#[test]
fn all_face_families_shared_by_near_and_far_tables_keep_original_polygons() {
    // The same 24 command families address 48 near/far retail handlers.
    // Decoding retains the polygon independently of the selected render table,
    // sprite material, lighting family, or a corner's model-space Z sign.
    for family in [0x00, 0x20, 0x40, 0x80, 0xa0, 0xc0] {
        for primitive in [0x03, 0x04, 0x07, 0x08] {
            let opcode = family | primitive;
            let quad = primitive & 1 == 0;
            let mirrored = primitive >= 7;
            let source: &[u16] = if quad { &[0, 2, 4, 6] } else { &[0, 2, 4] };
            let mut words = vec![opcode, 17, 0];
            words.extend_from_slice(source);
            if family & 0x20 != 0 {
                words.extend_from_slice(if quad { &[0, 1, 0, 1] } else { &[0, 1, 0] });
            }
            words.push(0);
            let entry = parse_entry(&CORNERS, &words);
            let live = entry.materialize(&AnimVars::default());
            assert_eq!(
                entry.face_vertices, live.face_vertices,
                "opcode {opcode:02x}"
            );
            assert_eq!(entry.triangles, live.triangles, "opcode {opcode:02x}");
            let expected = if quad {
                let mut faces = vec![ModelFaceVertices::Quad([0, 1, 2, 3]); 2];
                if mirrored {
                    faces.extend([ModelFaceVertices::Quad([4, 5, 6, 7]); 2]);
                }
                faces
            } else {
                let mut faces = vec![ModelFaceVertices::Triangle([0, 1, 2])];
                if mirrored {
                    faces.push(ModelFaceVertices::Triangle([3, 4, 5]));
                }
                faces
            };
            assert_eq!(entry.face_vertices, expected, "opcode {opcode:02x}");
            assert_eq!(entry.face_vertices.len(), entry.triangles.len());
            if mirrored {
                let corner_count = source.len();
                for corner in 0..corner_count {
                    let [x, y, z] = entry.vertices[corner];
                    assert_eq!(entry.vertices[corner + corner_count], [-x, y, z]);
                }
            }
        }
    }
}

#[test]
fn unresolved_fourth_corner_drops_both_quad_halves_and_keeps_following_face() {
    let entry = parse_entry(
        &CORNERS,
        &[
            0x08, 7, 0, 0, 2, 4, 100, // Unresolvable fourth corner, both mirrors.
            0x03, 9, 0, 0, 2, 4, 0,
        ],
    );
    assert_eq!(entry.triangles.len(), 1);
    assert_eq!(entry.face_materials, [9]);
    assert_eq!(
        entry.face_vertices,
        [ModelFaceVertices::Triangle([0, 1, 2])]
    );
    let live = entry.materialize(&AnimVars::default());
    assert_eq!(live.face_vertices, entry.face_vertices);
    let ranges: Vec<_> = live
        .painter_program
        .iter()
        .filter_map(|operation| match operation {
            ModelPainterOp::Face { triangle_range, .. } => Some(triangle_range.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(ranges, [0..1]);
}

#[test]
fn surviving_degenerate_quad_half_retains_all_authored_corner_references() {
    for (slots, expected) in [([0, 0, 2, 4], [0, 0, 1, 2]), ([0, 2, 4, 4], [0, 1, 2, 2])] {
        let mut words = vec![0x04, 7, 0];
        words.extend(slots);
        words.push(0);
        let entry = parse_entry(&CORNERS, &words);
        assert_eq!(entry.triangles, [[0, 1, 2]]);
        assert_eq!(entry.face_vertices, [ModelFaceVertices::Quad(expected)]);
        assert_eq!(entry.face_vertices[0].indices(), expected);
    }
}

#[test]
fn generated_fourth_corner_keeps_quad_provenance_in_static_and_live_geometry() {
    let mut records = CORNERS.to_vec();
    records.push([8, 0x80, 0, 6]); // Lerp P0/P3 from dynamic callback 0.
    let entry = parse_entry(&records, &[0x84, 17, 0, 0, 2, 4, 8, 0]);
    assert_eq!(
        entry.face_vertices,
        [ModelFaceVertices::Quad([0, 1, 2, 3]); 2]
    );
    assert_eq!(entry.vertices[3], [10.0, 0.0, -100.0]);
    let mut vars = AnimVars::default();
    vars.dynamic[0] = 0x8000;
    let live = entry.materialize(&vars);
    assert_eq!(live.vertices[3], [10.0, 50.0, 0.0]);
    assert_eq!(live.face_vertices, entry.face_vertices);
    assert_eq!(live.triangles, entry.triangles);
}

#[test]
fn screen_midpoints_retain_nested_mirrored_sources_but_model_midpoints_use_positions() {
    use ModelVertexProjection::{Position, ScreenMidpoint};

    let records = [
        CORNERS[0],
        CORNERS[1],
        CORNERS[3],
        [1, 0, 0, 2], // Screen midpoint of slots 0 and 2.
        [1, 0, 6, 4], // Screen midpoint of the previous midpoint and slot 4.
        [5, 0, 0, 2], // Same model-space position as slot 6, ordinary projector.
    ];
    let entry = parse_entry(&records, &[0x07, 17, 0, 8, 4, 10, 0]);
    assert_eq!(entry.triangles, [[0, 4, 5], [6, 10, 11]]);
    assert_eq!(
        entry.vertex_projection,
        [
            ScreenMidpoint([1, 4]),
            ScreenMidpoint([2, 3]),
            Position,
            Position,
            Position,
            Position,
            ScreenMidpoint([7, 10]),
            ScreenMidpoint([8, 9]),
            Position,
            Position,
            Position,
            Position,
        ]
    );
    assert_eq!(entry.vertex_projection.len(), entry.vertices.len());
    assert_eq!(entry.vertices[1], entry.vertices[5]);
    assert_eq!(entry.vertices[7], entry.vertices[11]);
    for vertex in 0..6 {
        let [x, y, z] = entry.vertices[vertex];
        assert_eq!(entry.vertices[vertex + 6], [-x, y, z]);
    }
    let live = entry.materialize(&AnimVars::default());
    assert_eq!(live.vertex_projection, entry.vertex_projection);
    assert_eq!(live.face_vertices, entry.face_vertices);
}

#[test]
fn missing_and_cyclic_screen_midpoint_sources_drop_faces_and_reset_point_cloud_metadata() {
    for midpoint in [
        [1, 0, 0, 100], // Missing second source.
        [1, 0, 6, 0],   // Self-reference.
        [1, 0, 8, 0],   // Nested cycle through slot 8.
    ] {
        let records = [CORNERS[0], CORNERS[1], CORNERS[2], midpoint, [1, 0, 6, 2]];
        let entry = parse_entry(&records, &[0x08, 17, 0, 0, 2, 4, 6, 0]);
        assert!(entry.triangles.is_empty());
        assert!(entry.face_vertices.is_empty());
        assert_eq!(entry.vertices.len(), 3);
        assert_eq!(
            entry.vertex_projection,
            [ModelVertexProjection::Position; 3]
        );
        let live = entry.materialize(&AnimVars::default());
        assert_eq!(live.vertex_projection, entry.vertex_projection);
        assert!(live.triangles.is_empty());
    }
}

#[test]
fn billboard_only_nodes_keep_resolved_anchor_indices_and_projection_dependencies() {
    use ModelVertexProjection::{Position, ScreenMidpoint};

    let records = [
        [0, 900, 0, 0], // Unreferenced plain point must not replace the anchor.
        [1, 0, 4, 6],
        [0, 10, 0, 32],
        [0, 30, 0, 160],
    ];
    for opcode in [0x68, 0x78] {
        let entry = parse_entry(&records, &[opcode, 2, 1, 3, 0, 0]);
        assert!(entry.triangles.is_empty());
        assert!(entry.edges.is_empty());
        assert_eq!(entry.billboards.len(), 1);
        assert_eq!(entry.billboards[0].vertex, 0);
        assert_eq!(
            entry.vertices,
            [[20.0, 0.0, 96.0], [10.0, 0.0, 32.0], [30.0, 0.0, 160.0]]
        );
        assert_eq!(
            entry.vertex_projection,
            [ScreenMidpoint([1, 2]), Position, Position]
        );
        let live = entry.materialize(&AnimVars::default());
        assert_eq!(live.vertices, entry.vertices);
        assert_eq!(live.vertex_projection, entry.vertex_projection);
        assert_eq!(live.billboards[0].vertex, 0);
    }
}
