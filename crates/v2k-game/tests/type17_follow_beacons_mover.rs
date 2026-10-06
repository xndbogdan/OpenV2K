use v2k_formats::collision::SubAPropulsionDescriptor;
use v2k_formats::terrain::{TerrainCell, TerrainGrid, GRID_SIZE};
use v2k_game::{
    common_mover::{
        component_dispatch::{
            CommonMoverDispatchMode, CommonMoverDispatchPhase, CommonMoverDispatchPlan,
        },
        sub_d::{
            type17_first_query_owner_for_seed, Type9SubDRuntime, ORDINARY_TYPE9_SUB_D, TYPE47_SUB_D,
        },
        target_prelude::CommonMoverTrackedTargetSnapshot,
        SubAPropulsionRuntime,
    },
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    ordinary_type47_live::{
        FRESH_LEVEL1_ORDINARY_TYPE47_COMPONENT_TOPOLOGY, FRESH_LEVEL1_ORDINARY_TYPE47_SUB_D_SEEDS,
    },
    type17_follow_beacons_mover::{
        evaluate_type17_follow_beacons_common_mover, Type17FollowBeaconsMoverBlock,
        Type17FollowBeaconsMoverRequest, Type17FollowBeaconsMoverTopology,
        Type17FollowBeaconsMoverTopologyError, TYPE17_FOLLOW_BEACONS_SCHEDULER_MODE,
    },
    type17_impact_live::{
        TYPE17_MODEL256_COMPONENT_TOPOLOGY, TYPE17_MODEL256_SUB_A_TARGET_SPEED_BASE_RAW,
        TYPE17_MODEL256_SUB_D,
    },
    wander_near_location::WanderNearPrivateState,
};

fn empty_terrain() -> TerrainGrid {
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

fn type17_metadata() -> EntityTypeRuntimeMetadata {
    let mut metadata = EntityTypeRuntimeMetadata::default();
    metadata.common_mover_topology = RetailRuntimeValue::Known(TYPE17_MODEL256_COMPONENT_TOPOLOGY);
    metadata.sub_d_steering_descriptor = RetailRuntimeValue::Known(Some(TYPE17_MODEL256_SUB_D));
    metadata.sub_a_propulsion_descriptor =
        RetailRuntimeValue::Known(Some(SubAPropulsionDescriptor {
            acceleration_raw: 100,
            overspeed_correction_raw: 200,
            target_speed_base_raw: TYPE17_MODEL256_SUB_A_TARGET_SPEED_BASE_RAW,
        }));
    metadata
}

#[test]
fn admits_only_the_authored_type17_component_route() {
    let metadata = type17_metadata();
    assert!(Type17FollowBeaconsMoverTopology::from_metadata(17, &metadata).is_ok());
    assert_eq!(
        Type17FollowBeaconsMoverTopology::from_metadata(47, &metadata),
        Err(Type17FollowBeaconsMoverTopologyError::WrongEntityType { actual: 47 })
    );

    let mut type47_shape = metadata;
    type47_shape.common_mover_topology =
        RetailRuntimeValue::Known(FRESH_LEVEL1_ORDINARY_TYPE47_COMPONENT_TOPOLOGY);
    assert_eq!(
        Type17FollowBeaconsMoverTopology::from_metadata(17, &type47_shape),
        Err(Type17FollowBeaconsMoverTopologyError::UnsupportedComponentTopology)
    );
}

#[test]
fn ordinary_mode_0_selects_type17_sub_h() {
    assert_eq!(
        CommonMoverDispatchMode::from_retail_word(TYPE17_FOLLOW_BEACONS_SCHEDULER_MODE),
        CommonMoverDispatchMode::Normal
    );
    assert_eq!(
        CommonMoverDispatchPlan::from_topology(
            TYPE17_MODEL256_COMPONENT_TOPOLOGY,
            CommonMoverDispatchMode::Normal,
        )
        .phases(),
        [Some(CommonMoverDispatchPhase::SubH), None, None]
    );
}

#[test]
fn authored_0x13_flags_reject_type9_first_query_reset() {
    let mut metadata = type17_metadata();
    metadata.sub_d_steering_descriptor = RetailRuntimeValue::Known(Some(ORDINARY_TYPE9_SUB_D));
    let mut target_private = WanderNearPrivateState::tracked_entity([0, 0, 0], 0x047F_0001);
    let result = evaluate_type17_follow_beacons_common_mover(
        Type17FollowBeaconsMoverRequest {
            entity_id: 0x0497_0001,
            entity_type: 17,
            metadata: Some(&metadata),
            position_raw: [0, 0, 0],
            velocity_raw: [10, 0, 0],
            heading_raw: 0,
            roll_raw: 0,
            sub_a_runtime: RetailRuntimeValue::Known(Some(
                SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Known(250), 1, 333),
            )),
            sub_d_stagger_seed: None,
            sub_d_frame_owner: None,
            sub_d_runtime: None,
            body_right_q31: RetailRuntimeValue::Unresolved,
            body_forward_q31: RetailRuntimeValue::Unresolved,
            body_up_q31: RetailRuntimeValue::Unresolved,
            sub_h_runtime: None,
            terrain: None,
            target_private: &mut target_private,
            tracked_target: RetailRuntimeValue::Known(Some(CommonMoverTrackedTargetSnapshot {
                state_flags: v2k_game::entity_collision_state::RetailStateWord::exact(4),
                position_raw: [1_000, 0, 0],
                velocity_raw: [0; 3],
            })),
            elapsed_micros: 19_500,
            global_elapsed_micros: 19_500,
        },
        || panic!("Type-9 Sub-D must not start FUN_00401430"),
    );
    assert_eq!(result, Err(Type17FollowBeaconsMoverBlock::UnexpectedSubD));
    assert_eq!(TYPE17_MODEL256_SUB_D.classifier_flags, 0x13);
    assert_eq!(ORDINARY_TYPE9_SUB_D.classifier_flags, 0x17);
}

#[test]
fn starts_fun_00401430_and_blocks_on_type17_sub_d() {
    let metadata = type17_metadata();
    let mut target_private = WanderNearPrivateState::tracked_entity([0, 0, 0], 0x047F_0001);
    let result = evaluate_type17_follow_beacons_common_mover(
        Type17FollowBeaconsMoverRequest {
            entity_id: 0x0497_0001,
            entity_type: 17,
            metadata: Some(&metadata),
            position_raw: [0, 0, 0],
            velocity_raw: [10, 0, 0],
            heading_raw: 0,
            roll_raw: 0,
            sub_a_runtime: RetailRuntimeValue::Known(Some(
                SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Known(250), 1, 333),
            )),
            sub_d_stagger_seed: None,
            sub_d_frame_owner: None,
            sub_d_runtime: None,
            body_right_q31: RetailRuntimeValue::Unresolved,
            body_forward_q31: RetailRuntimeValue::Unresolved,
            body_up_q31: RetailRuntimeValue::Unresolved,
            sub_h_runtime: None,
            terrain: None,
            target_private: &mut target_private,
            tracked_target: RetailRuntimeValue::Known(Some(CommonMoverTrackedTargetSnapshot {
                state_flags: v2k_game::entity_collision_state::RetailStateWord::exact(4),
                position_raw: [1_000, 0, 0],
                velocity_raw: [0; 3],
            })),
            elapsed_micros: 19_500,
            global_elapsed_micros: 19_500,
        },
        || 0,
    );
    assert_eq!(
        result,
        Err(Type17FollowBeaconsMoverBlock::SubDFirstQueryUnavailable)
    );
}

#[test]
fn spawn18_seed_applies_the_v200002_first_query_then_needs_sub_h() {
    assert_eq!(TYPE17_MODEL256_SUB_D, TYPE47_SUB_D);
    let metadata = type17_metadata();
    let terrain = empty_terrain();
    let mut owner = type17_first_query_owner_for_seed(0x32).expect("spawn-18 seed");
    let mut runtime = Type9SubDRuntime::from_constructor();
    let mut target_private = WanderNearPrivateState::tracked_entity([0, 0, 0], 0x047F_0001);
    let result = evaluate_type17_follow_beacons_common_mover(
        Type17FollowBeaconsMoverRequest {
            entity_id: 0x0497_0001,
            entity_type: 17,
            metadata: Some(&metadata),
            position_raw: [0, 0, 0],
            velocity_raw: [10, 0, 0],
            heading_raw: 0,
            roll_raw: 0,
            sub_a_runtime: RetailRuntimeValue::Known(Some(
                SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Known(250), 1, 333),
            )),
            sub_d_stagger_seed: Some(0x32),
            sub_d_frame_owner: Some(&mut owner),
            sub_d_runtime: Some(&mut runtime),
            body_right_q31: RetailRuntimeValue::Known([i32::MAX, 0, 0]),
            body_forward_q31: RetailRuntimeValue::Known([0, 0, i32::MAX]),
            body_up_q31: RetailRuntimeValue::Known([0, i32::MAX, 0]),
            sub_h_runtime: None,
            terrain: Some(&terrain),
            target_private: &mut target_private,
            tracked_target: RetailRuntimeValue::Known(Some(CommonMoverTrackedTargetSnapshot {
                state_flags: v2k_game::entity_collision_state::RetailStateWord::exact(4),
                position_raw: [1_000, 0, 0],
                velocity_raw: [0; 3],
            })),
            elapsed_micros: 19_500,
            global_elapsed_micros: 19_500,
        },
        || 0,
    );
    assert_eq!(result, Err(Type17FollowBeaconsMoverBlock::SubHUnavailable));
    assert!(matches!(
        owner.classifier_cache().origin(),
        RetailRuntimeValue::Known(_)
    ));
    assert!(
        owner.classifier_cache().rows().iter().any(|row| *row != 0),
        "0x13 classification must fill at least one cache nibble"
    );
}

#[test]
fn level1_type47_seeds_cannot_apply_type17_first_query() {
    let metadata = type17_metadata();
    let terrain = empty_terrain();
    let mut owner = type17_first_query_owner_for_seed(0x32).expect("type-17 seed");
    let mut target_private = WanderNearPrivateState::tracked_entity([0, 0, 0], 0x047F_0001);
    let result = evaluate_type17_follow_beacons_common_mover(
        Type17FollowBeaconsMoverRequest {
            entity_id: 0x042F_000B,
            entity_type: 17,
            metadata: Some(&metadata),
            position_raw: [0, 0, 0],
            velocity_raw: [10, 0, 0],
            heading_raw: 0,
            roll_raw: 0,
            sub_a_runtime: RetailRuntimeValue::Known(Some(
                SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Known(250), 1, 333),
            )),
            sub_d_stagger_seed: Some(FRESH_LEVEL1_ORDINARY_TYPE47_SUB_D_SEEDS[0]),
            sub_d_frame_owner: Some(&mut owner),
            sub_d_runtime: None,
            body_right_q31: RetailRuntimeValue::Unresolved,
            body_forward_q31: RetailRuntimeValue::Unresolved,
            body_up_q31: RetailRuntimeValue::Unresolved,
            sub_h_runtime: None,
            terrain: Some(&terrain),
            target_private: &mut target_private,
            tracked_target: RetailRuntimeValue::Known(Some(CommonMoverTrackedTargetSnapshot {
                state_flags: v2k_game::entity_collision_state::RetailStateWord::exact(4),
                position_raw: [1_000, 0, 0],
                velocity_raw: [0; 3],
            })),
            elapsed_micros: 19_500,
            global_elapsed_micros: 19_500,
        },
        || 0,
    );
    assert_eq!(
        result,
        Err(Type17FollowBeaconsMoverBlock::SubDFirstQueryUnavailable)
    );
    assert_eq!(
        owner.classifier_cache().origin(),
        RetailRuntimeValue::Unresolved
    );
}
