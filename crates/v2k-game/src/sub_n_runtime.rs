//! Loader-owned Sub-N runtime shared by Hive live control and Main Base cleanup.
//!
//! `FUN_00409A80` allocates and zero-fills the component from Section-12
//! Sub-N before behavior selection. When a Section-13 spawn carries animation,
//! `FUN_004381F0` optionally enriches that allocation through `FUN_0041BC20`.
//! The allocation therefore cannot be inferred from Alien-Hive behavior or
//! from the behavior-installed radial emitter.

use v2k_formats::anim_frames::TerrainObjectTable;
use v2k_formats::terrain::{TerrainGrid, GRID_SIZE};

use crate::entity_collision_state::RetailRuntimeValue;

/// Timer forced by `FUN_0041CF90` before its optional terrain cleanup.
pub const MAIN_BASE_ABORT_SUB_N_TIMER_US: i32 = -3_000_000;

/// Constructor evidence consumed only when the spawn has an animation block.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SubNConstructorTerrain<'a> {
    pub terrain: &'a TerrainGrid,
    pub terrain_objects: &'a TerrainObjectTable,
}

/// Loader-owned Sub-N words touched or observed by the alternate cleanup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubNRuntime {
    /// Loader zero-fill; `1CE10` writes one after an admitted inward impact.
    forced_death_requested_at_0x44: bool,
    terrain_patch_present_at_0x4c: RetailRuntimeValue<bool>,
    cleanup_timer_us_at_0x50: i32,
    anchor_raw_at_0x54: RetailRuntimeValue<[i16; 3]>,
    /// Host-retained constructor provenance used to authenticate the static
    /// descriptor which retail re-resolves only on a true `+0x4C` gate.
    descriptor_attachment_at_birth: RetailRuntimeValue<[i16; 3]>,
}

impl SubNRuntime {
    const fn zero_filled(descriptor_attachment_at_birth: RetailRuntimeValue<[i16; 3]>) -> Self {
        Self {
            forced_death_requested_at_0x44: false,
            terrain_patch_present_at_0x4c: RetailRuntimeValue::Known(false),
            cleanup_timer_us_at_0x50: 0,
            anchor_raw_at_0x54: RetailRuntimeValue::Known([0; 3]),
            descriptor_attachment_at_birth,
        }
    }

    const fn unresolved_enrichment(
        descriptor_attachment_at_birth: RetailRuntimeValue<[i16; 3]>,
    ) -> Self {
        Self {
            forced_death_requested_at_0x44: false,
            terrain_patch_present_at_0x4c: RetailRuntimeValue::Unresolved,
            cleanup_timer_us_at_0x50: 0,
            anchor_raw_at_0x54: RetailRuntimeValue::Unresolved,
            descriptor_attachment_at_birth,
        }
    }

    pub const fn terrain_patch_present(&self) -> RetailRuntimeValue<bool> {
        self.terrain_patch_present_at_0x4c
    }

    pub const fn forced_death_requested(&self) -> bool {
        self.forced_death_requested_at_0x44
    }

    pub(crate) fn request_forced_death(&mut self) {
        self.forced_death_requested_at_0x44 = true;
    }

    pub const fn cleanup_timer_us(&self) -> i32 {
        self.cleanup_timer_us_at_0x50
    }

    /// The same signed Sub-N+50 is the `1BEB0` clear timer and `1CF90`'s
    /// alternate-cleanup timer. Keep one word so abort's -3M write survives.
    pub(crate) fn live_grace_timer_us_mut(&mut self) -> &mut i32 {
        &mut self.cleanup_timer_us_at_0x50
    }

    pub const fn anchor_raw(&self) -> RetailRuntimeValue<[i16; 3]> {
        self.anchor_raw_at_0x54
    }

    pub(crate) const fn descriptor_attachment_at_birth(&self) -> RetailRuntimeValue<[i16; 3]> {
        self.descriptor_attachment_at_birth
    }

    /// Commit retail's timer write and return the optional X-major terrain
    /// sequence. A true gate requires the caller to preflight and supply the
    /// descriptor attachment; unresolved state returns before mutation.
    pub(crate) fn commit_main_base_abort_alternate_cleanup(
        &mut self,
        source_position_raw: [i16; 3],
        descriptor_attachment_raw: Option<[i16; 3]>,
    ) -> RetailRuntimeValue<SubNAlternateCleanupCommit> {
        let cells_x_major = match self.terrain_patch_present_at_0x4c {
            RetailRuntimeValue::Known(false) => None,
            RetailRuntimeValue::Known(true) => {
                let Some(attachment_raw) = descriptor_attachment_raw else {
                    return RetailRuntimeValue::Unresolved;
                };
                Some(sub_n_patch_cells_x_major(
                    source_position_raw,
                    attachment_raw,
                ))
            }
            RetailRuntimeValue::Unresolved => return RetailRuntimeValue::Unresolved,
        };
        self.cleanup_timer_us_at_0x50 = MAIN_BASE_ABORT_SUB_N_TIMER_US;
        RetailRuntimeValue::Known(SubNAlternateCleanupCommit { cells_x_major })
    }
}

/// Ordered terrain cells passed to `FUN_00433860` after the timer write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SubNAlternateCleanupCommit {
    pub cells_x_major: Option<[[u8; 2]; 9]>,
}

/// Construct loader allocation custody independently from behavior selection.
///
/// A proven absent Sub-N is `Known(None)`. A present Sub-N without spawn
/// animation retains the allocator's exact zero fill. Animation with missing
/// descriptor or Section-9/10 evidence keeps the allocation known while its
/// enrichment fields remain unresolved.
pub(crate) fn sub_n_runtime_from_constructor(
    sub_n_present: RetailRuntimeValue<bool>,
    sub_n_payload: Option<[u8; 10]>,
    has_spawn_animation: bool,
    source_position_raw: [i16; 3],
    constructor_terrain: Option<SubNConstructorTerrain<'_>>,
) -> RetailRuntimeValue<Option<SubNRuntime>> {
    match sub_n_present {
        RetailRuntimeValue::Unresolved => RetailRuntimeValue::Unresolved,
        RetailRuntimeValue::Known(false) => {
            if sub_n_payload.is_some() {
                RetailRuntimeValue::Unresolved
            } else {
                RetailRuntimeValue::Known(None)
            }
        }
        RetailRuntimeValue::Known(true) => {
            let descriptor_attachment_at_birth = sub_n_payload
                .map_or(RetailRuntimeValue::Unresolved, |payload| {
                    RetailRuntimeValue::Known(sub_n_descriptor_attachment(payload))
                });
            if !has_spawn_animation {
                return RetailRuntimeValue::Known(Some(SubNRuntime::zero_filled(
                    descriptor_attachment_at_birth,
                )));
            }
            let runtime = sub_n_payload
                .zip(constructor_terrain)
                .and_then(|(payload, context)| {
                    recover_enriched_runtime(source_position_raw, payload, context)
                })
                .unwrap_or_else(|| {
                    SubNRuntime::unresolved_enrichment(descriptor_attachment_at_birth)
                });
            RetailRuntimeValue::Known(Some(runtime))
        }
    }
}

pub(crate) fn sub_n_descriptor_attachment(payload: [u8; 10]) -> [i16; 3] {
    [
        i16::from_le_bytes(payload[4..6].try_into().expect("fixed Sub-N word")),
        i16::from_le_bytes(payload[6..8].try_into().expect("fixed Sub-N word")),
        i16::from_le_bytes(payload[8..10].try_into().expect("fixed Sub-N word")),
    ]
}

fn recover_enriched_runtime(
    source_position_raw: [i16; 3],
    payload: [u8; 10],
    context: SubNConstructorTerrain<'_>,
) -> Option<SubNRuntime> {
    let attachment_raw = sub_n_descriptor_attachment(payload);
    let base_x = source_position_raw[0].wrapping_add(attachment_raw[0]);
    let base_z = source_position_raw[2].wrapping_add(attachment_raw[2]);
    let mut runtime = SubNRuntime {
        forced_death_requested_at_0x44: false,
        terrain_patch_present_at_0x4c: RetailRuntimeValue::Known(false),
        cleanup_timer_us_at_0x50: 0,
        // FUN_0041BC20 retains entity Y, not Sub-N's signed Y offset.
        anchor_raw_at_0x54: RetailRuntimeValue::Known([base_x, source_position_raw[1], base_z]),
        descriptor_attachment_at_birth: RetailRuntimeValue::Known(attachment_raw),
    };

    for cell in sub_n_patch_cells_x_major(source_position_raw, attachment_raw) {
        let index = usize::from(cell[0]) * GRID_SIZE + usize::from(cell[1]);
        let attribute = context.terrain.cells.get(index)?.attribute;
        if attribute == 0 {
            continue;
        }
        let descriptor = context
            .terrain_objects
            .records
            .get(usize::from(attribute))?;
        if (22..=26).contains(&descriptor.kind_index) {
            runtime.terrain_patch_present_at_0x4c = RetailRuntimeValue::Known(true);
            runtime.anchor_raw_at_0x54 = RetailRuntimeValue::Known([
                (((u16::from(cell[0])) << 8) | 0x80) as i16,
                source_position_raw[1],
                (((u16::from(cell[1])) << 8) | 0x80) as i16,
            ]);
            break;
        }
    }
    Some(runtime)
}

pub(crate) fn sub_n_patch_cells_x_major(
    source_position_raw: [i16; 3],
    attachment_raw: [i16; 3],
) -> [[u8; 2]; 9] {
    let base_x = source_position_raw[0].wrapping_add(attachment_raw[0]);
    let base_z = source_position_raw[2].wrapping_add(attachment_raw[2]);
    let offsets = [-0x100_i16, 0, 0x100];
    std::array::from_fn(|index| {
        let x = base_x.wrapping_add(offsets[index / 3]);
        let z = base_z.wrapping_add(offsets[index % 3]);
        [(x as u16 >> 8) as u8, (z as u16 >> 8) as u8]
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use v2k_formats::anim_frames::{ModelSlotPattern, TerrainObjectDescriptor};
    use v2k_formats::terrain::TerrainCell;

    const PAYLOAD: [u8; 10] = [0, 0, 0, 0, 0x80, 0x01, 0x90, 0x01, 0x80, 0x01];

    fn terrain() -> TerrainGrid {
        TerrainGrid {
            header: [0; 5],
            cells: vec![
                TerrainCell {
                    height: 0,
                    attribute: 0,
                    terrain_type: 0,
                };
                GRID_SIZE * GRID_SIZE
            ],
        }
    }

    fn objects(attribute: u8, kind_index: u32) -> TerrainObjectTable {
        let mut records = vec![
            TerrainObjectDescriptor {
                model_ids: [0; 4],
                kind_index: 0,
                pattern: ModelSlotPattern::Static,
            };
            256
        ];
        records[usize::from(attribute)].kind_index = kind_index;
        TerrainObjectTable { records }
    }

    #[test]
    fn present_sub_n_without_animation_retains_zero_fill_and_commits_timer() {
        let RetailRuntimeValue::Known(Some(mut runtime)) = sub_n_runtime_from_constructor(
            RetailRuntimeValue::Known(true),
            Some(PAYLOAD),
            false,
            [0x0200, 0x0111, 0x0300],
            None,
        ) else {
            panic!("known loader allocation")
        };
        assert_eq!(
            runtime.terrain_patch_present(),
            RetailRuntimeValue::Known(false)
        );
        assert_eq!(runtime.anchor_raw(), RetailRuntimeValue::Known([0; 3]));
        assert_eq!(
            runtime.commit_main_base_abort_alternate_cleanup([0; 3], None),
            RetailRuntimeValue::Known(SubNAlternateCleanupCommit {
                cells_x_major: None,
            })
        );
        assert_eq!(runtime.cleanup_timer_us(), MAIN_BASE_ABORT_SUB_N_TIMER_US);
    }

    #[test]
    fn animated_constructor_retains_first_qualifying_x_major_anchor() {
        let source = [0x0200, -0x0123, 0x0300];
        let mut terrain = terrain();
        for (cell, attribute) in [([2_u8, 3_u8], 1), ([2, 4], 2), ([3, 3], 3)] {
            terrain.cells[usize::from(cell[0]) * GRID_SIZE + usize::from(cell[1])].attribute =
                attribute;
        }
        let objects = objects(2, 26);
        let runtime = sub_n_runtime_from_constructor(
            RetailRuntimeValue::Known(true),
            Some(PAYLOAD),
            true,
            source,
            Some(SubNConstructorTerrain {
                terrain: &terrain,
                terrain_objects: &objects,
            }),
        );
        let RetailRuntimeValue::Known(Some(runtime)) = runtime else {
            panic!("known enriched allocation")
        };
        assert_eq!(
            runtime.terrain_patch_present(),
            RetailRuntimeValue::Known(true)
        );
        assert_eq!(
            runtime.anchor_raw(),
            RetailRuntimeValue::Known([0x0280, -0x0123, 0x0480])
        );
    }

    #[test]
    fn animated_constructor_without_context_keeps_allocation_known() {
        let runtime = sub_n_runtime_from_constructor(
            RetailRuntimeValue::Known(true),
            Some(PAYLOAD),
            true,
            [0; 3],
            None,
        );
        let RetailRuntimeValue::Known(Some(runtime)) = runtime else {
            panic!("known loader allocation")
        };
        assert_eq!(
            runtime.terrain_patch_present(),
            RetailRuntimeValue::Unresolved
        );
        assert_eq!(runtime.anchor_raw(), RetailRuntimeValue::Unresolved);
        assert_eq!(runtime.cleanup_timer_us(), 0);
    }

    #[test]
    fn patch_cells_keep_x_major_order_across_signed_wrap() {
        assert_eq!(
            sub_n_patch_cells_x_major([i16::MAX, 0, i16::MIN], [1, 0, -1]),
            [
                [127, 126],
                [127, 127],
                [127, 128],
                [128, 126],
                [128, 127],
                [128, 128],
                [129, 126],
                [129, 127],
                [129, 128],
            ]
        );
    }
}
