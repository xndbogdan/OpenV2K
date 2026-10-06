//! Type-13 binding of shared `FUN_00401430`.
//!
//! First-world type 13 is D/E/G/K/L. Detailed scheduler mode 0 dispatches
//! G, K, then L; Restricted mode 1 dispatches only G. The accepted Intro2
//! transcript `20260727-235423-intro2-actor-task-mover.txt` records mode 1,
//! while the static `FUN_0040DCA0` path passes mode 0. Descriptor byte `+0x0A == 0`
//! bypasses every `FUN_0041FCB0` call, while the outer Sub-D allocation still
//! advances its stagger byte. This owner executes the exact target prelude,
//! classifier-free Sub-D steering, K/L post-write fan-out, and Type-13's full
//! normal Sub-G callback plus the selected K/L output callbacks, then resumes
//! the shared frame machine to its literal
//! nonzero return. Callers retain the outcome transactionally because the
//! physical entity and component allocations are separate retail owners.

use crate::entity_collision_state::{
    CommonMoverComponentTopology, EntityTypeRuntimeMetadata, RetailRuntimeValue,
};
use crate::gkl_common_mover::{evaluate_gkl_common_mover, GklCommonMoverProfile};
pub use crate::gkl_common_mover::{
    GklCommonMoverBlock, GklCommonMoverOutcome, GklCommonMoverRequest, GklCommonMoverRuntime,
    GklCommonMoverTopologyError,
};
use crate::search_attack_live::{
    TYPE13_SEARCH_ATTACK_GKL, TYPE13_SEARCH_ATTACK_SUB_D, TYPE13_SEARCH_ATTACK_TOPOLOGY,
};
use crate::type13_initial_behavior::TYPE13_ENTITY_TYPE;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type13CommonMoverTopology {
    components: CommonMoverComponentTopology,
}

impl Type13CommonMoverTopology {
    pub fn from_metadata(
        entity_type: u32,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> Result<Self, GklCommonMoverTopologyError> {
        if entity_type != TYPE13_ENTITY_TYPE {
            return Err(GklCommonMoverTopologyError::WrongEntityType {
                actual: entity_type,
            });
        }
        let components = match metadata.common_mover_topology {
            RetailRuntimeValue::Unresolved => {
                return Err(GklCommonMoverTopologyError::UnresolvedComponentTopology);
            }
            RetailRuntimeValue::Known(components) => components,
        };
        if components != TYPE13_SEARCH_ATTACK_TOPOLOGY {
            return Err(GklCommonMoverTopologyError::UnsupportedComponentTopology);
        }
        Ok(Self { components })
    }

    pub const fn components(self) -> CommonMoverComponentTopology {
        self.components
    }
}

pub fn evaluate_type13_common_mover(
    request: GklCommonMoverRequest<'_>,
    next_random: impl FnMut() -> u32,
) -> Result<GklCommonMoverOutcome, GklCommonMoverBlock> {
    let metadata = request
        .metadata
        .ok_or(GklCommonMoverBlock::TypeMetadataUnavailable)?;
    Type13CommonMoverTopology::from_metadata(request.entity_type, metadata)
        .map_err(GklCommonMoverBlock::Topology)?;
    evaluate_gkl_common_mover(
        request,
        GklCommonMoverProfile {
            topology: TYPE13_SEARCH_ATTACK_TOPOLOGY,
            sub_d: TYPE13_SEARCH_ATTACK_SUB_D,
            gkl: TYPE13_SEARCH_ATTACK_GKL,
        },
        next_random,
    )
    .map_err(|failure| failure.reason)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chase_target::ChaseTargetCommonMoverReturn;
    use crate::common_mover::component_dispatch::CommonMoverDispatchMode;
    use crate::common_mover::component_dispatch::{
        CommonMoverDispatchPhase, CommonMoverDispatchPlan,
    };
    use crate::common_mover::sub_d::ORDINARY_TYPE9_SUB_D;
    use crate::common_mover::target_prelude::CommonMoverTrackedTargetSnapshot;
    use crate::common_mover::type9_attitude::Type9BodyBasis;
    use crate::entity_collision_state::EntityTypeRuntimeMetadata;
    use crate::ordinary_type47_live::FRESH_LEVEL1_ORDINARY_TYPE47_COMPONENT_TOPOLOGY;
    use crate::search_attack_live::TYPE13_SEARCH_ATTACK_SUB_G;
    use crate::sub_g_runtime::SubG06070RuntimeState;
    use crate::wander_near_location::WanderNearPrivateState;
    use v2k_formats::terrain::TerrainCell;
    use v2k_formats::terrain::TerrainGrid;

    fn type13_metadata() -> EntityTypeRuntimeMetadata {
        let mut metadata = EntityTypeRuntimeMetadata::default();
        metadata.common_mover_topology = RetailRuntimeValue::Known(TYPE13_SEARCH_ATTACK_TOPOLOGY);
        metadata.common_mover_gkl_payloads = RetailRuntimeValue::Known(TYPE13_SEARCH_ATTACK_GKL);
        metadata.sub_d_steering_descriptor =
            RetailRuntimeValue::Known(Some(TYPE13_SEARCH_ATTACK_SUB_D));
        metadata
    }

    fn flat_terrain() -> TerrainGrid {
        TerrainGrid {
            header: [0; 5],
            cells: vec![
                TerrainCell {
                    height: 0,
                    attribute: 0,
                    terrain_type: 0,
                };
                256 * 256
            ],
        }
    }

    fn type13_sub_g_runtime() -> SubG06070RuntimeState {
        let mut runtime =
            SubG06070RuntimeState::from_1b8c0_constructor(&TYPE13_SEARCH_ATTACK_SUB_G, 0);
        runtime.apply_shared_06070_sub_g_branch(217, 0);
        runtime
    }

    #[test]
    fn admits_only_the_authored_type13_component_route() {
        let metadata = type13_metadata();
        assert!(Type13CommonMoverTopology::from_metadata(13, &metadata).is_ok());
        assert_eq!(
            Type13CommonMoverTopology::from_metadata(47, &metadata),
            Err(GklCommonMoverTopologyError::WrongEntityType { actual: 47 })
        );

        let mut type47_shape = metadata.clone();
        type47_shape.common_mover_topology =
            RetailRuntimeValue::Known(FRESH_LEVEL1_ORDINARY_TYPE47_COMPONENT_TOPOLOGY);
        assert_eq!(
            Type13CommonMoverTopology::from_metadata(13, &type47_shape),
            Err(GklCommonMoverTopologyError::UnsupportedComponentTopology)
        );

        let mut unresolved_gkl = metadata.clone();
        unresolved_gkl.common_mover_gkl_payloads = RetailRuntimeValue::Unresolved;
        Type13CommonMoverTopology::from_metadata(13, &unresolved_gkl)
            .expect("G/K/L payloads are not read until component dispatch");
        let profile = GklCommonMoverProfile {
            topology: TYPE13_SEARCH_ATTACK_TOPOLOGY,
            sub_d: TYPE13_SEARCH_ATTACK_SUB_D,
            gkl: TYPE13_SEARCH_ATTACK_GKL,
        };
        assert_eq!(
            profile.authenticate_gkl_payloads(&unresolved_gkl),
            Err(GklCommonMoverTopologyError::UnresolvedGklPayloads)
        );

        let mut wrong_gkl = metadata;
        wrong_gkl.common_mover_gkl_payloads =
            RetailRuntimeValue::Known(crate::entity_collision_state::CommonMoverGklPayloads {
                sub_g: TYPE13_SEARCH_ATTACK_GKL.sub_g,
                sub_k: None,
                sub_l: TYPE13_SEARCH_ATTACK_GKL.sub_l,
            });
        Type13CommonMoverTopology::from_metadata(13, &wrong_gkl)
            .expect("component topology remains independently authentic");
        assert_eq!(
            profile.authenticate_gkl_payloads(&wrong_gkl),
            Err(GklCommonMoverTopologyError::UnsupportedGklPayloads)
        );
    }

    #[test]
    fn dispatch_mode_selects_g_only_or_g_then_k_then_l() {
        assert_eq!(
            CommonMoverDispatchPlan::from_topology(
                TYPE13_SEARCH_ATTACK_TOPOLOGY,
                CommonMoverDispatchMode::Restricted,
            )
            .phases(),
            [Some(CommonMoverDispatchPhase::SubG), None, None]
        );
        assert_eq!(
            CommonMoverDispatchPlan::from_topology(
                TYPE13_SEARCH_ATTACK_TOPOLOGY,
                CommonMoverDispatchMode::Normal,
            )
            .phases(),
            [
                Some(CommonMoverDispatchPhase::SubG),
                Some(CommonMoverDispatchPhase::SubK),
                Some(CommonMoverDispatchPhase::SubL),
            ]
        );
    }

    #[test]
    fn rejects_a_non_type13_sub_d_descriptor() {
        let mut metadata = type13_metadata();
        metadata.sub_d_steering_descriptor = RetailRuntimeValue::Known(Some(ORDINARY_TYPE9_SUB_D));
        let target_private = WanderNearPrivateState::tracked_entity([0, 0, 0], 0x047F_0001);
        let terrain = flat_terrain();
        let result = evaluate_type13_common_mover(
            GklCommonMoverRequest {
                entity_id: 0x04fc_0001,
                entity_type: 13,
                metadata: Some(&metadata),
                dispatch_mode: CommonMoverDispatchMode::Restricted,
                position_raw: [0, 0, 0],
                velocity_raw: [10, 0, 0],
                heading_raw: 0,
                pitch_raw: 0,
                roll_raw: 0,
                body_basis: RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(0, 0, 0)),
                runtime: GklCommonMoverRuntime::from_intro2_constructor_capture(),
                sub_g_runtime: type13_sub_g_runtime(),
                target_private,
                tracked_target: RetailRuntimeValue::Known(Some(CommonMoverTrackedTargetSnapshot {
                    state_flags: crate::entity_collision_state::RetailStateWord::exact(4),
                    position_raw: [1_000, 0, 0],
                    velocity_raw: [0; 3],
                })),
                terrain: &terrain,
                active_model_extent_raw: 480,
                self_mass_raw: 100,
                attached_cargo_mass: 0,
                capability_flags: 8,
                retail_tick: 0,
                elapsed_micros: 19_500,
                global_elapsed_micros: 19_500,
            },
            || panic!("Type-9 Sub-D must not start FUN_00401430"),
        );
        assert_eq!(result, Err(GklCommonMoverBlock::UnexpectedSubD));
        assert_eq!(TYPE13_SEARCH_ATTACK_SUB_D.classifier_flags, 0);
        assert_eq!(ORDINARY_TYPE9_SUB_D.classifier_flags, 0x17);
    }

    #[test]
    fn commits_classifier_free_prelude_and_complete_selected_component_dispatch() {
        for dispatch_mode in [
            CommonMoverDispatchMode::Restricted,
            CommonMoverDispatchMode::Normal,
        ] {
            let metadata = type13_metadata();
            let terrain = flat_terrain();
            let target_private = WanderNearPrivateState::tracked_entity([0, 0, 0], 0x047F_0001);
            let original_target_private = target_private;
            let runtime = GklCommonMoverRuntime::from_intro2_constructor_capture();
            let original_runtime = runtime;
            let retained_body_basis = Type9BodyBasis::from_angle_words(0, 0, 0);
            let result = evaluate_type13_common_mover(
                GklCommonMoverRequest {
                    entity_id: 0x04fc_0001,
                    entity_type: 13,
                    metadata: Some(&metadata),
                    dispatch_mode,
                    position_raw: [0, 0, 0],
                    velocity_raw: [10, 20, 30],
                    heading_raw: 0,
                    pitch_raw: 0,
                    roll_raw: 0,
                    body_basis: RetailRuntimeValue::Known(retained_body_basis),
                    runtime,
                    sub_g_runtime: type13_sub_g_runtime(),
                    target_private,
                    tracked_target: RetailRuntimeValue::Known(Some(
                        CommonMoverTrackedTargetSnapshot {
                            state_flags: crate::entity_collision_state::RetailStateWord::exact(4),
                            position_raw: [0, 0, 1_000],
                            velocity_raw: [0; 3],
                        },
                    )),
                    terrain: &terrain,
                    active_model_extent_raw: 480,
                    self_mass_raw: 100,
                    attached_cargo_mass: 0,
                    capability_flags: 8,
                    retail_tick: 0,
                    elapsed_micros: 19_500,
                    global_elapsed_micros: 19_500,
                },
                || panic!("the classifier-free Type-13 prefix consumes no RNG"),
            );
            let staged = result.expect("Type-13 normal Sub-G is statically closed");
            assert_eq!(staged.result, ChaseTargetCommonMoverReturn::NonZero);
            assert_eq!(staged.position_raw, [0, 0, 0]);
            assert_eq!(staged.velocity_raw, [10, 20, 30]);
            assert_eq!(staged.heading_raw, 153);
            assert_eq!(staged.pitch_raw, 986);
            assert_eq!(staged.roll_raw, 142);
            assert_eq!(staged.sound, None);
            assert_eq!(staged.target_private.target_position_raw, [0, 0, 1_000]);
            assert_eq!(staged.runtime.sub_d_runtime.yaw_rate_raw, -1_024);
            assert_eq!(staged.runtime.sub_d_runtime.last_yaw_step_raw, -153);
            assert_eq!(
                staged
                    .runtime
                    .sub_d_frame_owner
                    .classifier_cache()
                    .stagger_counter(),
                1
            );
            assert_eq!(
                staged.runtime.sub_d_frame_owner.classifier_cache().rows(),
                [0; 8]
            );
            assert_eq!(staged.runtime.sub_k_smoothed_raw, -10);
            assert_eq!(staged.runtime.sub_l_target_raw, [0, 0, 1_000]);
            assert_eq!(staged.runtime.sub_l_exact_raw, -153);
            match dispatch_mode {
                CommonMoverDispatchMode::Restricted => {
                    assert_eq!(staged.runtime.sub_k_output_raw, [0; 2]);
                    assert_eq!(staged.runtime.sub_l_output_raw, [0; 2]);
                }
                CommonMoverDispatchMode::Normal => {
                    assert_eq!(staged.runtime.sub_k_output_raw, [40, -40]);
                    assert_eq!(staged.runtime.sub_l_output_raw, [-918, 0]);
                }
            }
            assert_eq!(
                staged.sub_g_runtime.rate_raw_at_0x34(),
                RetailRuntimeValue::Known(107)
            );
            assert_eq!(
                staged.sub_g_runtime.source_raw_at_0x20(),
                RetailRuntimeValue::Known(0)
            );

            assert_eq!(runtime, original_runtime);
            assert_eq!(target_private, original_target_private);
        }
    }

    #[test]
    fn normal_dispatch_sub_k_reads_velocity_after_sub_g_force() {
        let metadata = type13_metadata();
        let terrain = flat_terrain();
        let mut runtime = GklCommonMoverRuntime::from_intro2_constructor_capture();
        let mut sub_g_runtime = type13_sub_g_runtime();
        let mut target_private = WanderNearPrivateState::tracked_entity([0; 3], 0x047f_0001);
        let mut velocity_raw = [10, 20, 30];
        let mut heading_raw = 0;
        let mut pitch_raw = 0;
        let mut roll_raw = 0;
        let mut observed_force = false;
        for frame in 0..15 {
            let staged = evaluate_type13_common_mover(
                GklCommonMoverRequest {
                    entity_id: 0x04fc_0001,
                    entity_type: 13,
                    metadata: Some(&metadata),
                    dispatch_mode: CommonMoverDispatchMode::Normal,
                    position_raw: [0; 3],
                    velocity_raw,
                    heading_raw,
                    pitch_raw,
                    roll_raw,
                    body_basis: RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(
                        0, 0, 0,
                    )),
                    runtime,
                    sub_g_runtime,
                    target_private,
                    tracked_target: RetailRuntimeValue::Known(Some(
                        CommonMoverTrackedTargetSnapshot {
                            state_flags: crate::entity_collision_state::RetailStateWord::exact(4),
                            position_raw: [0, 0, 1_000],
                            velocity_raw: [0; 3],
                        },
                    )),
                    terrain: &terrain,
                    active_model_extent_raw: 480,
                    self_mass_raw: 100,
                    attached_cargo_mass: 0,
                    capability_flags: 8,
                    retail_tick: frame,
                    elapsed_micros: 32_768,
                    global_elapsed_micros: 32_768,
                },
                || panic!("classifier-free target tracking consumes no RNG"),
            )
            .expect("detailed G/K/L dispatch is closed");
            if staged.velocity_raw[1] != velocity_raw[1] {
                let expected = ((i32::from(runtime.sub_k_output_raw[1]) * 3
                    - i32::from(staged.velocity_raw[1]) * 8)
                    / 4) as i16;
                let stale_velocity_result = ((i32::from(runtime.sub_k_output_raw[1]) * 3
                    - i32::from(velocity_raw[1]) * 8)
                    / 4) as i16;
                assert_eq!(
                    staged.runtime.sub_k_output_raw[1],
                    expected.clamp(-0x1200, 0x1200)
                );
                assert_ne!(
                    staged.runtime.sub_k_output_raw[1],
                    stale_velocity_result.clamp(-0x1200, 0x1200)
                );
                observed_force = true;
                break;
            }
            runtime = staged.runtime;
            sub_g_runtime = staged.sub_g_runtime;
            target_private = staged.target_private;
            velocity_raw = staged.velocity_raw;
            heading_raw = staged.heading_raw;
            pitch_raw = staged.pitch_raw;
            roll_raw = staged.roll_raw;
        }
        assert!(
            observed_force,
            "fixture must reach the authored Sub-G force phase"
        );
    }
}
