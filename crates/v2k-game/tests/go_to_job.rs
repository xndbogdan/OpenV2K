use v2k_game::common_mover::actor_abdi::FIRST_WORLD_PEASANT_ENTITY_TYPE;
use v2k_game::entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue};
use v2k_game::go_to_job::{
    plan_go_to_job_setup, GoToJobOwner, GoToJobSetupRequest, GO_TO_JOB_FIRST_WORLD_SCIENTIST_TYPE,
};
use v2k_game::wrapped_axis_range::WrappedAxisRange;

fn high_resolution_first_world_overlay() -> v2k_formats::ovl::OvlFile {
    let path = v2k_test_support::retail_dir()
        .join("Overlay")
        .join("1X3XX.OVL");
    let bytes = std::fs::read(path).unwrap();
    v2k_formats::ovl::OvlFile::parse(&bytes).unwrap()
}

#[v2k_test_support::retail_test]
fn first_world_scientist_and_type9_go_to_job_derive_the_authored_constructor_suffix() {
    let overlay = high_resolution_first_world_overlay();
    let collision = v2k_formats::sections::parse_collision(&overlay).unwrap();

    // PRELOAD contributes cumulative types 0 and 1, so first-world local
    // records 6 and 7 are cumulative type 8 (`scientist`) and type 9
    // (`peasant`). Variant 1 is the normal high-resolution presentation tier
    // even though Section 12 is nonvisual.
    for (local_record, entity_type, owner_name) in [
        (6, GO_TO_JOB_FIRST_WORLD_SCIENTIST_TYPE, "type 8"),
        (7, FIRST_WORLD_PEASANT_ENTITY_TYPE, "type 9"),
    ] {
        let record = &collision.entries[local_record];
        let metadata = EntityTypeRuntimeMetadata::from_section12(record);
        let descriptor = record
            .sub_a_propulsion_descriptor()
            .unwrap_or_else(|| panic!("{owner_name} must author the Sub-A descriptor"));
        let RetailRuntimeValue::Known(topology) = metadata.common_mover_topology else {
            panic!("retail {owner_name} Section 12 must resolve component topology");
        };
        assert!(
            topology.sub_a,
            "Go To Job resets {owner_name}'s live Sub-A component"
        );
        assert!(
            !topology.sub_h,
            "{owner_name} has no preceding Sub-H generic-constructor effect"
        );
        assert!(
            !topology.sub_g,
            "{owner_name} has no preceding Sub-G generic-constructor effect"
        );
        assert!(
            !topology.sub_f,
            "{owner_name} has no preceding or class-specific Sub-F effect"
        );
        assert_eq!(
            metadata.sub_a_propulsion_descriptor,
            RetailRuntimeValue::Known(Some(descriptor))
        );

        let owner = GoToJobOwner::from_type_metadata(
            0x04F4_0001 + u32::from(entity_type),
            [0x0123, -0x0234, 0x0345],
            RetailRuntimeValue::Known(metadata.capability_flags),
            entity_type,
            &metadata,
        );
        let plan = plan_go_to_job_setup(GoToJobSetupRequest {
            owner,
            candidates_in_intrusive_order: &[],
            range: WrappedAxisRange::strict(record.common_axis_descriptor().strict_axis_limit_raw)
                .unwrap_or_else(|| panic!("{owner_name} must author a positive strict range")),
        })
        .unwrap_or_else(|error| panic!("{owner_name} suffix must authenticate: {error:?}"));
        assert_eq!(plan.target_id(), None);
        assert_eq!(
            plan.constructor_sub_a_target_speed_raw(),
            i32::from(descriptor.target_speed_base_raw) * 4 / 3
        );
    }
}
