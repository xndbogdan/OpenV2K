use v2k_game::common_mover::type9::{OrdinaryType9Topology, ORDINARY_TYPE9_ENTITY_TYPE};
use v2k_game::common_mover::type9_surface::ORDINARY_TYPE9_AUTHORED_LIFETIME_MS;
use v2k_game::entity_collision_state::{
    CommonMoverComponentTopology, EntityTypeRuntimeMetadata, RetailRuntimeValue,
};

fn first_world_overlay(variant: u8) -> v2k_formats::ovl::OvlFile {
    let path = v2k_test_support::retail_dir()
        .join("Overlay")
        .join(format!("{variant}X3XX.OVL"));
    let bytes = std::fs::read(path).unwrap();
    v2k_formats::ovl::OvlFile::parse(&bytes).unwrap()
}

#[v2k_test_support::retail_test]
fn high_resolution_first_world_type9_proves_the_ordinary_mover_route() {
    let overlay = first_world_overlay(1);
    let collision = v2k_formats::sections::parse_collision(&overlay).unwrap();

    // PRELOAD contributes cumulative types 0 and 1, so first-world local
    // record 7 is cumulative entity type 9.
    let peasant = &collision.entries[7];
    assert_eq!(
        (peasant.raw_header[0x72], peasant.raw_header[0x73]),
        (1, 0),
        "the callback crosses the nontrivial FUN_0040E370 surface-effects owner"
    );
    let metadata = EntityTypeRuntimeMetadata::from_section12(peasant);
    assert!(OrdinaryType9Topology::from_metadata(ORDINARY_TYPE9_ENTITY_TYPE, &metadata).is_ok());
    assert_eq!(
        metadata.common_mover_topology,
        RetailRuntimeValue::Known(CommonMoverComponentTopology {
            sub_a: true,
            sub_b: true,
            sub_d: true,
            sub_i: true,
            ..CommonMoverComponentTopology::default()
        })
    );
    let initializer = metadata
        .initializer
        .as_ref()
        .expect("type 9 has an authored initializer");
    assert_eq!(
        initializer
            .behavior_choices
            .iter()
            .map(|choice| (
                choice.weight_rule_id,
                choice.weight_multiplier,
                choice.behavior_class_id,
            ))
            .collect::<Vec<_>>(),
        vec![
            (7, 10, 10),   // Baddie Nearby -> Run Away
            (6, 3, 45),    // Player Nearby -> Attract Attention
            (12, 200, 54), // Base Nearby -> Go To Job
            (1, 1, 6),     // Always -> Wander Near Location
        ],
        "live admission must not substitute a capture-specific behavior"
    );
}

#[v2k_test_support::retail_test]
fn every_high_resolution_first_world_tier_retains_type9_runtime_metadata() {
    for variant in 1..=3 {
        let overlay = first_world_overlay(variant);
        let collision = v2k_formats::sections::parse_collision(&overlay).unwrap();
        let peasant = &collision.entries[7];
        let metadata = EntityTypeRuntimeMetadata::from_section12(peasant);
        let lifetime_ms = u32::from_le_bytes(peasant.raw_header[0x74..0x78].try_into().unwrap());
        assert_eq!((peasant.raw_header[0x72], peasant.raw_header[0x73]), (1, 0));
        assert_eq!(
            metadata.run_away_optional_sound_id,
            RetailRuntimeValue::Known(Some(85)),
            "all high-resolution retail presentation tiers retain the Type-9 flee cue"
        );
        assert_eq!(
            metadata.run_away_sound_period_raw,
            RetailRuntimeValue::Known(0),
            "the authored zero period suppresses flee audio and its RNG draw"
        );
        assert_eq!(
            lifetime_ms, ORDINARY_TYPE9_AUTHORED_LIFETIME_MS,
            "FUN_004162B0 ages against this exact authored duration"
        );
    }
}
