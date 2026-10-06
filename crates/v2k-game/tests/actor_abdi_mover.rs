use v2k_formats::terrain::{TerrainCell, TerrainGrid, GRID_SIZE};
use v2k_game::actor_animation::ActorAnimationController;
use v2k_game::common_mover::actor_abdi::{
    advance_actor_abdi_common_mover, ActorAbdiEntityKind, ActorAbdiFrameOutcome,
    ActorAbdiFrameRequest, ActorAbdiFrameState, ActorAbdiPhase, ActorAbdiTopology,
    ActorAbdiTopologyError, ACTOR_ABDI_PHASE_ORDER, FIRST_WORLD_PEASANT_ENTITY_TYPE,
    FIRST_WORLD_SCIENTIST_ENTITY_TYPE, ORDINARY_TYPE7_ENTITY_TYPE,
};
use v2k_game::common_mover::sub_d::{
    Type9SubDFrameOwner, Type9SubDRuntime, ORDINARY_TYPE7_SUB_D, ORDINARY_TYPE90_SUB_D,
};
use v2k_game::common_mover::SubAPropulsionRuntime;
use v2k_game::entity_collision_state::{
    CommonMoverComponentTopology, EntityTypeRuntimeMetadata, RetailRuntimeValue,
};
use v2k_game::wander_near_location::WanderNearPrivateState;

fn high_resolution_first_world_overlay() -> v2k_formats::ovl::OvlFile {
    let path = v2k_test_support::retail_dir()
        .join("Overlay")
        .join("1X3XX.OVL");
    let bytes = std::fs::read(path).unwrap();
    v2k_formats::ovl::OvlFile::parse(&bytes).unwrap()
}

fn flat_terrain() -> TerrainGrid {
    TerrainGrid {
        header: [-1 << 8, 0, 0, 0, 0],
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

fn frame_state(topology: ActorAbdiTopology) -> ActorAbdiFrameState {
    ActorAbdiFrameState {
        sub_d_frame_owner: Type9SubDFrameOwner::from_retail_state([0; 8], [0, 0], 5),
        sub_d_runtime: Type9SubDRuntime::default(),
        sub_a_runtime: SubAPropulsionRuntime::from_retail_words(
            RetailRuntimeValue::Known(250),
            1,
            100,
        ),
        actor_animation: ActorAnimationController::from_descriptor(
            topology.actor_animation_descriptor(),
        )
        .unwrap(),
    }
}

fn run_fixture(
    topology: ActorAbdiTopology,
    terrain: &TerrainGrid,
) -> v2k_game::common_mover::actor_abdi::ActorAbdiFrameStep {
    let mut state = frame_state(topology);
    let outcome = advance_actor_abdi_common_mover(
        &mut state,
        ActorAbdiFrameRequest {
            topology,
            target_state: WanderNearPrivateState {
                target_position_raw: [0, 0, 100],
                tracked_entity_handle: 0,
                direction: 1,
                reversal_timer_ms: 0,
            },
            tracked_target: RetailRuntimeValue::Unresolved,
            terrain,
            position_raw: [0, 0, 0],
            pre_mover_right_q31: [0, 0, i32::MAX],
            pre_mover_forward_q31: [i32::MAX, 0, 0],
            heading_raw: 0x1000,
            velocity_raw: [0, 0, 0],
            elapsed_micros: 20_000,
            global_elapsed_micros: 20_000,
            scheduler_mode: 0,
        },
        || panic!("the deterministic D-I-A-B fixture must not consume RNG"),
    )
    .unwrap();
    let ActorAbdiFrameOutcome::Applied(step) = outcome else {
        panic!("the deterministic D-I-A-B fixture must apply");
    };
    step
}

#[v2k_test_support::retail_test]
fn retail_type8_and_type9_admit_only_the_exact_shared_route() {
    let overlay = high_resolution_first_world_overlay();
    let collision = v2k_formats::sections::parse_collision(&overlay).unwrap();

    // PRELOAD contributes cumulative types 0 and 1. First-world local records
    // 6 and 7 are therefore cumulative scientist type 8 and peasant type 9.
    let scientist_metadata = EntityTypeRuntimeMetadata::from_section12(&collision.entries[6]);
    let peasant_metadata = EntityTypeRuntimeMetadata::from_section12(&collision.entries[7]);
    let scientist =
        ActorAbdiTopology::from_metadata(FIRST_WORLD_SCIENTIST_ENTITY_TYPE, &scientist_metadata)
            .unwrap();
    let peasant =
        ActorAbdiTopology::from_metadata(FIRST_WORLD_PEASANT_ENTITY_TYPE, &peasant_metadata)
            .unwrap();

    let exact_components = CommonMoverComponentTopology {
        sub_a: true,
        sub_b: true,
        sub_d: true,
        sub_i: true,
        ..CommonMoverComponentTopology::default()
    };
    assert_eq!(scientist.entity_kind(), ActorAbdiEntityKind::Scientist);
    assert_eq!(peasant.entity_kind(), ActorAbdiEntityKind::Peasant);
    assert_eq!(scientist.component_topology(), exact_components);
    assert_eq!(peasant.component_topology(), exact_components);
    assert_eq!(
        scientist.phase_order(),
        [
            ActorAbdiPhase::SubD,
            ActorAbdiPhase::SubI,
            ActorAbdiPhase::SubA,
            ActorAbdiPhase::SubB,
        ]
    );
    assert_eq!(scientist.phase_order(), ACTOR_ABDI_PHASE_ORDER);
    assert_eq!(peasant.phase_order(), ACTOR_ABDI_PHASE_ORDER);
    assert_ne!(
        scientist.actor_animation_descriptor(),
        peasant.actor_animation_descriptor(),
        "the common route must retain each record's authored Sub-I sound words"
    );

    let mut unsupported = scientist_metadata.clone();
    let RetailRuntimeValue::Known(mut components) = unsupported.common_mover_topology else {
        unreachable!();
    };
    components.sub_h = true;
    unsupported.common_mover_topology = RetailRuntimeValue::Known(components);
    assert_eq!(
        ActorAbdiTopology::from_metadata(FIRST_WORLD_SCIENTIST_ENTITY_TYPE, &unsupported),
        Err(ActorAbdiTopologyError::UnsupportedComponentTopology)
    );

    let mut unresolved = scientist_metadata;
    unresolved.common_mover_topology = RetailRuntimeValue::Unresolved;
    assert_eq!(
        ActorAbdiTopology::from_metadata(FIRST_WORLD_SCIENTIST_ENTITY_TYPE, &unresolved),
        Err(ActorAbdiTopologyError::UnresolvedComponentTopology)
    );
    assert_eq!(
        ActorAbdiTopology::from_metadata(10, &peasant_metadata),
        Err(ActorAbdiTopologyError::UnsupportedEntityType { actual: 10 })
    );
}

fn type7_metadata() -> EntityTypeRuntimeMetadata {
    let overlay = high_resolution_first_world_overlay();
    let collision = v2k_formats::sections::parse_collision(&overlay).unwrap();
    // PRELOAD contributes two records: the diver's cumulative Type7 is local5.
    EntityTypeRuntimeMetadata::from_section12(&collision.entries[5])
}

#[v2k_test_support::retail_test]
fn retail_type7_retains_its_classifier_free_descriptor_and_attention_cue() {
    let mut metadata = type7_metadata();
    let topology = ActorAbdiTopology::from_metadata(ORDINARY_TYPE7_ENTITY_TYPE, &metadata).unwrap();
    assert_eq!(topology.entity_kind(), ActorAbdiEntityKind::Type7Worker);
    assert_eq!(
        topology.entity_kind().entity_type(),
        ORDINARY_TYPE7_ENTITY_TYPE
    );
    assert_eq!(topology.sub_d_descriptor(), ORDINARY_TYPE7_SUB_D);
    assert_eq!(topology.phase_order(), ACTOR_ABDI_PHASE_ORDER);
    assert_eq!(
        topology
            .actor_animation_descriptor()
            .attention_stop_sound_id,
        106
    );
    assert_eq!(
        topology.component_topology(),
        CommonMoverComponentTopology {
            sub_a: true,
            sub_b: true,
            sub_d: true,
            sub_i: true,
            ..CommonMoverComponentTopology::default()
        }
    );
    assert_eq!(
        metadata.sub_a_propulsion_descriptor,
        RetailRuntimeValue::Known(Some(v2k_formats::collision::SubAPropulsionDescriptor {
            acceleration_raw: 1500,
            overspeed_correction_raw: -3000,
            target_speed_base_raw: 250,
        }))
    );
    metadata.sub_d_steering_descriptor = RetailRuntimeValue::Known(Some(ORDINARY_TYPE90_SUB_D));
    assert_eq!(
        ActorAbdiTopology::from_metadata(ORDINARY_TYPE7_ENTITY_TYPE, &metadata),
        Err(ActorAbdiTopologyError::UnsupportedSubDDescriptor),
        "matching divisor32 does not authorize the person terrain classifier"
    );
}

#[v2k_test_support::retail_test]
fn retail_type7_abdi_frame_never_requires_or_resolves_a_classifier_origin() {
    let topology =
        ActorAbdiTopology::from_metadata(ORDINARY_TYPE7_ENTITY_TYPE, &type7_metadata()).unwrap();
    let mut state = frame_state(topology);
    // Deliberately unresolved allocator residue is safe only because Type7
    // never consumes the classifier origin. Frame cadence still wraps.
    state.sub_d_frame_owner = Type9SubDFrameOwner::pending_constructor_origin(0xfe);
    assert!(!state.sub_d_frame_owner.classifier_cache().can_classify());
    let terrain = flat_terrain();
    for expected_stagger in [0xff, 0] {
        let outcome = advance_actor_abdi_common_mover(
            &mut state,
            ActorAbdiFrameRequest {
                topology,
                target_state: WanderNearPrivateState {
                    target_position_raw: [100, 0, 0],
                    tracked_entity_handle: 0,
                    direction: 1,
                    reversal_timer_ms: 0,
                },
                tracked_target: RetailRuntimeValue::Unresolved,
                terrain: &terrain,
                position_raw: [0; 3],
                pre_mover_right_q31: [i32::MAX, 0, 0],
                pre_mover_forward_q31: [0, 0, i32::MAX],
                heading_raw: 0,
                velocity_raw: [0; 3],
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                scheduler_mode: 0,
            },
            || panic!("the flags0 D/I/A/B frame consumes no RNG"),
        )
        .unwrap();
        let ActorAbdiFrameOutcome::Applied(step) = outcome else {
            panic!("the diver's untracked target must execute its full mover");
        };
        assert!(step.yaw_step_raw > 0);
        assert!(step.propulsion_applied);
        let cache = state.sub_d_frame_owner.classifier_cache();
        assert_eq!(cache.stagger_counter(), expected_stagger);
        assert_eq!(cache.origin(), RetailRuntimeValue::Unresolved);
        assert_eq!(cache.rows(), [0; 8]);
        assert!(!cache.can_classify());
    }
}

#[v2k_test_support::retail_test]
fn retail_type8_and_type9_produce_the_same_exact_d_i_a_b_fixture_result() {
    let overlay = high_resolution_first_world_overlay();
    let collision = v2k_formats::sections::parse_collision(&overlay).unwrap();
    let scientist_metadata = EntityTypeRuntimeMetadata::from_section12(&collision.entries[6]);
    let peasant_metadata = EntityTypeRuntimeMetadata::from_section12(&collision.entries[7]);
    let scientist =
        ActorAbdiTopology::from_metadata(FIRST_WORLD_SCIENTIST_ENTITY_TYPE, &scientist_metadata)
            .unwrap();
    let peasant =
        ActorAbdiTopology::from_metadata(FIRST_WORLD_PEASANT_ENTITY_TYPE, &peasant_metadata)
            .unwrap();

    let terrain = flat_terrain();
    let scientist_step = run_fixture(scientist, &terrain);
    let peasant_step = run_fixture(peasant, &terrain);
    assert_eq!(scientist_step, peasant_step);
    assert_eq!(
        scientist_step.target_state,
        WanderNearPrivateState {
            target_position_raw: [0, 0, 100],
            tracked_entity_handle: 0,
            direction: 1,
            reversal_timer_ms: 0,
        }
    );
    assert_eq!(scientist_step.heading_raw, 3_597);
    assert_eq!(scientist_step.velocity_raw, [27, 0, 0]);
    assert_eq!(scientist_step.yaw_step_raw, 499);
    assert_eq!(scientist_step.actor_animation_selector, 25);
    assert!(scientist_step.propulsion_applied);
}
