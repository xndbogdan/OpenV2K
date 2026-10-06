//! First-world type-13 Section-12 Sub-E reaches ADE0 Aim as Known data.

use std::path::PathBuf;

use v2k_game::entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue};
use v2k_game::search_attack_live::{
    TYPE13_SEARCH_ATTACK_ALTERNATE_BEHAVIOR_CLASS, TYPE13_SEARCH_ATTACK_BEHAVIOR_CHOICES,
    TYPE13_SEARCH_ATTACK_BEHAVIOR_RULE_REF, TYPE13_SEARCH_ATTACK_COMMON_AXIS,
    TYPE13_SEARCH_ATTACK_GKL, TYPE13_SEARCH_ATTACK_PROJECTILE_DESCRIPTOR,
    TYPE13_SEARCH_ATTACK_SUB_D, TYPE13_SEARCH_ATTACK_SUB_G_1B8C0_TABLE_BYTES,
    TYPE13_SEARCH_ATTACK_SUB_G_RANDOMIZED_TARGET_BASE_RAW_AT_0X0C,
    TYPE13_SEARCH_ATTACK_SUB_G_SOURCE_RAW_AT_0X00, TYPE13_SEARCH_ATTACK_TOPOLOGY,
};

fn overlay_dir() -> PathBuf {
    v2k_test_support::retail_dir().join("Overlay")
}

#[v2k_test_support::retail_test]
fn first_world_type13_section12_is_known_sub_e_for_ade0_aim() {
    let path = overlay_dir().join("1X3XX.OVL");
    let data = std::fs::read(&path).unwrap();
    let ovl = v2k_formats::ovl::OvlFile::parse(&data).unwrap();
    let collision = v2k_formats::sections::parse_collision(&ovl).unwrap();
    let metadata = EntityTypeRuntimeMetadata::from_section12(&collision.entries[11]);
    assert_eq!(metadata.model_slots, [291; 4]);
    assert_eq!(
        metadata.projectile_emitter_descriptor,
        RetailRuntimeValue::Known(Some(TYPE13_SEARCH_ATTACK_PROJECTILE_DESCRIPTOR))
    );
    assert_eq!(
        metadata.search_attack_optional_prelude_sound_id,
        RetailRuntimeValue::Known(None)
    );
    assert_eq!(
        metadata.search_attack_aim_sound_id,
        RetailRuntimeValue::Known(None)
    );
    assert_eq!(
        metadata.search_attack_aim_sound_period_raw,
        RetailRuntimeValue::Known(0)
    );
    assert_eq!(
        metadata.common_mover_topology,
        RetailRuntimeValue::Known(TYPE13_SEARCH_ATTACK_TOPOLOGY)
    );
    assert_eq!(
        metadata
            .initializer
            .as_ref()
            .map(|initializer| initializer.common_axis_descriptor),
        Some(TYPE13_SEARCH_ATTACK_COMMON_AXIS)
    );
    assert_eq!(
        metadata.sub_d_steering_descriptor,
        RetailRuntimeValue::Known(Some(TYPE13_SEARCH_ATTACK_SUB_D))
    );
    assert_eq!(
        metadata.common_mover_gkl_payloads,
        RetailRuntimeValue::Known(TYPE13_SEARCH_ATTACK_GKL)
    );
    assert_eq!(TYPE13_SEARCH_ATTACK_SUB_G_SOURCE_RAW_AT_0X00, 0);
    assert_eq!(
        TYPE13_SEARCH_ATTACK_SUB_G_RANDOMIZED_TARGET_BASE_RAW_AT_0X0C,
        200
    );
    assert_eq!(
        TYPE13_SEARCH_ATTACK_SUB_G_1B8C0_TABLE_BYTES,
        [0, 0, 0, 230, 210]
    );
    let initializer = metadata.initializer.expect("type-13 initializer");
    assert_eq!(
        initializer.behavior_choices.as_ref(),
        TYPE13_SEARCH_ATTACK_BEHAVIOR_CHOICES.as_slice()
    );
    assert_eq!(
        initializer.behavior_rule_ref,
        TYPE13_SEARCH_ATTACK_BEHAVIOR_RULE_REF
    );
    assert_eq!(
        initializer.alternate_behavior_class_ref,
        TYPE13_SEARCH_ATTACK_ALTERNATE_BEHAVIOR_CLASS
    );
    assert_eq!(
        EntityTypeRuntimeMetadata::default().projectile_emitter_descriptor,
        RetailRuntimeValue::Unresolved,
        "compatibility metadata must not acquire type-13 Sub-E"
    );
}
