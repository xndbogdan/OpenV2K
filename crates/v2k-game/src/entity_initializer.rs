//! Evidence-backed entity construction and initial behavior ownership.
//!
//! Retail construction combines universal state, terrain classification,
//! Section-12 type policy, a weighted behavior selection, and a resource-domain
//! marker before the successful wrapper installs the entity basis. This module
//! keeps those phases together while retaining unknown water/RNG outcomes at
//! bit granularity.

use v2k_formats::terrain::TerrainGrid;

use crate::entity_behavior::{
    behavior_program, initial_behavior_state_policy, initializer_success_is_identity_determined,
    resolve_rng_independent_initial_behavior, translate_state_policy, BehaviorContextRuntime,
    BehaviorProgram, BehaviorSelection, BehaviorWeightRule, RngIndependentInitialBehavior,
    TranslatedStatePolicy,
};
use crate::entity_collision_state::{
    EntityTypeRuntimeMetadata, RetailRuntimeValue, RetailStateWord,
    ACTIVE_MODEL_SLOT_HIGH_STATE_BIT, FULLY_ABOVE_SURFACE_STATE_BIT, FULLY_BELOW_SURFACE_STATE_BIT,
    SURFACE_STATE_MASK,
};

const CONSTRUCTOR_BASE_STATE: u32 = 0x0607_8801;
const CONSTRUCTOR_SPAWN_PARAM_STATE_BIT: u32 = 0x0100_0000;
pub const CONSTRUCTOR_SURFACE_STATE_MASK: u32 = SURFACE_STATE_MASK;
const CROSS_DOMAIN_STATE_BIT: u32 = 0x8000_0000;

/// D4A0's birth position, before copying the immutable +90 anchor or
/// selecting behavior. Bit20 admits the terrain snap; bit40 adds the active
/// model's header08 radius. Independently of bit40,40D650..40D691 then adds
/// Sub-C's signed first word when the descriptor exists. Neither addition
/// belongs to a later mover visit, including for dormant cinematic actors.
pub(crate) fn constructor_position_raw(
    metadata: &EntityTypeRuntimeMetadata,
    authored_position_raw: [i16; 3],
    terrain: Option<&TerrainGrid>,
    active_model_radius_raw: Option<i16>,
) -> RetailRuntimeValue<[i16; 3]> {
    let Some(initializer) = metadata.initializer.as_ref() else {
        return RetailRuntimeValue::Unresolved;
    };
    if initializer.initializer_state_flags_raw & 0x20 == 0 {
        return RetailRuntimeValue::Known(authored_position_raw);
    }
    let Some(terrain) = terrain else {
        // D515/D52A skip the complete placement branch when no terrain exists.
        return RetailRuntimeValue::Known(authored_position_raw);
    };
    let model_radius = if initializer.initializer_state_flags_raw & 0x40 != 0 {
        let Some(radius) = active_model_radius_raw else {
            return RetailRuntimeValue::Unresolved;
        };
        radius
    } else {
        0
    };
    let clearance = match metadata.sub_c_lift_descriptor {
        RetailRuntimeValue::Known(Some(descriptor)) => descriptor.base_clearance_raw,
        RetailRuntimeValue::Known(None) => 0,
        RetailRuntimeValue::Unresolved => return RetailRuntimeValue::Unresolved,
    };
    let [x, _, z] = authored_position_raw;
    RetailRuntimeValue::Known([
        x,
        terrain
            .bilinear_height_raw(x, z)
            .wrapping_add(model_radius)
            .wrapping_add(clearance),
        z,
    ])
}

/// 104B0 compares the pre-D4A0 authored Y against the current sea/wave.
/// The wave flag is the world descriptor +84 policy copied by42EA30/433BD0;
/// it is independent from the renderer's water-plane visibility threshold.
pub(crate) fn constructor_surface_bits_at_tick(
    authored_position_raw: [i16; 3],
    terrain: &TerrainGrid,
    retail_tick: u32,
    waves_enabled: bool,
) -> Option<u32> {
    let [x, y, z] = authored_position_raw;
    let cell = terrain.cell(usize::from((x as u16) >> 8), usize::from((z as u16) >> 8))?;
    let floor = i16::from(cell.height as i8) * 32;
    let sea = terrain.sea_level_raw();
    let surface = if waves_enabled && floor < sea {
        v2k_formats::terrain::wave_surface_raw(x, z, retail_tick as i32, sea, floor)
    } else {
        sea
    };
    Some(match y.cmp(&surface) {
        std::cmp::Ordering::Less => FULLY_BELOW_SURFACE_STATE_BIT,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => FULLY_ABOVE_SURFACE_STATE_BIT,
    })
}

/// `104B0` compares authored Y with the sea before `09A80` publishes components.
/// Its wave branch is unreachable when the integer-cell floor is at or above
/// sea, so these two bits need neither a draw clock nor a collision radius.
/// Wet cells remain unresolved without the constructor's wave clock. Terrain
/// type bit0x10 selects another state domain and does not gate this comparison.
/// All 42 authored Type-9 births in gameplay overlays 13--49 select dry cells
/// here, including the 36 births outside Level 1. Their pre-grounding Y is zero
/// and their static sea is negative, so this source branch resolves them without
/// a level/spawn whitelist or a fabricated wave clock.
pub(crate) fn constructor_surface_bits_without_wave(
    authored_position_raw: [i16; 3],
    terrain: &TerrainGrid,
) -> Option<u32> {
    let [x, y, z] = authored_position_raw;
    let cell = terrain.cell(usize::from((x as u16) >> 8), usize::from((z as u16) >> 8))?;
    let sea = terrain.sea_level_raw();
    if i16::from(cell.height as i8) * 32 < sea {
        return None;
    }
    Some(match y.cmp(&sea) {
        std::cmp::Ordering::Less => FULLY_BELOW_SURFACE_STATE_BIT,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => FULLY_ABOVE_SURFACE_STATE_BIT,
    })
}

/// Relation between the resource handle supplied to `FUN_004104B0` and the
/// current six-bit world domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceDomainRelation {
    Current,
    Foreign,
    Unresolved,
}

/// Complete context needed to retain construction policy without hidden
/// globals or a positional argument train.
#[derive(Debug, Clone, Copy)]
pub struct EntityInitializerRequest<'a> {
    pub metadata: Option<&'a EntityTypeRuntimeMetadata>,
    pub spawn_param: u32,
    pub authored_position_raw: [i16; 3],
    pub terrain: Option<&'a TerrainGrid>,
    pub resource_domain: ResourceDomainRelation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityInitializerResolution {
    pub state_flags: RetailStateWord,
    pub initial_behavior: RetailRuntimeValue<Option<BehaviorSelection>>,
    /// Live context stays unresolved after a fallible nonempty selection until
    /// an owner publishes either initializer success or the unnamed failure
    /// fallback. Class-23 Power Up is the audited infallible exception.
    pub current_behavior_context: RetailRuntimeValue<Option<BehaviorContextRuntime>>,
}

#[derive(Debug, Clone, Copy)]
enum InitialBehaviorMode {
    Passive,
    Selected(Option<BehaviorSelection>),
}

fn resolve_behavior_identity(
    metadata: Option<&EntityTypeRuntimeMetadata>,
    mode: InitialBehaviorMode,
) -> RetailRuntimeValue<Option<BehaviorSelection>> {
    if let InitialBehaviorMode::Selected(selection) = mode {
        return RetailRuntimeValue::Known(selection);
    }
    let Some(initializer) = metadata.and_then(|metadata| metadata.initializer.as_ref()) else {
        return RetailRuntimeValue::Unresolved;
    };
    match resolve_rng_independent_initial_behavior(&initializer.behavior_choices) {
        Ok(RngIndependentInitialBehavior::Known(selection)) => RetailRuntimeValue::Known(selection),
        Ok(RngIndependentInitialBehavior::RuntimeDependent) | Err(_) => {
            RetailRuntimeValue::Unresolved
        }
    }
}

fn initial_context_without_publication(
    behavior: RetailRuntimeValue<Option<BehaviorSelection>>,
) -> RetailRuntimeValue<Option<BehaviorContextRuntime>> {
    match behavior {
        RetailRuntimeValue::Known(None) => RetailRuntimeValue::Known(None),
        RetailRuntimeValue::Known(Some(selection))
            if initializer_success_is_identity_determined(selection.program) =>
        {
            BehaviorContextRuntime::from_infallible_initial_selection(selection)
                .map_or(RetailRuntimeValue::Unresolved, |context| {
                    RetailRuntimeValue::Known(Some(context))
                })
        }
        RetailRuntimeValue::Known(Some(_)) | RetailRuntimeValue::Unresolved => {
            RetailRuntimeValue::Unresolved
        }
    }
}

fn overwrite_state_policy(state: &mut RetailStateWord, policy: TranslatedStatePolicy) {
    state.overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);
}

fn apply_behavior_program_state(
    mut state: RetailStateWord,
    program: &'static BehaviorProgram,
) -> Vec<RetailStateWord> {
    overwrite_state_policy(&mut state, initial_behavior_state_policy(program));
    if program.component_setup_success_state_set_bits == 0 {
        return vec![state];
    }

    let mut setup_succeeded = state;
    setup_succeeded.overwrite(
        program.component_setup_success_state_set_bits,
        program.component_setup_success_state_set_bits,
    );
    vec![state, setup_succeeded]
}

fn apply_initial_behavior_state_domain(
    seed: RetailStateWord,
    metadata: &EntityTypeRuntimeMetadata,
    mode: InitialBehaviorMode,
) -> RetailStateWord {
    if let InitialBehaviorMode::Selected(selection) = mode {
        return selection.map_or(seed, |selection| {
            RetailStateWord::merge_alternatives(&apply_behavior_program_state(
                seed,
                selection.program,
            ))
        });
    }
    let Some(initializer) = metadata.initializer.as_ref() else {
        return RetailStateWord::unknown();
    };
    match resolve_rng_independent_initial_behavior(&initializer.behavior_choices) {
        Ok(RngIndependentInitialBehavior::Known(None)) => seed,
        Ok(RngIndependentInitialBehavior::Known(Some(selection))) => {
            RetailStateWord::merge_alternatives(&apply_behavior_program_state(
                seed,
                selection.program,
            ))
        }
        Ok(RngIndependentInitialBehavior::RuntimeDependent) => {
            let mut alternatives = Vec::new();
            let mut has_positive_always = false;
            let mut maximum_total = 0_i64;
            for choice in initializer.behavior_choices.iter() {
                let Some(rule) = BehaviorWeightRule::from_raw(choice.weight_rule_id) else {
                    return RetailStateWord::unknown();
                };
                let signed_weight = choice.weight_multiplier as i32;
                if signed_weight < 0 {
                    return RetailStateWord::unknown();
                }
                if signed_weight == 0 {
                    continue;
                }
                has_positive_always |= rule == BehaviorWeightRule::Always;
                maximum_total += i64::from(signed_weight);
                let Some(program) = behavior_program(choice.behavior_class_id) else {
                    return RetailStateWord::unknown();
                };
                alternatives.extend(apply_behavior_program_state(seed, program));
            }
            let guaranteed_selection = has_positive_always && maximum_total <= i64::from(i32::MAX);
            if !guaranteed_selection || alternatives.is_empty() {
                alternatives.push(seed);
            }
            RetailStateWord::merge_alternatives(&alternatives)
        }
        Err(_) => RetailStateWord::unknown(),
    }
}

fn apply_resource_domain(state: &mut RetailStateWord, relation: ResourceDomainRelation) {
    match relation {
        ResourceDomainRelation::Current => state.overwrite(CROSS_DOMAIN_STATE_BIT, 0),
        ResourceDomainRelation::Foreign => {
            state.overwrite(CROSS_DOMAIN_STATE_BIT, CROSS_DOMAIN_STATE_BIT)
        }
        ResourceDomainRelation::Unresolved => state.invalidate(CROSS_DOMAIN_STATE_BIT),
    }
}

/// Resolve every constructor/initializer state domain supported by the current
/// evidence without advancing retail's shared RNG stream.
pub fn resolve_entity_initializer(
    request: EntityInitializerRequest<'_>,
) -> EntityInitializerResolution {
    resolve_entity_initializer_inner(request, InitialBehaviorMode::Passive)
}

/// Resolve construction after the caller has evaluated the live world and
/// advanced the process-global RNG exactly once through the retail selector.
///
/// This seam deliberately accepts the resulting selection rather than owning
/// traversal or RNG. A future sequential spawn transaction can therefore make
/// selection, topology publication, scheduler installation, and rollback
/// boundaries explicit.
pub fn resolve_entity_initializer_with_selected_behavior(
    request: EntityInitializerRequest<'_>,
    selected: Option<BehaviorSelection>,
) -> EntityInitializerResolution {
    resolve_entity_initializer_inner(request, InitialBehaviorMode::Selected(selected))
}

fn resolve_entity_initializer_inner(
    request: EntityInitializerRequest<'_>,
    behavior_mode: InitialBehaviorMode,
) -> EntityInitializerResolution {
    let initial_behavior = resolve_behavior_identity(request.metadata, behavior_mode);
    // Selection identity normally precedes a fallible initializer. Until a
    // live owner supplies its zero/nonzero result, retail may have retained the
    // selected descriptor or replaced it with the unnamed fallback. The
    // helper admits only the separately audited infallible class-23 exception.
    let current_behavior_context = initial_context_without_publication(initial_behavior);
    let Some(metadata) = request.metadata else {
        let mut state_flags = RetailStateWord::unknown();
        apply_resource_domain(&mut state_flags, request.resource_domain);
        state_flags.overwrite(4, 4);
        return EntityInitializerResolution {
            state_flags,
            initial_behavior,
            current_behavior_context,
        };
    };
    let Some(initializer) = metadata.initializer.as_ref() else {
        let mut state_flags = RetailStateWord::unknown();
        apply_resource_domain(&mut state_flags, request.resource_domain);
        state_flags.overwrite(4, 4);
        return EntityInitializerResolution {
            state_flags,
            initial_behavior,
            current_behavior_context,
        };
    };

    let mut state_flags = RetailStateWord::exact(
        CONSTRUCTOR_BASE_STATE
            | u32::from(request.spawn_param != 0) * CONSTRUCTOR_SPAWN_PARAM_STATE_BIT,
    );
    if let Some(terrain) = request.terrain {
        // Retail's animated surface comparison needs the process-global tick.
        // Forget only those two bits; terrain type and every unrelated domain
        // remain exact. Coordinates come from the authored pre-grounding pose.
        state_flags.invalidate(CONSTRUCTOR_SURFACE_STATE_MASK);
        let cell_x = ((request.authored_position_raw[0] as i32 >> 8) & 0xff) as usize;
        let cell_z = ((request.authored_position_raw[2] as i32 >> 8) & 0xff) as usize;
        let terrain_selects_slot_two = terrain
            .cell(cell_x, cell_z)
            .is_some_and(|cell| cell.terrain_type & 0x10 != 0);
        state_flags.overwrite(
            ACTIVE_MODEL_SLOT_HIGH_STATE_BIT,
            u32::from(terrain_selects_slot_two) * ACTIVE_MODEL_SLOT_HIGH_STATE_BIT,
        );
    } else {
        state_flags.overwrite(
            CONSTRUCTOR_SURFACE_STATE_MASK,
            FULLY_ABOVE_SURFACE_STATE_BIT,
        );
    }
    apply_resource_domain(&mut state_flags, request.resource_domain);
    overwrite_state_policy(
        &mut state_flags,
        translate_state_policy(initializer.initializer_state_flags_raw),
    );
    state_flags = apply_initial_behavior_state_domain(state_flags, metadata, behavior_mode);
    // FUN_00413F70 applies this only after successful initialization.
    state_flags.overwrite(4, 4);

    EntityInitializerResolution {
        state_flags,
        initial_behavior,
        current_behavior_context,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity_collision_state::{EntityInitializerSpec, PAIR_COLLISION_ENABLED_STATE_BIT};
    use v2k_formats::collision::BehaviorChoice;
    use v2k_formats::terrain::{TerrainCell, GRID_SIZE};

    fn metadata(choices: &[(u32, u32, u32)]) -> EntityTypeRuntimeMetadata {
        EntityTypeRuntimeMetadata {
            initializer: Some(EntityInitializerSpec {
                initializer_state_flags_raw: 0,
                common_axis_descriptor: Default::default(),
                behavior_choices: choices
                    .iter()
                    .map(
                        |&(weight_rule_id, weight_multiplier, behavior_class_id)| BehaviorChoice {
                            weight_rule_id,
                            weight_multiplier,
                            behavior_class_id,
                        },
                    )
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
                behavior_rule_ref: 0,
                alternate_behavior_class_ref: 0,
            }),
            ..EntityTypeRuntimeMetadata::default()
        }
    }

    fn terrain(terrain_type: u8, attribute: u8) -> TerrainGrid {
        TerrainGrid {
            header: [0; 5],
            cells: vec![
                TerrainCell {
                    height: 0,
                    attribute,
                    terrain_type,
                };
                GRID_SIZE * GRID_SIZE
            ],
        }
    }

    #[test]
    fn d4a0_position_keeps_authored_y_without_ground_policy_or_terrain() {
        let mut metadata = metadata(&[]);
        let terrain = terrain(0, 0);
        for y in [0, -200, 700] {
            let authored = [123, y, -456];
            // No placement branch: unresolved Sub-C and model data are unused.
            assert_eq!(
                constructor_position_raw(&metadata, authored, Some(&terrain), None),
                RetailRuntimeValue::Known(authored)
            );
        }
        metadata
            .initializer
            .as_mut()
            .unwrap()
            .initializer_state_flags_raw = 0x60;
        assert_eq!(
            constructor_position_raw(&metadata, [1, 2, 3], None, None),
            RetailRuntimeValue::Known([1, 2, 3])
        );
    }

    #[test]
    fn d4a0_position_adds_model_radius_and_signed_sub_c_clearance_before_anchor() {
        let mut metadata = metadata(&[]);
        metadata
            .initializer
            .as_mut()
            .unwrap()
            .initializer_state_flags_raw = 0x20;
        let mut terrain = terrain(0, 0);
        for cell in &mut terrain.cells {
            cell.height = (-2_i8) as u8;
        }
        let authored = [123, 900, -456];
        assert_eq!(
            constructor_position_raw(&metadata, authored, Some(&terrain), None),
            RetailRuntimeValue::Unresolved
        );
        metadata.sub_c_lift_descriptor = RetailRuntimeValue::Known(None);
        assert_eq!(
            constructor_position_raw(&metadata, authored, Some(&terrain), None),
            RetailRuntimeValue::Known([123, -64, -456])
        );
        let mut descriptor =
            crate::ordinary_type47_death_live::TYPE47_COMMON_DYING_SUB_C_DESCRIPTOR;
        descriptor.base_clearance_raw = 75;
        metadata.sub_c_lift_descriptor = RetailRuntimeValue::Known(Some(descriptor));
        assert_eq!(
            constructor_position_raw(&metadata, authored, Some(&terrain), None),
            RetailRuntimeValue::Known([123, 11, -456])
        );
        metadata
            .initializer
            .as_mut()
            .unwrap()
            .initializer_state_flags_raw |= 0x40;
        assert_eq!(
            constructor_position_raw(&metadata, authored, Some(&terrain), None),
            RetailRuntimeValue::Unresolved
        );
        descriptor.base_clearance_raw = -75;
        metadata.sub_c_lift_descriptor = RetailRuntimeValue::Known(Some(descriptor));
        assert_eq!(
            constructor_position_raw(&metadata, authored, Some(&terrain), Some(300)),
            RetailRuntimeValue::Known([123, 161, -456])
        );
    }

    fn request<'a>(
        metadata: Option<&'a EntityTypeRuntimeMetadata>,
        terrain: Option<&'a TerrainGrid>,
        resource_domain: ResourceDomainRelation,
    ) -> EntityInitializerRequest<'a> {
        EntityInitializerRequest {
            metadata,
            spawn_param: 0,
            authored_position_raw: [0; 3],
            terrain,
            resource_domain,
        }
    }

    #[test]
    fn dry_constructor_surface_uses_authored_y_with_strict_equalities() {
        // Floor == sea excludes waves. The terrain-type bit and attributes
        // belong to independent constructor state and never change this test.
        let grid = terrain(0x10, 0xff);
        for (y, expected) in [
            (-1, FULLY_BELOW_SURFACE_STATE_BIT),
            (0, 0),
            (1, FULLY_ABOVE_SURFACE_STATE_BIT),
        ] {
            assert_eq!(
                constructor_surface_bits_without_wave([0, y, 0], &grid),
                Some(expected)
            );
        }
    }

    #[test]
    fn wet_constructor_surface_retains_clock_boundary_and_wraps_integer_cell() {
        let mut grid = terrain(0, 0);
        grid.cells[255 * GRID_SIZE + 254].height = (-1i8) as u8;
        assert_eq!(
            constructor_surface_bits_without_wave([-1, 1024, -257], &grid),
            None
        );
        // The adjacent dry cell is selected by unsigned high bytes, without
        // bilinear interpolation across the wet corner or nearest-cell bias.
        assert_eq!(
            constructor_surface_bits_without_wave([-1, 1024, -256], &grid),
            Some(FULLY_ABOVE_SURFACE_STATE_BIT)
        );
    }

    #[v2k_test_support::retail_test]
    fn all_authored_gameplay_type9_births_have_clock_independent_constructor_surface() {
        let overlay_dir = v2k_test_support::retail_dir().join("Overlay");
        let mut births_by_level = Vec::new();
        for level_id in 13..=49 {
            let bytes = std::fs::read(overlay_dir.join(format!("1X{level_id}XX.OVL")))
                .expect("the retail corpus must contain each gameplay overlay");
            let ovl = v2k_formats::ovl::OvlFile::parse(&bytes).unwrap();
            let terrain =
                v2k_formats::terrain::parse_terrain(&ovl.section(10).unwrap().data).unwrap();
            let level = v2k_formats::levels::parse_level(&ovl.section(13).unwrap().data).unwrap();
            let mut births = 0;
            for spawn in level.entities.iter().filter(|spawn| spawn.entity_type == 9) {
                births += 1;
                let position = spawn.position_raw();
                assert_eq!(position[1], 0, "level {level_id} spawn {}", spawn.index);
                assert!(terrain.sea_level_raw() < 0);
                assert_eq!(
                    constructor_surface_bits_without_wave(position, &terrain),
                    Some(FULLY_ABOVE_SURFACE_STATE_BIT),
                    "level {level_id} spawn {} must classify its authored pre-grounding pose",
                    spawn.index
                );
            }
            if births != 0 {
                births_by_level.push((level_id, births));
            }
        }
        assert_eq!(
            births_by_level,
            [(13, 6), (14, 8), (15, 8), (25, 14), (39, 6)]
        );
    }

    #[test]
    fn surface_uncertainty_does_not_hide_pair_or_model_domains() {
        let metadata = metadata(&[(1, 1, 0)]);
        let terrain = terrain(0x10, 0xff);
        let resolution = resolve_entity_initializer(request(
            Some(&metadata),
            Some(&terrain),
            ResourceDomainRelation::Current,
        ));
        assert_eq!(
            resolution.state_flags.known_mask(),
            !CONSTRUCTOR_SURFACE_STATE_MASK
        );
        assert_eq!(
            resolution
                .state_flags
                .masked(PAIR_COLLISION_ENABLED_STATE_BIT),
            RetailRuntimeValue::Known(PAIR_COLLISION_ENABLED_STATE_BIT)
        );
        assert_eq!(
            resolution
                .state_flags
                .masked(ACTIVE_MODEL_SLOT_HIGH_STATE_BIT),
            RetailRuntimeValue::Known(ACTIVE_MODEL_SLOT_HIGH_STATE_BIT)
        );
        assert_eq!(
            resolution.initial_behavior,
            RetailRuntimeValue::Known(Some(BehaviorSelection {
                choice_index: 0,
                program: behavior_program(0).unwrap(),
            }))
        );
        assert_eq!(
            resolution.current_behavior_context,
            RetailRuntimeValue::Unresolved
        );
    }

    #[test]
    fn weighted_identity_and_conditional_component_state_stay_unresolved() {
        let metadata = metadata(&[(1, 9, 29), (1, 1, 32)]);
        let resolution = resolve_entity_initializer(request(
            Some(&metadata),
            None,
            ResourceDomainRelation::Current,
        ));
        assert_eq!(resolution.initial_behavior, RetailRuntimeValue::Unresolved);
        assert_eq!(
            resolution.current_behavior_context,
            RetailRuntimeValue::Unresolved
        );
        assert_eq!(
            resolution
                .state_flags
                .masked(PAIR_COLLISION_ENABLED_STATE_BIT),
            RetailRuntimeValue::Known(PAIR_COLLISION_ENABLED_STATE_BIT)
        );
        assert_eq!(
            resolution.state_flags.masked(0x0000_0028),
            RetailRuntimeValue::Unresolved,
            "only a selected turret whose component setup succeeds sets 0x28"
        );
    }

    #[test]
    fn caller_selected_job_behavior_closes_identity_style_and_state_policy() {
        let metadata = metadata(&[(13, 100, 54), (1, 1, 6)]);
        let passive = resolve_entity_initializer(request(
            Some(&metadata),
            None,
            ResourceDomainRelation::Current,
        ));
        assert_eq!(passive.initial_behavior, RetailRuntimeValue::Unresolved);

        for (choice_index, class_id) in [(0, 54), (1, 6)] {
            let selection = BehaviorSelection {
                choice_index,
                program: behavior_program(class_id).unwrap(),
            };
            let resolution = resolve_entity_initializer_with_selected_behavior(
                request(Some(&metadata), None, ResourceDomainRelation::Current),
                Some(selection),
            );
            assert_eq!(
                resolution.initial_behavior,
                RetailRuntimeValue::Known(Some(selection))
            );
            assert_eq!(
                resolution.current_behavior_context,
                RetailRuntimeValue::Unresolved
            );

            let policy = initial_behavior_state_policy(selection.program);
            assert_eq!(
                resolution
                    .state_flags
                    .masked(policy.set_bits | policy.clear_bits),
                RetailRuntimeValue::Known(policy.set_bits)
            );
        }
    }

    #[test]
    fn resource_domain_is_explicit_even_without_type_metadata() {
        for (relation, expected) in [
            (ResourceDomainRelation::Current, 0),
            (ResourceDomainRelation::Foreign, CROSS_DOMAIN_STATE_BIT),
        ] {
            let resolution = resolve_entity_initializer(request(None, None, relation));
            assert_eq!(
                resolution.state_flags.masked(CROSS_DOMAIN_STATE_BIT),
                RetailRuntimeValue::Known(expected)
            );
            assert_eq!(
                resolution.state_flags.masked(4),
                RetailRuntimeValue::Known(4)
            );
        }
        assert_eq!(
            resolve_entity_initializer(request(None, None, ResourceDomainRelation::Unresolved,))
                .state_flags
                .masked(CROSS_DOMAIN_STATE_BIT),
            RetailRuntimeValue::Unresolved
        );
    }
}
