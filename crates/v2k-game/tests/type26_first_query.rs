use v2k_formats::collision::CommonAxisDescriptor;
use v2k_formats::terrain::{TerrainCell, TerrainGrid, GRID_SIZE};
use v2k_game::chase_target::ChaseTargetCommonMoverReturn;
use v2k_game::common_mover::component_dispatch::CommonMoverDispatchMode;
use v2k_game::common_mover::sub_d::{
    intro2_type26_first_query_owner_for_seed, intro2_type47_first_query_owner_for_seed,
    level1_type47_first_query_owner_for_seed, type17_first_query_owner_for_seed,
    type8_first_query_owner_for_seed, Type9SubDFrameOwner, Type9SubDRuntime, FLYER_SUB_D,
    INTRO2_TYPE26_FIRST_QUERY_SEEDS, INTRO2_TYPE26_SUB_D, INTRO2_TYPE47_FIRST_QUERY_SEEDS,
    TYPE17_FIRST_QUERY_SEEDS, TYPE47_SUB_D, TYPE8_FIRST_QUERY_SEEDS,
    TYPE8_MAIN_BASE_CONVERSION_SUB_D_SEED,
};
use v2k_game::common_mover::target_prelude::CommonMoverTrackedTargetSnapshot;
use v2k_game::common_mover::type9_attitude::Type9BodyBasis;
use v2k_game::common_mover::SubAPropulsionRuntime;
use v2k_game::defecate_virus::DefecateVirusCallbackPlan;
use v2k_game::entity::{Entity, EntityKind};
use v2k_game::entity_collision_state::{
    EntityInitializerSpec, EntityTypeRuntimeMetadata, RetailRuntimeValue, RetailStateWord,
};
use v2k_game::infection_evolution::InfectionCellWrite;
use v2k_game::intro2_type26_defecate_virus::{
    intro2_type26_model_extent_raw_by_state, publish_intro2_type26_defecate_virus,
    retain_intro2_type26_physical_body_basis, tick_intro2_type26_defecate_virus_primary,
    tick_intro2_type26_defecate_virus_tertiary, tick_intro2_type26_scheduler_owner_on_entity,
    Intro2Type26OwnerTransition, Intro2Type26PrimaryVisitBlock, Intro2Type26PrimaryVisitResult,
    Intro2Type26SchedulerOwner, Intro2Type26SchedulerProductionOutcome,
    Intro2Type26TertiaryVisitBlock, Intro2Type26TertiaryVisitResult,
    INTRO2_DEFECATE_VIRUS_ENTITY_TYPE, INTRO2_DEFECATE_VIRUS_MODEL_ID,
    INTRO2_DEFECATE_VIRUS_SPAWN_INDEX, INTRO2_TYPE26_BE00_0F00_SUB_D_SEED,
    INTRO2_TYPE26_BEHAVIOR_CHOICES, INTRO2_TYPE26_COMMON_MOVER_TOPOLOGY,
    INTRO2_TYPE26_SPAWN10_RETAIL_HANDLE, INTRO2_TYPE26_SPAWN25_SUB_D_SEED, INTRO2_TYPE26_SUB_A,
    INTRO2_TYPE26_SUB_B, INTRO2_TYPE26_SUB_C,
};
use v2k_game::intro2_type47_common_mover::{
    evaluate_intro2_type47_common_mover, Intro2Type47CommonMoverBlock,
    Intro2Type47CommonMoverRequest, INTRO2_TYPE47_TTD_HANDLES, INTRO2_TYPE47_TTD_POSITIONS_RAW,
};
use v2k_game::ordinary_type47_live::FRESH_LEVEL1_ORDINARY_TYPE47_SUB_D_SEEDS;
use v2k_game::sub_g_runtime::SubG06070RuntimeState;
use v2k_game::type13_common_mover::{
    evaluate_type13_common_mover, GklCommonMoverRequest, GklCommonMoverRuntime,
};
use v2k_game::type26_common_mover::{
    evaluate_type26_common_mover, Type26CommonMoverBlock, Type26CommonMoverRequest,
};
use v2k_game::type47_chase_mover::{
    evaluate_type47_chase_common_mover, Type47ChaseMoverBlock, Type47ChaseMoverRequest,
};
use v2k_game::wander_near_location::WanderNearPrivateState;

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

fn type13_sub_g_runtime() -> SubG06070RuntimeState {
    let mut runtime = SubG06070RuntimeState::from_1b8c0_constructor(
        &v2k_game::search_attack_live::TYPE13_SEARCH_ATTACK_SUB_G,
        0,
    );
    runtime.apply_shared_06070_sub_g_branch(217, 0);
    runtime
}

fn type26_metadata() -> EntityTypeRuntimeMetadata {
    let mut metadata = EntityTypeRuntimeMetadata::default();
    metadata.common_mover_topology = RetailRuntimeValue::Known(INTRO2_TYPE26_COMMON_MOVER_TOPOLOGY);
    metadata.sub_a_propulsion_descriptor = RetailRuntimeValue::Known(Some(INTRO2_TYPE26_SUB_A));
    metadata.sub_b_lateral_descriptor = RetailRuntimeValue::Known(Some(INTRO2_TYPE26_SUB_B));
    metadata.sub_c_lift_descriptor = RetailRuntimeValue::Known(Some(INTRO2_TYPE26_SUB_C));
    metadata.sub_d_steering_descriptor = RetailRuntimeValue::Known(Some(INTRO2_TYPE26_SUB_D));
    metadata
}

#[test]
fn spawn25_applies_type26_cab_tail_and_returns_one() {
    let metadata = type26_metadata();
    let terrain = empty_terrain();
    let mut owner = intro2_type26_first_query_owner_for_seed(INTRO2_TYPE26_SPAWN25_SUB_D_SEED)
        .expect("spawn-25 seed");
    let mut runtime = Type9SubDRuntime::from_constructor();
    let mut target_private = WanderNearPrivateState::tracked_entity([0, 0, 0], 0x047F_0001);
    let result = evaluate_type26_common_mover(
        Type26CommonMoverRequest {
            entity_id: 0x047E_0001,
            entity_type: 26,
            metadata: Some(&metadata),
            position_raw: [0xC200u16 as i16, 0, 0x0E00],
            velocity_raw: [10, 0, 0],
            heading_raw: 0,
            roll_raw: 0,
            sub_a_runtime: RetailRuntimeValue::Known(Some(
                v2k_game::common_mover::SubAPropulsionRuntime::from_retail_words(
                    RetailRuntimeValue::Known(1),
                    1,
                    100,
                ),
            )),
            sub_d_stagger_seed: Some(INTRO2_TYPE26_SPAWN25_SUB_D_SEED),
            sub_d_frame_owner: Some(&mut owner),
            sub_d_runtime: Some(&mut runtime),
            body_right_q31: RetailRuntimeValue::Known([i32::MAX, 0, 0]),
            body_forward_q31: RetailRuntimeValue::Known([0, 0, i32::MAX]),
            body_up_q31: RetailRuntimeValue::Known([0, i32::MAX, 0]),
            terrain: Some(&terrain),
            global_elapsed_micros: 19_500,
            target_private: &mut target_private,
            tracked_target: RetailRuntimeValue::Known(Some(CommonMoverTrackedTargetSnapshot {
                state_flags: v2k_game::entity_collision_state::RetailStateWord::exact(4),
                position_raw: [1_000, 0, 0],
                velocity_raw: [0; 3],
            })),
            elapsed_micros: 19_500,
        },
        || 0,
    );
    assert_eq!(
        result.as_ref().map(|outcome| outcome.result),
        Ok(ChaseTargetCommonMoverReturn::NonZero)
    );
    assert!(matches!(
        owner.classifier_cache().origin(),
        RetailRuntimeValue::Known(_)
    ));
    assert!(
        owner.classifier_cache().rows().iter().any(|row| *row != 0),
        "0x12 classification must fill at least one cache nibble"
    );
    assert_ne!(runtime.yaw_rate_raw, 0);
}

#[test]
fn spawn10_applies_type26_cab_tail_and_returns_one() {
    let metadata = type26_metadata();
    let terrain = empty_terrain();
    let mut owner = intro2_type26_first_query_owner_for_seed(INTRO2_TYPE26_BE00_0F00_SUB_D_SEED)
        .expect("spawn-10 seed");
    let mut runtime = Type9SubDRuntime::from_constructor();
    let mut target_private = WanderNearPrivateState::tracked_entity([0, 0, 0], 0x047F_0001);
    let result = evaluate_type26_common_mover(
        Type26CommonMoverRequest {
            entity_id: INTRO2_TYPE26_SPAWN10_RETAIL_HANDLE,
            entity_type: 26,
            metadata: Some(&metadata),
            position_raw: [0xBE00u16 as i16, 0, 0x0F00],
            velocity_raw: [10, 0, 0],
            heading_raw: -0x8000i16 as u16,
            roll_raw: 0,
            sub_a_runtime: RetailRuntimeValue::Known(Some(
                v2k_game::common_mover::SubAPropulsionRuntime::from_retail_words(
                    RetailRuntimeValue::Known(1),
                    1,
                    100,
                ),
            )),
            sub_d_stagger_seed: Some(INTRO2_TYPE26_BE00_0F00_SUB_D_SEED),
            sub_d_frame_owner: Some(&mut owner),
            sub_d_runtime: Some(&mut runtime),
            body_right_q31: RetailRuntimeValue::Known([0, 0, i32::MAX]),
            body_forward_q31: RetailRuntimeValue::Known([-i32::MAX, 0, 0]),
            body_up_q31: RetailRuntimeValue::Known([0, i32::MAX, 0]),
            terrain: Some(&terrain),
            global_elapsed_micros: 19_500,
            target_private: &mut target_private,
            tracked_target: RetailRuntimeValue::Known(Some(CommonMoverTrackedTargetSnapshot {
                state_flags: v2k_game::entity_collision_state::RetailStateWord::exact(4),
                position_raw: [1_000, 0, 0],
                velocity_raw: [0; 3],
            })),
            elapsed_micros: 19_500,
        },
        || 0,
    );
    assert_eq!(
        result.as_ref().map(|outcome| outcome.result),
        Ok(ChaseTargetCommonMoverReturn::NonZero)
    );
    assert!(matches!(
        owner.classifier_cache().origin(),
        RetailRuntimeValue::Known(_)
    ));
    assert!(
        owner.classifier_cache().rows().iter().any(|row| *row != 0),
        "0x12 classification must fill at least one cache nibble"
    );
    assert_ne!(runtime.yaw_rate_raw, 0);
}

#[test]
fn type13_bypasses_queries_while_level1_type47_stays_fail_closed() {
    assert!(intro2_type26_first_query_owner_for_seed(0).is_none());
    assert!(intro2_type47_first_query_owner_for_seed(0x2B).is_none());
    assert!(intro2_type47_first_query_owner_for_seed(0x2C).is_none());
    assert!(intro2_type47_first_query_owner_for_seed(0x2D).is_none());
    assert!(type17_first_query_owner_for_seed(0x2B).is_none());
    assert!(type8_first_query_owner_for_seed(0x2B).is_none());
    assert_eq!(INTRO2_TYPE26_FIRST_QUERY_SEEDS, [0x0A, 0x15]);
    assert_eq!(INTRO2_TYPE47_FIRST_QUERY_SEEDS, [0x06, 0x07, 0x08]);
    assert_eq!(
        TYPE17_FIRST_QUERY_SEEDS,
        [0x04, 0x17, 0x31, 0x32, 0x33, 0x34]
    );
    assert_eq!(TYPE8_FIRST_QUERY_SEEDS, [0x0C, 0x0D, 0x0E, 0x38]);
    assert_eq!(TYPE8_MAIN_BASE_CONVERSION_SUB_D_SEED, 0x38);
    assert!(type17_first_query_owner_for_seed(0x32).is_some());
    assert!(type8_first_query_owner_for_seed(0x38).is_some());

    let mut metadata = EntityTypeRuntimeMetadata::default();
    metadata.common_mover_topology =
        RetailRuntimeValue::Known(v2k_game::search_attack_live::TYPE13_SEARCH_ATTACK_TOPOLOGY);
    metadata.common_mover_gkl_payloads =
        RetailRuntimeValue::Known(v2k_game::search_attack_live::TYPE13_SEARCH_ATTACK_GKL);
    metadata.sub_d_steering_descriptor = RetailRuntimeValue::Known(Some(
        v2k_game::search_attack_live::TYPE13_SEARCH_ATTACK_SUB_D,
    ));
    let target_private = WanderNearPrivateState::tracked_entity([0, 0, 0], 0x047F_0001);
    let runtime = GklCommonMoverRuntime::from_intro2_constructor_capture();
    let body_basis = Type9BodyBasis::from_angle_words(0, 0, 0);
    let terrain = empty_terrain();
    let type13 = evaluate_type13_common_mover(
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
            body_basis: RetailRuntimeValue::Known(body_basis),
            runtime,
            sub_g_runtime: type13_sub_g_runtime(),
            target_private,
            tracked_target: RetailRuntimeValue::Known(Some(CommonMoverTrackedTargetSnapshot {
                state_flags: v2k_game::entity_collision_state::RetailStateWord::exact(4),
                position_raw: [0, 0, 1_000],
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
        || panic!("classifier-free Type-13 Sub-D must not draw RNG"),
    );
    let outcome = type13.expect("Type-13 completes its classifier-free Sub-D and normal Sub-G");
    assert_eq!(outcome.result, ChaseTargetCommonMoverReturn::NonZero);
    assert_eq!(outcome.position_raw, [0, 0, 0]);
    assert_eq!(outcome.velocity_raw, [10, 0, 0]);
    assert_eq!(outcome.heading_raw, 153);
    assert_eq!(outcome.pitch_raw, 986);
    assert_eq!(outcome.roll_raw, 142);
    assert_eq!(outcome.sound, None);
    assert_eq!(outcome.target_private.target_position_raw, [0, 0, 1_000]);
    assert_eq!(outcome.runtime.sub_d_runtime.yaw_rate_raw, -1_024);
    assert_eq!(outcome.runtime.sub_d_runtime.last_yaw_step_raw, -153);
    assert_eq!(
        outcome
            .runtime
            .sub_d_frame_owner
            .classifier_cache()
            .stagger_counter(),
        1
    );
    assert_eq!(outcome.runtime.sub_k_smoothed_raw, -10);
    assert_eq!(outcome.runtime.sub_l_target_raw, [0, 0, 1_000]);
    assert_eq!(outcome.runtime.sub_l_exact_raw, -153);
    assert_eq!(
        outcome.sub_g_runtime.rate_raw_at_0x34(),
        RetailRuntimeValue::Known(107)
    );
    assert_eq!(
        outcome.sub_g_runtime.source_raw_at_0x20(),
        RetailRuntimeValue::Known(0)
    );
    assert_eq!(
        runtime,
        GklCommonMoverRuntime::from_intro2_constructor_capture()
    );
    assert_eq!(
        target_private,
        WanderNearPrivateState::tracked_entity([0, 0, 0], 0x047F_0001)
    );

    let mut type47_metadata = EntityTypeRuntimeMetadata::default();
    type47_metadata.common_mover_topology = RetailRuntimeValue::Known(
        v2k_game::ordinary_type47_live::FRESH_LEVEL1_ORDINARY_TYPE47_COMPONENT_TOPOLOGY,
    );
    type47_metadata.sub_a_propulsion_descriptor = RetailRuntimeValue::Known(Some(
        v2k_game::ordinary_type47_death_live::TYPE47_COMMON_DYING_SUB_A_DESCRIPTOR,
    ));
    type47_metadata.sub_b_lateral_descriptor = RetailRuntimeValue::Known(Some(
        v2k_game::ordinary_type47_death_live::TYPE47_COMMON_DYING_SUB_B_DESCRIPTOR,
    ));
    type47_metadata.sub_c_lift_descriptor = RetailRuntimeValue::Known(Some(
        v2k_game::ordinary_type47_death_live::TYPE47_COMMON_DYING_SUB_C_DESCRIPTOR,
    ));
    type47_metadata.sub_d_steering_descriptor = RetailRuntimeValue::Known(Some(
        v2k_game::ordinary_type47_live::FRESH_LEVEL1_ORDINARY_TYPE47_SUB_D,
    ));
    type47_metadata.sub_h_external_frame_descriptor =
        RetailRuntimeValue::Known(Some(v2k_formats::collision::SubHExternalFrameDescriptor {
            completion_sound_id: None,
            records: v2k_game::ordinary_type47_death_live::TYPE47_COMMON_DYING_SUB_H_RECORDS
                .to_vec(),
        }));
    let mut target_private = WanderNearPrivateState::tracked_entity([0, 0, 0], 0x047F_0001);
    let type47 = evaluate_type47_chase_common_mover(
        Type47ChaseMoverRequest {
            entity_id: 0x042F_000B,
            entity_type: 47,
            metadata: Some(&type47_metadata),
            position_raw: [0, 0, 0],
            velocity_raw: [10, 0, 0],
            heading_raw: 0,
            roll_raw: 0,
            sub_a_runtime: RetailRuntimeValue::Known(Some(
                v2k_game::common_mover::SubAPropulsionRuntime::from_retail_words(
                    RetailRuntimeValue::Known(400),
                    1,
                    100,
                ),
            )),
            sub_d_stagger_seed: Some(FRESH_LEVEL1_ORDINARY_TYPE47_SUB_D_SEEDS[0]),
            sub_d_frame_owner: None,
            sub_d_runtime: None,
            body_right_q31: RetailRuntimeValue::Unresolved,
            body_forward_q31: RetailRuntimeValue::Unresolved,
            body_up_q31: RetailRuntimeValue::Unresolved,
            sub_h_runtime: None,
            terrain: None,
            global_elapsed_micros: 19_500,
            target_private: &mut target_private,
            tracked_target: RetailRuntimeValue::Known(Some(CommonMoverTrackedTargetSnapshot {
                state_flags: v2k_game::entity_collision_state::RetailStateWord::exact(4),
                position_raw: [1_000, 0, 0],
                velocity_raw: [0; 3],
            })),
            elapsed_micros: 19_500,
        },
        || 0,
    );
    assert_eq!(
        type47.unwrap_err(),
        Type47ChaseMoverBlock::SubDFirstQueryUnavailable
    );

    type47_metadata.sub_b_lateral_descriptor = RetailRuntimeValue::Known(Some(
        v2k_game::ordinary_type47_death_live::TYPE47_COMMON_DYING_SUB_B_DESCRIPTOR,
    ));
    type47_metadata.sub_c_lift_descriptor = RetailRuntimeValue::Known(Some(
        v2k_game::ordinary_type47_death_live::TYPE47_COMMON_DYING_SUB_C_DESCRIPTOR,
    ));
    type47_metadata.sub_d_steering_descriptor = RetailRuntimeValue::Known(Some(
        v2k_game::ordinary_type47_live::FRESH_LEVEL1_ORDINARY_TYPE47_SUB_D,
    ));
    type47_metadata.sub_h_external_frame_descriptor =
        RetailRuntimeValue::Known(Some(v2k_formats::collision::SubHExternalFrameDescriptor {
            completion_sound_id: None,
            records: v2k_game::ordinary_type47_death_live::TYPE47_COMMON_DYING_SUB_H_RECORDS
                .to_vec(),
        }));
    let terrain = empty_terrain();
    let mut owner = intro2_type47_first_query_owner_for_seed(0x07).expect("Intro2 seed");
    let mut target_private = WanderNearPrivateState::tracked_entity([0, 0, 0], 0x047F_0001);
    let intro2_bind = evaluate_intro2_type47_common_mover(
        Intro2Type47CommonMoverRequest {
            entity_id: 0x042F_000B,
            entity_type: 47,
            metadata: Some(&type47_metadata),
            position_raw: [0xB700u16 as i16, 0, 0x7F00u16 as i16],
            velocity_raw: [10, 0, 0],
            heading_raw: 0,
            roll_raw: 0,
            sub_a_runtime: RetailRuntimeValue::Known(Some(
                v2k_game::common_mover::SubAPropulsionRuntime::from_retail_words(
                    RetailRuntimeValue::Known(400),
                    1,
                    100,
                ),
            )),
            sub_d_stagger_seed: Some(FRESH_LEVEL1_ORDINARY_TYPE47_SUB_D_SEEDS[0]),
            sub_d_frame_owner: Some(&mut owner),
            sub_d_runtime: None,
            body_right_q31: RetailRuntimeValue::Unresolved,
            body_forward_q31: RetailRuntimeValue::Unresolved,
            body_up_q31: RetailRuntimeValue::Unresolved,
            sub_h_runtime: None,
            terrain: Some(&terrain),
            global_elapsed_micros: 19_500,
            target_private: &mut target_private,
            tracked_target: RetailRuntimeValue::Known(Some(CommonMoverTrackedTargetSnapshot {
                state_flags: v2k_game::entity_collision_state::RetailStateWord::exact(4),
                position_raw: [1_000, 0, 0],
                velocity_raw: [0; 3],
            })),
            elapsed_micros: 19_500,
        },
        || 0,
    );
    assert_eq!(
        intro2_bind,
        Err(Intro2Type47CommonMoverBlock::SubDFirstQueryUnavailable)
    );
    assert_eq!(
        owner.classifier_cache().origin(),
        RetailRuntimeValue::Unresolved
    );
}

#[test]
fn type13_inactive_target_returns_before_later_mover_evidence() {
    let mut metadata = EntityTypeRuntimeMetadata::default();
    metadata.common_mover_topology =
        RetailRuntimeValue::Known(v2k_game::search_attack_live::TYPE13_SEARCH_ATTACK_TOPOLOGY);
    metadata.common_mover_gkl_payloads = RetailRuntimeValue::Unresolved;
    metadata.sub_d_steering_descriptor = RetailRuntimeValue::Known(Some(
        v2k_game::search_attack_live::TYPE13_SEARCH_ATTACK_SUB_D,
    ));

    let terrain = empty_terrain();
    let outcome = evaluate_type13_common_mover(
        GklCommonMoverRequest {
            entity_id: 0x04fc_0001,
            entity_type: 13,
            metadata: Some(&metadata),
            dispatch_mode: CommonMoverDispatchMode::Restricted,
            position_raw: [0; 3],
            velocity_raw: [10, 0, 0],
            heading_raw: 0,
            pitch_raw: 0,
            roll_raw: 0,
            body_basis: RetailRuntimeValue::Unresolved,
            runtime: GklCommonMoverRuntime::from_intro2_constructor_capture(),
            sub_g_runtime: SubG06070RuntimeState::pending(),
            target_private: WanderNearPrivateState::tracked_entity([0; 3], 0x047F_0001),
            tracked_target: RetailRuntimeValue::Known(Some(CommonMoverTrackedTargetSnapshot {
                state_flags: v2k_game::entity_collision_state::RetailStateWord::exact(0),
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
        || panic!("inactive-target return must not draw RNG"),
    )
    .expect("inactive target returns successfully before later mover evidence");
    assert_eq!(
        outcome.result,
        ChaseTargetCommonMoverReturn::Zero,
        "retail rejects the target before reading body-basis or G/K/L evidence"
    );
}

#[test]
fn classifier_free_sub_d_keeps_the_pre_branch_stagger_maintenance() {
    let basis = Type9BodyBasis::from_angle_words(0, 0, 0);
    let mut owner = Type9SubDFrameOwner::from_retail_state([1, 2, 3, 4, 5, 6, 7, 8], [9, 10], 7);
    owner
        .evidence_for_classifier_free_frame(
            FLYER_SUB_D,
            [0; 3],
            [0, 0, 1_000],
            basis.lateral,
            basis.forward,
        )
        .expect("zero classifier flags");
    assert_eq!(owner.classifier_cache().stagger_counter(), 8);
    assert_eq!(
        owner.classifier_cache().origin(),
        RetailRuntimeValue::Known([9, 10])
    );
    assert_eq!(owner.classifier_cache().rows(), [1, 0, 3, 4, 5, 6, 7, 8]);
}

#[test]
fn intro2_type47_seed_mints_pending_reset_without_level1_seeds() {
    let terrain = empty_terrain();
    let mut owner = intro2_type47_first_query_owner_for_seed(0x07).expect("Intro2 Type-47 seed");
    assert_eq!(
        owner.apply_first_query(&terrain, [(0xbcu16 << 8) as i16, (0x7cu16 << 8) as i16]),
        RetailRuntimeValue::Known(0)
    );
    assert_eq!(
        owner.classifier_cache().origin(),
        RetailRuntimeValue::Known([0xbc, 0x7c])
    );
    assert_eq!(owner.classifier_cache().rows(), [1, 0, 0, 0, 0, 0, 0, 0]);
}

fn intro2_type47_metadata() -> EntityTypeRuntimeMetadata {
    let mut metadata = EntityTypeRuntimeMetadata::default();
    metadata.common_mover_topology = RetailRuntimeValue::Known(
        v2k_game::ordinary_type47_live::FRESH_LEVEL1_ORDINARY_TYPE47_COMPONENT_TOPOLOGY,
    );
    metadata.sub_a_propulsion_descriptor = RetailRuntimeValue::Known(Some(
        v2k_game::ordinary_type47_death_live::TYPE47_COMMON_DYING_SUB_A_DESCRIPTOR,
    ));
    metadata.sub_b_lateral_descriptor = RetailRuntimeValue::Known(Some(
        v2k_game::ordinary_type47_death_live::TYPE47_COMMON_DYING_SUB_B_DESCRIPTOR,
    ));
    metadata.sub_c_lift_descriptor = RetailRuntimeValue::Known(Some(
        v2k_game::ordinary_type47_death_live::TYPE47_COMMON_DYING_SUB_C_DESCRIPTOR,
    ));
    metadata.sub_d_steering_descriptor = RetailRuntimeValue::Known(Some(
        v2k_game::ordinary_type47_live::FRESH_LEVEL1_ORDINARY_TYPE47_SUB_D,
    ));
    metadata.sub_h_external_frame_descriptor =
        RetailRuntimeValue::Known(Some(v2k_formats::collision::SubHExternalFrameDescriptor {
            completion_sound_id: None,
            records: v2k_game::ordinary_type47_death_live::TYPE47_COMMON_DYING_SUB_H_RECORDS
                .to_vec(),
        }));
    metadata
}

#[test]
fn intro2_type47_rejects_type26_topology_and_foreign_sub_d() {
    use v2k_game::intro2_type47_common_mover::Intro2Type47CommonMoverTopology;

    let metadata = intro2_type47_metadata();
    assert!(Intro2Type47CommonMoverTopology::from_metadata(47, &metadata).is_ok());
    assert!(Intro2Type47CommonMoverTopology::from_metadata(26, &metadata).is_err());

    let mut type26_shape = intro2_type47_metadata();
    type26_shape.common_mover_topology =
        RetailRuntimeValue::Known(INTRO2_TYPE26_COMMON_MOVER_TOPOLOGY);
    assert!(Intro2Type47CommonMoverTopology::from_metadata(47, &type26_shape).is_err());

    let mut type9_sub_d = intro2_type47_metadata();
    type9_sub_d.sub_d_steering_descriptor = RetailRuntimeValue::Known(Some(INTRO2_TYPE26_SUB_D));
    let mut target_private = WanderNearPrivateState::tracked_entity([0, 0, 0], 0x047F_0001);
    let result = evaluate_intro2_type47_common_mover(
        Intro2Type47CommonMoverRequest {
            entity_id: INTRO2_TYPE47_TTD_HANDLES[1],
            entity_type: 47,
            metadata: Some(&type9_sub_d),
            position_raw: INTRO2_TYPE47_TTD_POSITIONS_RAW[1],
            velocity_raw: [10, 0, 0],
            heading_raw: 0,
            roll_raw: 0,
            sub_a_runtime: RetailRuntimeValue::Known(None),
            sub_d_stagger_seed: Some(0x07),
            sub_d_frame_owner: None,
            sub_d_runtime: None,
            body_right_q31: RetailRuntimeValue::Unresolved,
            body_forward_q31: RetailRuntimeValue::Unresolved,
            body_up_q31: RetailRuntimeValue::Unresolved,
            sub_h_runtime: None,
            terrain: None,
            global_elapsed_micros: 19_500,
            target_private: &mut target_private,
            tracked_target: RetailRuntimeValue::Known(Some(CommonMoverTrackedTargetSnapshot {
                state_flags: v2k_game::entity_collision_state::RetailStateWord::exact(4),
                position_raw: [1_000, 0, 0],
                velocity_raw: [0; 3],
            })),
            elapsed_micros: 19_500,
        },
        || panic!("type-26 Sub-D must not start Intro2 Type-47 FUN_00401430"),
    );
    assert_eq!(result, Err(Intro2Type47CommonMoverBlock::UnexpectedSubD));
}

#[test]
fn intro2_type47_first_query_applies_0x13_then_sub_h_and_cab_tail() {
    let metadata = intro2_type47_metadata();
    let terrain = empty_terrain();
    let mut owner = intro2_type47_first_query_owner_for_seed(0x07).expect("Intro2 Type-47 seed");
    let mut runtime = Type9SubDRuntime::from_constructor();
    let mut sub_h = v2k_game::sub_h_external_frame::SubHRuntimeState::new(
        v2k_game::ordinary_type47_live::FRESH_LEVEL1_ORDINARY_TYPE47_SUB_H_RECORD_COUNT,
    )
    .expect("six-record Type-47 Sub-H");
    let mut target_private = WanderNearPrivateState::tracked_entity([0, 0, 0], 0x047F_0001);
    let result = evaluate_intro2_type47_common_mover(
        Intro2Type47CommonMoverRequest {
            entity_id: INTRO2_TYPE47_TTD_HANDLES[1],
            entity_type: 47,
            metadata: Some(&metadata),
            position_raw: INTRO2_TYPE47_TTD_POSITIONS_RAW[1],
            velocity_raw: [10, 0, 0],
            heading_raw: 0,
            roll_raw: 0,
            sub_a_runtime: RetailRuntimeValue::Known(Some(
                v2k_game::common_mover::SubAPropulsionRuntime::from_retail_words(
                    RetailRuntimeValue::Known(400),
                    1,
                    100,
                ),
            )),
            sub_d_stagger_seed: Some(0x07),
            sub_d_frame_owner: Some(&mut owner),
            sub_d_runtime: Some(&mut runtime),
            body_right_q31: RetailRuntimeValue::Known([i32::MAX, 0, 0]),
            body_forward_q31: RetailRuntimeValue::Known([0, 0, i32::MAX]),
            body_up_q31: RetailRuntimeValue::Known([0, i32::MAX, 0]),
            sub_h_runtime: Some(&mut sub_h),
            terrain: Some(&terrain),
            global_elapsed_micros: 19_500,
            target_private: &mut target_private,
            tracked_target: RetailRuntimeValue::Known(Some(CommonMoverTrackedTargetSnapshot {
                state_flags: v2k_game::entity_collision_state::RetailStateWord::exact(4),
                position_raw: [1_000, 0, 0],
                velocity_raw: [0; 3],
            })),
            elapsed_micros: 19_500,
        },
        || 0,
    );
    assert_eq!(
        result.expect("Intro2 Type-47 first query").result,
        ChaseTargetCommonMoverReturn::NonZero
    );
    assert!(matches!(
        owner.classifier_cache().origin(),
        RetailRuntimeValue::Known(_)
    ));
    assert!(
        owner.classifier_cache().rows().iter().any(|row| *row != 0),
        "0x13 classification must fill at least one cache nibble"
    );
    assert_ne!(runtime.yaw_rate_raw, 0);
    assert_eq!(
        v2k_game::ordinary_type47_live::FRESH_LEVEL1_ORDINARY_TYPE47_SUB_D,
        TYPE47_SUB_D
    );
}

#[test]
fn v200003_level1_type47_first_query_applies_0x13_then_sub_h_and_cab_tail() {
    let metadata = intro2_type47_metadata();
    let terrain = empty_terrain();
    let mut owner = level1_type47_first_query_owner_for_seed(0x2D).expect("V200003 spawn-13 seed");
    let mut runtime = Type9SubDRuntime::from_constructor();
    let mut sub_h = v2k_game::sub_h_external_frame::SubHRuntimeState::new(
        v2k_game::ordinary_type47_live::FRESH_LEVEL1_ORDINARY_TYPE47_SUB_H_RECORD_COUNT,
    )
    .expect("six-record Type-47 Sub-H");
    let mut target_private = WanderNearPrivateState::tracked_entity([0, 0, 0], 0x047F_0001);
    let result = evaluate_type47_chase_common_mover(
        Type47ChaseMoverRequest {
            entity_id: 0x04AB_0001,
            entity_type: 47,
            metadata: Some(&metadata),
            position_raw: [0xBE00u16 as i16, 0, 0x7D00u16 as i16],
            velocity_raw: [10, 0, 0],
            heading_raw: 0,
            roll_raw: 0,
            sub_a_runtime: RetailRuntimeValue::Known(Some(
                v2k_game::common_mover::SubAPropulsionRuntime::from_retail_words(
                    RetailRuntimeValue::Known(400),
                    1,
                    100,
                ),
            )),
            sub_d_stagger_seed: Some(0x2D),
            sub_d_frame_owner: Some(&mut owner),
            sub_d_runtime: Some(&mut runtime),
            body_right_q31: RetailRuntimeValue::Known([i32::MAX, 0, 0]),
            body_forward_q31: RetailRuntimeValue::Known([0, 0, i32::MAX]),
            body_up_q31: RetailRuntimeValue::Known([0, i32::MAX, 0]),
            sub_h_runtime: Some(&mut sub_h),
            terrain: Some(&terrain),
            global_elapsed_micros: 19_500,
            target_private: &mut target_private,
            tracked_target: RetailRuntimeValue::Known(Some(CommonMoverTrackedTargetSnapshot {
                state_flags: v2k_game::entity_collision_state::RetailStateWord::exact(4),
                position_raw: [1_000, 0, 0],
                velocity_raw: [0; 3],
            })),
            elapsed_micros: 19_500,
        },
        || 0,
    )
    .expect("V200003 first query then Sub-H/C/A/B");
    assert_eq!(result.result, ChaseTargetCommonMoverReturn::NonZero);
    assert!(matches!(
        owner.classifier_cache().origin(),
        RetailRuntimeValue::Known(_)
    ));
    assert!(
        owner.classifier_cache().rows().iter().any(|row| *row != 0),
        "0x13 classification must fill at least one cache nibble"
    );
    assert_ne!(runtime.yaw_rate_raw, 0);
    assert!(level1_type47_first_query_owner_for_seed(0x3C).is_none());
    assert!(level1_type47_first_query_owner_for_seed(0x06).is_none());
}

fn spawn25_publication_metadata() -> EntityTypeRuntimeMetadata {
    EntityTypeRuntimeMetadata {
        model_slots: [INTRO2_DEFECATE_VIRUS_MODEL_ID as u16; 4],
        mass_raw: 400,
        capability_flags: 8,
        initial_health_raw: Some(5_000),
        terrain_contact_task_lifetime_ms: RetailRuntimeValue::Known(67),
        sub_a_propulsion_descriptor: RetailRuntimeValue::Known(Some(INTRO2_TYPE26_SUB_A)),
        sub_b_lateral_descriptor: RetailRuntimeValue::Known(Some(INTRO2_TYPE26_SUB_B)),
        sub_c_lift_descriptor: RetailRuntimeValue::Known(Some(INTRO2_TYPE26_SUB_C)),
        common_mover_topology: RetailRuntimeValue::Known(INTRO2_TYPE26_COMMON_MOVER_TOPOLOGY),
        sub_d_steering_descriptor: RetailRuntimeValue::Known(Some(INTRO2_TYPE26_SUB_D)),
        initializer: Some(EntityInitializerSpec {
            initializer_state_flags_raw: 0x0439,
            common_axis_descriptor: CommonAxisDescriptor {
                strict_axis_limit_raw: 0x0F00,
                raw_word_at_0x04: 0x0225,
            },
            behavior_choices: INTRO2_TYPE26_BEHAVIOR_CHOICES.to_vec().into_boxed_slice(),
            behavior_rule_ref: 1,
            alternate_behavior_class_ref: 12,
        }),
        ..EntityTypeRuntimeMetadata::default()
    }
}

fn spawn25_unpublished_entity() -> Entity {
    let mut entity = Entity::unresolved_port_entity(
        77,
        EntityKind::Unknown(INTRO2_DEFECATE_VIRUS_ENTITY_TYPE),
        INTRO2_DEFECATE_VIRUS_ENTITY_TYPE,
    );
    entity.authored_spawn_index = Some(INTRO2_DEFECATE_VIRUS_SPAWN_INDEX);
    entity.model_slots = [Some(INTRO2_DEFECATE_VIRUS_MODEL_ID); 4];
    entity.model_index = Some(INTRO2_DEFECATE_VIRUS_MODEL_ID);
    entity.position = [194.0, 0.0, 14.0];
    entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(
        SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Unresolved, 1, 100),
    ));
    entity
}

fn identity_body_basis() -> Type9BodyBasis {
    Type9BodyBasis {
        lateral: [i32::MAX, 0, 0],
        up: [0, i32::MAX, 0],
        forward: [0, 0, i32::MAX],
    }
}

#[test]
fn spawn25_primary_visit_applies_type26_cab_pose() {
    let metadata = spawn25_publication_metadata();
    let terrain = empty_terrain();
    let mut entity = spawn25_unpublished_entity();
    publish_intro2_type26_defecate_virus(&mut entity, &metadata).expect("exact capture fixture");
    retain_intro2_type26_physical_body_basis(&mut entity, identity_body_basis());
    entity.set_motion_raw(entity.position_raw(), [10, 0, 0]);
    let heading_before = entity.heading_raw();

    let visit = tick_intro2_type26_defecate_virus_primary(
        &mut entity,
        Some(&metadata),
        Some(&terrain),
        None,
        19_500,
        &mut || 0,
    )
    .expect("spawn-25 Primary visit");

    assert_eq!(visit.result, Intro2Type26PrimaryVisitResult::Continue);
    assert_eq!(
        visit.owner_transition,
        Intro2Type26OwnerTransition::NotRequested
    );
    assert_eq!(visit.lifetime_ms, 19);
    assert!(matches!(
        entity
            .intro2_type26_sub_d_frame_owner
            .as_ref()
            .map(|owner| owner.classifier_cache().origin()),
        Some(RetailRuntimeValue::Known(_))
    ));
    assert_ne!(
        entity
            .intro2_type26_sub_d_runtime
            .expect("spawn-25 Sub-D runtime")
            .yaw_rate_raw,
        0
    );
    assert_ne!(entity.heading_raw(), heading_before);
}

#[test]
fn unresolved_body_basis_and_type13_fail_closed() {
    let metadata = spawn25_publication_metadata();
    let terrain = empty_terrain();
    let mut type13 = spawn25_unpublished_entity();
    type13.entity_type = 13;
    let mut entity = spawn25_unpublished_entity();
    publish_intro2_type26_defecate_virus(&mut entity, &metadata).expect("exact capture fixture");
    let heading_before = entity.heading_raw();
    let position_before = entity.position_raw();

    let visit = tick_intro2_type26_defecate_virus_primary(
        &mut entity,
        Some(&metadata),
        Some(&terrain),
        None,
        19_500,
        &mut || 0,
    )
    .expect("visit opens before the mover fail-closes");
    assert_eq!(
        visit.result,
        Intro2Type26PrimaryVisitResult::CommonMoverBlocked(
            Type26CommonMoverBlock::BodyBasisUnavailable
        )
    );
    assert_eq!(entity.heading_raw(), heading_before);
    assert_eq!(entity.position_raw(), position_before);
    assert_eq!(
        entity.intro2_type26_sub_d_runtime,
        Some(Type9SubDRuntime::from_constructor())
    );

    assert_eq!(
        tick_intro2_type26_defecate_virus_primary(
            &mut type13,
            Some(&metadata),
            Some(&terrain),
            None,
            19_500,
            &mut || 0,
        ),
        Err(Intro2Type26PrimaryVisitBlock::EntityIdentityMismatch)
    );
}

#[test]
fn spawn25_production_owner_ticks_primary_and_rejects_type13() {
    let metadata = spawn25_publication_metadata();
    let terrain = empty_terrain();
    let mut entity = spawn25_unpublished_entity();
    assert!(Intro2Type26SchedulerOwner::adopt_published(&entity).is_err());
    publish_intro2_type26_defecate_virus(&mut entity, &metadata).expect("exact capture fixture");
    let owner = Intro2Type26SchedulerOwner::adopt_published(&entity).expect("published spawn 25");
    retain_intro2_type26_physical_body_basis(&mut entity, identity_body_basis());
    entity.set_motion_raw(entity.position_raw(), [10, 0, 0]);

    let tick = tick_intro2_type26_scheduler_owner_on_entity(
        &mut entity,
        Some(&metadata),
        Some(&terrain),
        None,
        [None; 4],
        19_500,
        &mut || 0,
        owner,
    );
    assert!(tick.retained_owner.is_some());
    match tick.outcome {
        Intro2Type26SchedulerProductionOutcome::Primary {
            visit, tertiary, ..
        } => {
            assert_eq!(visit.result, Intro2Type26PrimaryVisitResult::Continue);
            assert_eq!(
                tertiary.result,
                Intro2Type26TertiaryVisitResult::StateFlagsUnavailable
            );
        }
        other => panic!("expected Primary continue, got {other:?}"),
    }
    assert_ne!(
        entity
            .intro2_type26_sub_d_runtime
            .expect("spawn-25 Sub-D runtime")
            .yaw_rate_raw,
        0
    );

    let mut type13 = spawn25_unpublished_entity();
    type13.entity_type = 13;
    assert!(Intro2Type26SchedulerOwner::adopt_published(&type13).is_err());
}

#[test]
fn spawn25_tertiary_coarse_visit_and_detailed_basis_fail_closed() {
    let metadata = spawn25_publication_metadata();
    let mut entity = spawn25_unpublished_entity();
    publish_intro2_type26_defecate_virus(&mut entity, &metadata).expect("exact capture fixture");
    entity.collision.state_flags_at_0x08 = RetailStateWord::exact(0);

    let visit =
        tick_intro2_type26_defecate_virus_tertiary(&mut entity, [None; 4], 19_500, &mut || 0)
            .expect("spawn-25 Tertiary visit");
    assert_eq!(visit.prefix.elapsed_ms, 19);
    assert_eq!(
        visit.owner_transition,
        Intro2Type26OwnerTransition::NotRequested
    );
    assert_eq!(
        visit.result,
        Intro2Type26TertiaryVisitResult::Planned(DefecateVirusCallbackPlan::CoarseTerrainMutation(
            InfectionCellWrite {
                cell: [0xC1, 0x0D],
                infected: true,
            }
        ))
    );

    entity.collision.state_flags_at_0x08 = RetailStateWord::exact(0x0200_0000);
    let detailed =
        tick_intro2_type26_defecate_virus_tertiary(&mut entity, [Some(512); 4], 19_500, &mut || 0)
            .expect("detailed visit opens before basis fail-closes");
    assert_eq!(
        detailed.result,
        Intro2Type26TertiaryVisitResult::BodyBasisUnavailable
    );

    let mut type13 = spawn25_unpublished_entity();
    type13.entity_type = 13;
    assert_eq!(
        tick_intro2_type26_defecate_virus_tertiary(&mut type13, [None; 4], 19_500, &mut || 0),
        Err(Intro2Type26TertiaryVisitBlock::EntityIdentityMismatch)
    );
}

#[test]
fn spawn25_detailed_visit_uses_header_0x08_extent() {
    let metadata = spawn25_publication_metadata();
    let mut entity = spawn25_unpublished_entity();
    publish_intro2_type26_defecate_virus(&mut entity, &metadata).expect("exact capture fixture");
    retain_intro2_type26_physical_body_basis(
        &mut entity,
        Type9BodyBasis::from_angle_words(0x4000, 0, 0),
    );
    entity.collision.state_flags_at_0x08 = RetailStateWord::exact(0x0200_0000);

    assert_eq!(
        intro2_type26_model_extent_raw_by_state(&entity, |_| Some(512)),
        [Some(512); 4]
    );
    assert_eq!(
        intro2_type26_model_extent_raw_by_state(&entity, |_| None),
        [None; 4]
    );

    let missing =
        tick_intro2_type26_defecate_virus_tertiary(&mut entity, [None; 4], 19_500, &mut || 0)
            .expect("gate draw then fail-closed extent");
    assert_eq!(
        missing.result,
        Intro2Type26TertiaryVisitResult::Planned(
            DefecateVirusCallbackPlan::DetailedModelExtentUnavailable {
                model_slot_index: 0
            }
        )
    );

    let visit =
        tick_intro2_type26_defecate_virus_tertiary(&mut entity, [Some(512); 4], 19_500, &mut || 0)
            .expect("header +0x08 Detailed visit");
    let Intro2Type26TertiaryVisitResult::Planned(DefecateVirusCallbackPlan::DetailedParticle(
        emission,
    )) = visit.result
    else {
        panic!("expected DetailedParticle, got {:?}", visit.result);
    };
    assert_eq!(emission.scale_raw(), 0x0800);
    assert_eq!(emission.owner_entity_handle(), entity.id);
    let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
        panic!("constructor heading-0x4000 matrix must be retained")
    };
    let extent_offset = |component: i32| ((i64::from(component) * 512) >> 31) as i16;
    assert_eq!(
        emission.position_raw(),
        [
            entity.position_raw()[0].wrapping_sub(extent_offset(basis.forward[0])),
            entity.position_raw()[1]
                .wrapping_sub(extent_offset(basis.forward[1]))
                .wrapping_add(100),
            entity.position_raw()[2].wrapping_sub(extent_offset(basis.forward[2])),
        ]
    );
}

#[test]
fn spawn25_tertiary_timeout_is_callback_absent_unless_suppressed() {
    let metadata = spawn25_publication_metadata();
    let mut entity = spawn25_unpublished_entity();
    publish_intro2_type26_defecate_virus(&mut entity, &metadata).expect("exact capture fixture");
    entity.collision.state_flags_at_0x08 = RetailStateWord::exact(0);

    let timed_out =
        tick_intro2_type26_defecate_virus_tertiary(&mut entity, [None; 4], 68_000, &mut || 0)
            .expect("lifetime 67 expires after 68 ms");
    assert_eq!(timed_out.prefix.elapsed_ms, 68);
    assert_eq!(
        timed_out.owner_transition,
        Intro2Type26OwnerTransition::CallbackAbsent
    );
    assert!(matches!(
        entity.actor_task_state(v2k_game::actor_task_owner::ActorTaskSlot::Tertiary),
        Some(v2k_game::actor_task_dispatcher::ActorTaskRuntime::DefecateVirusTerrain(_))
    ));

    entity.collision.state_flags_at_0x08 = RetailStateWord::exact(0x0000_1000);
    let suppressed =
        tick_intro2_type26_defecate_virus_tertiary(&mut entity, [None; 4], 1_000, &mut || 0)
            .expect("elapsed already past lifetime");
    assert_eq!(
        suppressed.owner_transition,
        Intro2Type26OwnerTransition::SuppressedByEntityState
    );
}

#[test]
fn spawn25_primary_timeout_is_callback_absent_unless_suppressed() {
    let metadata = spawn25_publication_metadata();
    let terrain = empty_terrain();
    let mut entity = spawn25_unpublished_entity();
    publish_intro2_type26_defecate_virus(&mut entity, &metadata).expect("exact capture fixture");
    retain_intro2_type26_physical_body_basis(&mut entity, identity_body_basis());
    entity.set_motion_raw(entity.position_raw(), [10, 0, 0]);
    entity.collision.state_flags_at_0x08 = RetailStateWord::exact(0);

    let timed_out = tick_intro2_type26_defecate_virus_primary(
        &mut entity,
        Some(&metadata),
        Some(&terrain),
        None,
        2_001_000,
        &mut || 0,
    )
    .expect("wander lifetime 2000 expires after 2001 ms");
    assert_eq!(timed_out.result, Intro2Type26PrimaryVisitResult::Continue);
    assert_eq!(timed_out.lifetime_ms, 2_001);
    assert_eq!(
        timed_out.owner_transition,
        Intro2Type26OwnerTransition::CallbackAbsent
    );
    assert!(matches!(
        entity.actor_task_state(v2k_game::actor_task_owner::ActorTaskSlot::Primary),
        Some(v2k_game::actor_task_dispatcher::ActorTaskRuntime::DefecateVirusWander(_))
    ));

    entity.collision.state_flags_at_0x08 = RetailStateWord::exact(0x0000_1000);
    let suppressed = tick_intro2_type26_defecate_virus_primary(
        &mut entity,
        Some(&metadata),
        Some(&terrain),
        None,
        1_000,
        &mut || 0,
    )
    .expect("elapsed already past wander lifetime");
    assert_eq!(
        suppressed.owner_transition,
        Intro2Type26OwnerTransition::SuppressedByEntityState
    );
}
