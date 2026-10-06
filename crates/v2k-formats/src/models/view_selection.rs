//! Per-node view input for the retail 0x0B/0x0C command predicates.

/// Selection of camera-dependent blocks in an otherwise intrinsic model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelViewSelection {
    /// Retain every view-dependent block for asset inspection. Animation
    /// branches still execute normally; this does not change collision code.
    IntrinsicAllBranches,
    /// Execute FUN_00467050/00467090 using the current model's retail context.
    Retail {
        /// Model origin minus camera, expressed in this model's raw local
        /// axes: ctx+0x7C/0x80/0x84 from FUN_00466160. Presentation constructs
        /// this after the current node's transform, before interpreting its
        /// commands; a child must receive its own local vector.
        local_origin_from_camera_raw: [i32; 3],
    },
}

/// FUN_0046D3F0's normal-cache byte +4. The raw anchor words deliberately do
/// not pass through the generated-vertex resolver. x86 IMUL/ADD retain the
/// low signed dword, and SETGE admits equality into the backfacing side.
pub(super) fn retail_normal_backfacing(
    records: &[[i16; 4]],
    normal_pool: &[[i16; 4]],
    normal_ref: u16,
    local_origin_from_camera_raw: [i32; 3],
) -> Option<bool> {
    // FUN_0046D5A0 seeds both implicit entries with flag zero and marks them
    // evaluated; even a zero stored normal instead goes through SETGE.
    if normal_ref < 2 {
        return Some(false);
    }
    let normal = normal_pool.get(usize::from(normal_ref / 2 - 1))?;
    let anchor_ref = normal[0] as u16;
    let record = records.get(usize::from(anchor_ref / 2))?;
    let mut anchor = [
        i32::from(record[1]),
        i32::from(record[2]),
        i32::from(record[3]),
    ];
    let mut direction = [
        i32::from(normal[1]),
        i32::from(normal[2]),
        i32::from(normal[3]),
    ];
    if (anchor_ref ^ normal_ref) & 1 != 0 {
        anchor[0] = -anchor[0];
    }
    if normal_ref & 1 != 0 {
        direction[0] = -direction[0];
    }
    let dot = (0..3).fold(0i32, |sum, axis| {
        sum.wrapping_add(
            local_origin_from_camera_raw[axis]
                .wrapping_add(anchor[axis])
                .wrapping_mul(direction[axis]),
        )
    });
    Some(dot >= 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        interpret, parse_model_subblocks, AnimVars, ModelEntry, ModelMaterializationContext,
        ModelPainterOp, StreamStats,
    };

    fn retail_context(vars: &AnimVars, origin: [i32; 3]) -> ModelMaterializationContext<'_> {
        ModelMaterializationContext {
            view_selection: ModelViewSelection::Retail {
                local_origin_from_camera_raw: origin,
            },
            ..ModelMaterializationContext::intrinsic(vars, None)
        }
    }

    fn alternative_model() -> ModelEntry {
        let mut words = Vec::new();
        for (opcode, material, child, register) in [(0x0B, 17, 71, 1), (0x0C, 18, 72, 2)] {
            let block = [
                0x0D, 0, register, 0, // r0 = the selected side
                0x15, 0, // one enclosing painter group
                0x04, material, 0, 0, 2, 4, 6, 0x22, 630, 3, 0, 2, 0x0E, 0, child, 8, 0, 0xE6,
            ];
            words.extend([opcode, block.len() as u16 + 2, 2]);
            words.extend(block);
        }
        // A later unguarded child must observe only the executed register write.
        words.extend([0x0E, 0, 90, 8, 0, 0]);
        ModelEntry {
            records: vec![[0, 0, 0, 0], [0, 10, 0, 0], [0, 10, 10, 0], [0, 0, 10, 0]],
            normal_pool: vec![[0, 0, 0, 1]],
            cmd_words: words,
            ..ModelEntry::default()
        }
    }

    #[test]
    fn retail_branch_selection_precedes_all_output_and_register_side_effects() {
        let model = alternative_model();
        let vars = AnimVars::default();
        let authored_words = model.cmd_words.clone();
        for (z, child, material, register) in [(-1, 71, 17, 1), (0, 72, 18, 2), (1, 72, 18, 2)] {
            let mut stats = StreamStats::default();
            let output =
                model.materialize_into_with_context(retail_context(&vars, [0, 0, z]), &mut stats);
            assert!(output.has_view_commands);
            assert_eq!(output.triangles.len(), 2);
            assert_eq!(output.face_materials, [material; 2]);
            assert_eq!(output.edges.len(), 1);
            assert_eq!(
                output
                    .instances
                    .iter()
                    .map(|instance| instance.model_id)
                    .collect::<Vec<_>>(),
                [child, 90]
            );
            assert!(output
                .instances
                .iter()
                .all(|instance| instance.registers[0] == register));
            assert_eq!(
                output
                    .painter_program
                    .iter()
                    .filter(|operation| matches!(operation, ModelPainterOp::BeginGroup { .. }))
                    .count(),
                1
            );
            assert_eq!(
                output
                    .painter_program
                    .iter()
                    .filter(|operation| matches!(operation, ModelPainterOp::EndGroup))
                    .count(),
                1
            );
            assert_eq!(stats.jumps_taken, 1);
            assert!(stats.is_clean(), "{stats:?}");
        }
        let intrinsic = model.materialize(&vars);
        assert!(intrinsic.has_view_commands);
        assert_eq!(intrinsic.triangles.len(), 4);
        assert_eq!(intrinsic.edges.len(), 2);
        assert_eq!(
            intrinsic
                .instances
                .iter()
                .map(|instance| instance.model_id)
                .collect::<Vec<_>>(),
            [71, 72, 90]
        );
        assert_eq!(model.cmd_words, authored_words);
    }

    #[test]
    fn parsed_view_metadata_comes_from_executed_opcodes_not_operand_words() {
        let mut model = alternative_model();
        for expected in [true, false] {
            if !expected {
                // 0B/0C occur only as materials, not command opcodes.
                model.cmd_words = vec![0x03, 0x0B, 0, 0, 2, 4, 0x03, 0x0C, 0, 0, 4, 6, 0];
            }
            let mut bytes = Vec::new();
            for word in [
                model.cmd_words.len() as u16,
                0x4000,
                (model.records.len() * 2) as u16,
                (model.normal_pool.len() * 2 + 2) as u16,
                0,
                0,
            ] {
                bytes.extend(word.to_le_bytes());
            }
            for record in model.records.iter().chain(&model.normal_pool) {
                for word in record {
                    bytes.extend(word.to_le_bytes());
                }
            }
            for word in &model.cmd_words {
                bytes.extend(word.to_le_bytes());
            }
            bytes.resize((bytes.len() + 3) & !3, 0);
            let parsed = parse_model_subblocks(&bytes, 0x10000 | bytes.len() as u32).unwrap();
            assert_eq!(parsed.all_entries[0].has_view_commands, expected);
            assert!(parsed.stats.is_clean(), "{:?}", parsed.stats);
        }
    }

    #[test]
    fn normal_and_anchor_mirrors_select_opposite_half_spaces_including_equality() {
        let records = [[0, 10, 0, 0]];
        for (anchor_ref, normal_ref, boundary, normal_sign) in [
            (0, 2, -10, 1),
            (0, 3, 10, -1),
            (1, 2, 10, 1),
            (1, 3, -10, -1),
        ] {
            for offset in [-1, 0, 1] {
                assert_eq!(
                    retail_normal_backfacing(
                        &records,
                        &[[anchor_ref, 1, 0, 0]],
                        normal_ref,
                        [boundary + offset, 0, 0]
                    ),
                    Some(offset * normal_sign >= 0)
                );
            }
        }
        // Negate after sign extension, including the -32768 source value.
        assert_eq!(
            retail_normal_backfacing(
                &[[0, i16::MIN, 0, 0]],
                &[[0, i16::MIN, 0, 0]],
                3,
                [-32769, 0, 0]
            ),
            Some(false)
        );
    }

    #[test]
    fn view_normal_uses_raw_anchor_words_and_signed_y_z_components() {
        // tf12 would resolve to the first point's X=100, but the normal
        // consumer reads the alias record's raw X word (slot operand zero).
        let records = [[0, 100, 0, 0], [12, 0, -7, 3]];
        assert_eq!(
            retail_normal_backfacing(&records, &[[2, 1, 0, 0]], 2, [-1, 0, 0]),
            Some(false)
        );
        assert_eq!(
            retail_normal_backfacing(&records, &[[2, 0, 2, -3]], 2, [0, 7, -3]),
            Some(true)
        );
        assert_eq!(
            retail_normal_backfacing(&records, &[[2, 0, 2, -3]], 3, [0, 6, -3]),
            Some(false)
        );
    }

    #[test]
    fn view_normal_preserves_wrapping_origin_products_and_dot_sum() {
        let records = [[0, 1, 1, 1]];
        assert_eq!(
            retail_normal_backfacing(&records, &[[0, 1, 0, 0]], 2, [i32::MAX, 0, 0]),
            Some(false)
        );
        // Positive mathematical products/sums cross bit31 in both cases.
        for origin in [[65538, -1, -1], [65536, 0, 0]] {
            assert_eq!(
                retail_normal_backfacing(&records, &[[0, 32767, 32767, 32767]], 2, origin),
                Some(false)
            );
        }
    }

    #[test]
    fn implicit_normals_remain_front_facing_but_stored_zero_is_back_facing() {
        for normal_ref in [0, 1] {
            assert_eq!(
                retail_normal_backfacing(&[], &[], normal_ref, [i32::MAX; 3]),
                Some(false)
            );
        }
        assert_eq!(
            retail_normal_backfacing(&[[0; 4]], &[[0; 4]], 2, [i32::MAX; 3]),
            Some(true)
        );
    }

    #[test]
    fn branch_offsets_are_signed_words_relative_to_the_first_argument() {
        let vars = AnimVars::default();
        // The first jump reaches pc4; the negative second jump reaches the
        // terminator at pc3. Treating -2 as unsigned would miss that target.
        let words = [0x0B, 3, 2, 0, 0x0B, (-2i16) as u16, 2, 0];
        let mut stats = StreamStats::default();
        let (output, _) = interpret(
            &words,
            &[[0; 4]],
            &[[0; 4]],
            retail_context(&vars, [0; 3]),
            &mut stats,
        );
        assert!(output.tris.is_empty());
        assert_eq!(stats.jumps_taken, 2);
        assert!(stats.is_clean(), "{stats:?}");
    }

    #[test]
    fn malformed_view_branches_stop_without_guessing_a_side() {
        let vars = AnimVars::default();
        for words in [
            vec![0x0B],
            vec![0x0B, 2],
            vec![0x0B, 2, 4, 0],
            vec![0x0B, 0xFFFE, 2, 0],
            vec![0x0B, 0x7FFF, 2, 0],
        ] {
            let mut stats = StreamStats::default();
            let (output, _) = interpret(
                &words,
                &[[0; 4]],
                &[[0; 4]],
                retail_context(&vars, [0; 3]),
                &mut stats,
            );
            assert!(output.tris.is_empty());
            assert_eq!(stats.invalid_view_branches, 1, "{words:?}");
            assert!(!stats.is_clean());
        }
        assert_eq!(retail_normal_backfacing(&[], &[[0; 4]], 2, [0; 3]), None);
    }

    #[test]
    fn a_backward_view_branch_keeps_the_existing_step_guard() {
        let vars = AnimVars::default();
        let mut stats = StreamStats::default();
        let (output, _) = interpret(
            &[0x0B, 0xFFFF, 2],
            &[[0; 4]],
            &[[0; 4]],
            retail_context(&vars, [0; 3]),
            &mut stats,
        );
        assert!(output.tris.is_empty());
        assert_eq!(stats.guard_aborts, 1);
        assert_eq!(stats.invalid_view_branches, 0);
    }
}
