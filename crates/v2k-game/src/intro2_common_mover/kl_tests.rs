use super::*;
use crate::common_mover::{
    sub_c::{apply_sub_c_lift_raw, sub_c_surface_sample_point_raw, SubCSurfaceSample},
    sub_d::{construct_native_sub_d, SubDAllocationCounter},
    type9_attitude::Type9BodyBasis,
    SubAPropulsionRuntime,
};
use crate::entity::EntityKind;
use crate::session::GameSession;
use crate::sub_h_external_frame::SubHRuntimeState;
use v2k_formats::{
    models::AnimVars,
    terrain::{TerrainCell, GRID_SIZE},
};

fn canonical_metadata() -> Option<EntityTypeRuntimeMetadata> {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(17, 1).unwrap();
    Some(EntityTypeRuntimeMetadata::from_section12(
        session.cache.global_entity_type(30).unwrap(),
    ))
}

// This fixture exercises the shared component phase, not native actor/type
// admission. A future Type30 owner must issue its real lease and task graph.
struct Fixture {
    metadata: EntityTypeRuntimeMetadata,
    entity: Entity,
    terrain: TerrainGrid,
    basis: Type9BodyBasis,
    sub_d_runtime: Type9SubDRuntime,
    sub_d_owner: Type9SubDFrameOwner,
    kl: Intro2KlComponents,
}

impl Fixture {
    fn new(metadata: EntityTypeRuntimeMetadata) -> Self {
        let RetailRuntimeValue::Known(Some(a)) = metadata.sub_a_propulsion_descriptor else {
            panic!("canonical Type30 owns A");
        };
        let RetailRuntimeValue::Known(Some(d)) = metadata.sub_d_steering_descriptor else {
            panic!("canonical Type30 owns D");
        };
        let RetailRuntimeValue::Known(Some(h)) = &metadata.sub_h_external_frame_descriptor else {
            panic!("canonical Type30 owns H");
        };
        let kl = Intro2KlComponents::from_native_constructor(&metadata).unwrap();
        let mut counter = SubDAllocationCounter::from_next_seed(0);
        let sub_d = construct_native_sub_d(&mut counter, d);
        let angles = [0x1800, 0x1000, -0x1400];
        let basis = Type9BodyBasis::from_angle_words(angles[0], angles[1], angles[2]);
        let mut entity = Entity::unresolved_port_entity(7, EntityKind::Enemy, 30);
        entity.set_rotation_heading_pitch_roll_raw(angles);
        entity.physical_body_basis_q31 = RetailRuntimeValue::Known(basis);
        entity.set_motion_raw([10_000, 120, 10_000], [250, -125, 90]);
        entity.sub_a_propulsion_runtime =
            RetailRuntimeValue::Known(Some(SubAPropulsionRuntime::from_20450_constructor(a, 0)));
        entity.sub_h_external_frame_runtime =
            RetailRuntimeValue::Known(Some(SubHRuntimeState::new(h.records.len()).unwrap()));
        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(7);
        Self {
            metadata,
            entity,
            terrain: TerrainGrid {
                header: [0; 5],
                cells: vec![
                    TerrainCell {
                        height: 0,
                        attribute: 0,
                        terrain_type: 0,
                    };
                    GRID_SIZE * GRID_SIZE
                ],
            },
            basis,
            sub_d_runtime: sub_d.runtime,
            sub_d_owner: sub_d.frame_owner,
            kl,
        }
    }

    fn run(
        &mut self,
        mode: CommonMoverDispatchMode,
        target: Option<&mut WanderNearPrivateState>,
    ) -> Result<bool, Intro2CommonMoverBlock> {
        run_intro2_common_mover(
            &mut self.entity,
            Intro2CommonMoverFrame {
                metadata: &self.metadata,
                topology: ABCDEHKL_TOPOLOGY,
                terrain: &self.terrain,
                wave_tick_50hz: None,
                dispatch_mode: mode,
                elapsed_micros: 20_000,
                global_elapsed_micros: 40_000,
                sub_d_runtime: &mut self.sub_d_runtime,
                sub_d_owner: &mut self.sub_d_owner,
                kl_components: Some(&mut self.kl),
                target,
                tracked_target: RetailRuntimeValue::Known(None),
            },
            &mut || panic!("these phase inputs cannot consume RNG"),
        )
    }

    fn prime_null_target_components(&mut self) {
        self.basis = Type9BodyBasis::from_angle_words(0, 0, 0);
        self.entity.set_rotation_heading_pitch_roll_raw([0; 3]);
        self.entity.physical_body_basis_q31 = RetailRuntimeValue::Known(self.basis);
        self.entity
            .set_motion_raw([10_000, -20, 10_000], [-9, -125, 13]);
        self.kl.commit_target([10_001, 50, 10_001]);
        self.kl.commit_sub_d_writes(64, 128);
    }
}

#[v2k_test_support::retail_test]
fn type30_kl_constructor_owns_four_zeroed_words_and_real_one_based_bindings() {
    let Some(metadata) = canonical_metadata() else {
        return;
    };
    let mut kl = Intro2KlComponents::from_native_constructor(&metadata).unwrap();
    assert_eq!(kl.model_variables_raw(), &[0; 4]);
    assert_eq!(kl.sub_k_smoothed_raw(), 0);
    assert_eq!(kl.sub_l_target_raw(), [0; 3]);
    assert_eq!(kl.sub_l_exact_raw(), 0);
    kl.commit_sub_d_writes(64, 128);
    kl.advance_sub_k(80);
    // K+0 points at selector3; K+4 points at selector4.
    assert_eq!(kl.model_variables_raw(), &[0, 0, -256, -160]);
    kl.advance_sub_l([0; 3], Type9BodyBasis::from_angle_words(0, 0, 0));
    // L+0 points at selector2; L+4 points at selector1, independently of K.
    assert_eq!(kl.model_variables_raw(), &[0, 768, -256, -160]);
    let mut vars = AnimVars::default();
    vars.dynamic[0] = 123_456;
    vars.dynamic[5] = 888;
    kl.publish_model_variables(&mut vars);
    assert_eq!(&vars.dynamic[..6], &[123_456, 0, 768, -256, -160, 888]);

    let mut foreign = metadata.clone();
    foreign.model_variable_count_raw = RetailRuntimeValue::Known(3);
    assert_eq!(
        Intro2KlComponents::from_native_constructor(&foreign),
        Err(Intro2KlConstructionError::VariableCount)
    );
    foreign = metadata.clone();
    let RetailRuntimeValue::Known(mut payloads) = foreign.common_mover_gkl_payloads else {
        panic!();
    };
    payloads.sub_k = Some([11, 10]);
    foreign.common_mover_gkl_payloads = RetailRuntimeValue::Known(payloads);
    assert_eq!(
        Intro2KlComponents::from_native_constructor(&foreign),
        Err(Intro2KlConstructionError::ComponentDescriptors)
    );
    foreign = metadata;
    foreign.common_mover_topology = RetailRuntimeValue::Known(ABCDEH_TOPOLOGY);
    assert_eq!(
        Intro2KlComponents::from_native_constructor(&foreign),
        Err(Intro2KlConstructionError::ComponentTopology)
    );
}

/// Model 1131 (Type38/Type129) authors K [4,3] and L [1,2]: 09A80 binds the
/// same callbacks to the other word of each pair.
#[v2k_test_support::retail_test]
fn model1131_kl_binds_each_pair_the_other_way_round() {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(42, 1).unwrap();
    let type30 =
        EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(30).unwrap());
    for entity_type in [38, 129] {
        let metadata = EntityTypeRuntimeMetadata::from_section12(
            session.cache.global_entity_type(entity_type).unwrap(),
        );
        let mut kl = Intro2KlComponents::from_native_constructor(&metadata).unwrap();
        assert!(kl.authenticates(&metadata));
        // A bank bound for model 1131 is not Type30's.
        assert!(!kl.authenticates(&type30));
        kl.commit_sub_d_writes(64, 128);
        kl.advance_sub_k(80);
        // K+0 points at selector4; K+4 points at selector3.
        assert_eq!(kl.model_variables_raw(), &[0, 0, -160, -256]);
        kl.advance_sub_l([0; 3], Type9BodyBasis::from_angle_words(0, 0, 0));
        // L+0 points at selector1; L+4 points at selector2.
        assert_eq!(kl.model_variables_raw(), &[768, 0, -160, -256]);
    }
}

#[v2k_test_support::retail_test]
fn type30_kl_restricted_keeps_sub_d_writes_but_skips_h_k_l_callbacks() {
    let Some(metadata) = canonical_metadata() else {
        return;
    };
    let mut normal = Fixture::new(metadata.clone());
    let mut restricted = Fixture::new(metadata);
    normal.sub_d_runtime.yaw_rate_raw = 8192;
    restricted.sub_d_runtime.yaw_rate_raw = 8192;
    let mut target = WanderNearPrivateState::tracked_entity([12_048, 120, 11_024], 0);
    let mut restricted_target = target;
    assert!(normal
        .run(CommonMoverDispatchMode::Normal, Some(&mut target))
        .unwrap());
    assert!(restricted
        .run(
            CommonMoverDispatchMode::Restricted,
            Some(&mut restricted_target)
        )
        .unwrap());
    let yaw = i32::from(normal.sub_d_runtime.last_yaw_step_raw);
    assert_ne!(yaw, 0);
    assert_eq!(normal.kl.sub_l_target_raw(), target.target_position_raw);
    assert_eq!(normal.kl.sub_k_smoothed_raw(), yaw >> 4);
    assert_eq!(normal.kl.sub_l_exact_raw(), yaw);
    assert_eq!(
        restricted.kl.sub_l_target_raw(),
        normal.kl.sub_l_target_raw()
    );
    assert_eq!(
        restricted.kl.sub_k_smoothed_raw(),
        normal.kl.sub_k_smoothed_raw()
    );
    assert_eq!(restricted.kl.sub_l_exact_raw(), normal.kl.sub_l_exact_raw());
    assert_ne!(normal.kl.model_variables_raw(), &[0; 4]);
    assert_eq!(restricted.kl.model_variables_raw(), &[0; 4]);
    for (fixture, expected_cursor) in [(&normal, 1), (&restricted, 0)] {
        let RetailRuntimeValue::Known(Some(h)) = &fixture.entity.sub_h_external_frame_runtime
        else {
            panic!();
        };
        assert_eq!(h.cursor(), expected_cursor);
        assert_eq!(
            fixture.entity.physical_body_basis_q31(),
            RetailRuntimeValue::Known(fixture.basis)
        );
        assert_eq!(
            fixture.entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(7)
        );
    }
    assert_eq!(normal.sub_d_runtime, restricted.sub_d_runtime);
    assert_eq!(normal.sub_d_owner, restricted.sub_d_owner);
    assert_eq!(
        normal.entity.position_raw(),
        restricted.entity.position_raw()
    );
    assert_eq!(
        normal.entity.velocity_raw(),
        restricted.entity.velocity_raw()
    );
}

#[v2k_test_support::retail_test]
fn type30_kl_null_target_retains_inputs_and_reads_motion_before_c_a_b() {
    let Some(metadata) = canonical_metadata() else {
        return;
    };
    let mut fixture = Fixture::new(metadata);
    fixture.prime_null_target_components();
    let sub_d_before = (fixture.sub_d_runtime, fixture.sub_d_owner);
    assert!(fixture.run(CommonMoverDispatchMode::Normal, None).unwrap());
    assert_eq!(fixture.kl.sub_l_target_raw(), [10_001, 50, 10_001]);
    assert_eq!(fixture.kl.sub_k_smoothed_raw(), 64);
    assert_eq!(fixture.kl.sub_l_exact_raw(), 128);
    assert_eq!((fixture.sub_d_runtime, fixture.sub_d_owner), sub_d_before);
    // K uses incoming Y velocity -125. L reads incoming height -20 against
    // retained target50. C later changes both motion inputs.
    assert_eq!(fixture.kl.model_variables_raw(), &[-2048, 768, -256, 250]);
    assert_ne!(fixture.entity.position_raw()[1], -20);
    assert_ne!(fixture.entity.velocity_raw()[1], -125);
    assert_eq!(
        fixture.entity.collision.animation_offset_at_0xb2,
        RetailRuntimeValue::Known(7)
    );
}

#[v2k_test_support::retail_test]
fn type30_kl_h_block_preserves_target_steering_and_post_sub_d_prefix() {
    let Some(metadata) = canonical_metadata() else {
        return;
    };
    let mut fixture = Fixture::new(metadata);
    fixture.sub_d_runtime.yaw_rate_raw = 8192;
    fixture.entity.sub_h_external_frame_runtime = RetailRuntimeValue::Known(None);
    let before_motion = (fixture.entity.position_raw(), fixture.entity.velocity_raw());
    let before_angles = fixture.entity.rotation_heading_pitch_roll_raw();
    let mut target = WanderNearPrivateState::tracked_entity([12_048, 120, 11_024], 0);
    assert_eq!(
        fixture.run(CommonMoverDispatchMode::Normal, Some(&mut target)),
        Err(Intro2CommonMoverBlock::Runtime("Sub-H allocation"))
    );
    let yaw = i32::from(fixture.sub_d_runtime.last_yaw_step_raw);
    assert_ne!(yaw, 0);
    assert_eq!(fixture.kl.sub_l_target_raw(), target.target_position_raw);
    assert_eq!(fixture.kl.sub_k_smoothed_raw(), yaw >> 4);
    assert_eq!(fixture.kl.sub_l_exact_raw(), yaw);
    assert_eq!(fixture.kl.model_variables_raw(), &[0; 4]);
    assert_eq!(
        fixture.entity.rotation_heading_pitch_roll_raw()[0],
        before_angles[0].wrapping_sub(yaw as i16)
    );
    assert_eq!(
        (fixture.entity.position_raw(), fixture.entity.velocity_raw()),
        before_motion
    );
}

#[v2k_test_support::retail_test]
fn type30_kl_missing_bank_rejects_before_target_or_body_mutation() {
    let Some(metadata) = canonical_metadata() else {
        return;
    };
    let mut fixture = Fixture::new(metadata);
    let before_motion = (fixture.entity.position_raw(), fixture.entity.velocity_raw());
    let before_angles = fixture.entity.rotation_heading_pitch_roll_raw();
    let mut target = WanderNearPrivateState::tracked_entity([12_048, 120, 11_024], 0);
    let before_target = target;
    assert_eq!(
        run_intro2_common_mover(
            &mut fixture.entity,
            Intro2CommonMoverFrame {
                metadata: &fixture.metadata,
                topology: ABCDEHKL_TOPOLOGY,
                terrain: &fixture.terrain,
                wave_tick_50hz: None,
                dispatch_mode: CommonMoverDispatchMode::Normal,
                elapsed_micros: 20_000,
                global_elapsed_micros: 40_000,
                sub_d_runtime: &mut fixture.sub_d_runtime,
                sub_d_owner: &mut fixture.sub_d_owner,
                kl_components: None,
                target: Some(&mut target),
                tracked_target: RetailRuntimeValue::Known(None),
            },
            &mut || panic!("bank rejection precedes RNG"),
        ),
        Err(Intro2CommonMoverBlock::Runtime(
            "Sub-K/L allocation or variable bindings"
        ))
    );
    assert_eq!(target, before_target);
    assert_eq!(
        (fixture.entity.position_raw(), fixture.entity.velocity_raw()),
        before_motion
    );
    assert_eq!(
        fixture.entity.rotation_heading_pitch_roll_raw(),
        before_angles
    );
    let RetailRuntimeValue::Known(Some(h)) = &fixture.entity.sub_h_external_frame_runtime else {
        panic!();
    };
    assert_eq!(h.cursor(), 0);
}

#[v2k_test_support::retail_test]
fn type30_kl_late_a_block_preserves_h_k_l_and_c_without_replaying_prefix() {
    let Some(metadata) = canonical_metadata() else {
        return;
    };
    let mut fixture = Fixture::new(metadata);
    fixture.prime_null_target_components();
    fixture.entity.sub_a_propulsion_runtime = RetailRuntimeValue::Unresolved;
    let RetailRuntimeValue::Known(Some(c)) = fixture.metadata.sub_c_lift_descriptor else {
        panic!();
    };
    let c = HoverLiftConfig::from(c);
    let mut expected_position = fixture.entity.position_raw();
    let mut expected_velocity = fixture.entity.velocity_raw();
    let point =
        sub_c_surface_sample_point_raw(expected_position, fixture.basis.up, c.offset_sample);
    apply_sub_c_lift_raw(
        c,
        &mut expected_position,
        &mut expected_velocity,
        20_000,
        SubCSurfaceSample::Terrain {
            terrain_y_raw: fixture.terrain.bilinear_height_raw(point[0], point[1]),
        },
        fixture.basis.up,
    );
    assert_eq!(
        fixture.run(CommonMoverDispatchMode::Normal, None),
        Err(Intro2CommonMoverBlock::MoverAdvance(
            CommonMoverFrameAdvanceError::Block(CommonMoverFrameBlock::UnresolvedSubARuntime)
        ))
    );
    assert_eq!(fixture.kl.model_variables_raw(), &[-2048, 768, -256, 250]);
    assert_eq!(fixture.entity.position_raw(), expected_position);
    assert_eq!(fixture.entity.velocity_raw(), expected_velocity);
    let RetailRuntimeValue::Known(Some(h)) = &fixture.entity.sub_h_external_frame_runtime else {
        panic!();
    };
    assert_eq!(h.cursor(), 1);
    assert_eq!(
        fixture.entity.collision.animation_offset_at_0xb2,
        RetailRuntimeValue::Known(7)
    );
    // The caller receives the committed error and must park that callback.
    // No hidden retry or constructor reset is performed by the shared phase.
    assert_eq!(fixture.kl.sub_l_target_raw(), [10_001, 50, 10_001]);
    assert_eq!(fixture.kl.sub_k_smoothed_raw(), 64);
    assert_eq!(fixture.kl.sub_l_exact_raw(), 128);
}
