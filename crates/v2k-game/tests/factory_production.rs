use v2k_game::entity_collision_state::EntityTypeRuntimeMetadata;
use v2k_game::factory_production::{
    FactoryProductionPhase, FactoryProductionRuntime, FactorySection13Config,
};
use v2k_game::session::GameSession;

#[v2k_test_support::retail_test]
fn level_one_factory_config_matches_retail_state_template() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("init session");
    session.load_auxiliary_ovl(3, 1).expect("load L3 aux");
    session
        .load_level_by_id(13, 1)
        .expect("load Level 1 into cache");
    let state = session.load_ovl_by_id(13, 1).expect("load Level 1");
    let factory = state
        .level
        .as_ref()
        .expect("Section 13")
        .entities
        .iter()
        .find(|spawn| spawn.entity_type == 66)
        .expect("Working Factory spawn");
    let config =
        FactorySection13Config::decode(&factory.config.expect("Working Factory 0x58-byte config"));
    assert_eq!(
        config.raw_words(),
        [
            0x0001_F412,
            2,
            6_000_000,
            5_000_000,
            (-1_000_i32) as u32,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            31,
            0,
        ]
    );

    let record = session
        .cache
        .global_entity_type(66)
        .expect("Working Factory Section-12 record");
    let metadata = EntityTypeRuntimeMetadata::from_section12(record);
    assert_eq!(metadata.initial_health_raw, Some(99_999));
    let runtime = FactoryProductionRuntime::from_retail_template(config, record.dims[0] as i32);
    assert_eq!(runtime.output_payload_packed, 0x0001_F412);
    assert_eq!(runtime.scientist_capacity_raw, 2);
    assert_eq!(runtime.remaining_stock_raw, 1);
    assert_eq!(runtime.health_per_scientist_raw, 49_999);
    assert_eq!(runtime.repair_rate_raw, 833);
    assert_eq!(runtime.phase, FactoryProductionPhase::Producing);
}
