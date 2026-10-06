use v2k_formats::collision::StatusComponentDescriptor;
use v2k_game::base_factory_progression::ProgressiveDeathState;
use v2k_game::entity::BaseFactoryRuntimeState;
use v2k_game::factory_production::{
    FactoryProductionRuntime, FactorySection13Config, FACTORY_CONFIG_BYTES,
};
use v2k_game::factory_production_live::FactoryStatusPublication;
use v2k_game::factory_status_runtime::project_factory_status;

fn level_one_production() -> FactoryProductionRuntime {
    let mut raw = [0_u8; FACTORY_CONFIG_BYTES];
    for (offset, value) in [
        (0x00, 0x0001_f412_u32),
        (0x04, 2),
        (0x08, 6_000_000),
        (0x0c, 5_000_000),
        (0x10, (-1_000_i32) as u32),
    ] {
        raw[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    FactoryProductionRuntime::from_retail_template(FactorySection13Config::decode(&raw), 99_999)
}

fn factory_words() -> BaseFactoryRuntimeState {
    BaseFactoryRuntimeState {
        // Level 1 type 66's audited Sub-M selectors publish delivery,
        // production, required, and current through callback indices 1..4.
        status_descriptor: StatusComponentDescriptor {
            raw_word_at_0x00: 8,
            variable_bindings: [4, 3, 0, 2, 1, 0],
            raw_tail: [0; 10],
        },
        control_value_raw: 0,
        required_scientists: 2,
        current_scientists: 1,
        lifter_progress_raw: 0,
        production_progress_raw: 0,
        recovery_progress_raw: 0,
        production: Some(level_one_production()),
        live_owner: None,
        progressive_death: ProgressiveDeathState::idle(0x21),
    }
}

#[test]
fn final_staff_publication_projects_the_exact_pole_channels() {
    // 20260731-001224-second-scientist-factory-activation-v2.txt records the
    // final 1 -> 2 intake at tick 0xC7E and the corresponding 2/2 Sub-M pole
    // publication at tick 0xC80.
    let before = factory_words();
    let publication = FactoryStatusPublication {
        current_scientists_raw: 2,
        scientist_capacity_raw: 2,
        // The live owner already publishes a zero-extended byte. Exercise the
        // pure boundary defensively so a freely constructed value cannot
        // smuggle a high byte into the retained status word.
        output_selector_raw: 0xAB12,
        production_or_cooldown_ratio_raw: 0x1111,
        delivery_or_cooldown_ratio_raw: 0x2222,
        understaffed_ratio_raw: 0x3333,
    };

    let applied = project_factory_status(before, publication);
    assert_eq!(applied.before, before);
    assert_eq!(applied.after.current_scientists, 2);
    assert_eq!(applied.after.required_scientists, 2);
    assert_eq!(applied.after.control_value_raw, 0x12);
    assert_eq!(applied.after.production_progress_raw, 0x1111);
    assert_eq!(applied.after.lifter_progress_raw, 0x2222);
    assert_eq!(applied.after.recovery_progress_raw, 0x3333);
    assert_eq!(
        applied.after.progressive_death,
        ProgressiveDeathState::idle(0x21)
    );
}

#[test]
fn projection_zero_extends_only_the_low_selector_byte_and_preserves_owners() {
    let before = factory_words();
    let publication = FactoryStatusPublication {
        current_scientists_raw: 2,
        scientist_capacity_raw: 2,
        output_selector_raw: 0x12,
        production_or_cooldown_ratio_raw: 0,
        delivery_or_cooldown_ratio_raw: 0,
        understaffed_ratio_raw: 0,
    };

    let projected = project_factory_status(before, publication);
    assert_eq!(projected.after.control_value_raw, 0x12);
    assert_eq!(projected.after.status_descriptor, before.status_descriptor);
    assert_eq!(projected.after.production, before.production);
    assert_eq!(projected.after.progressive_death, before.progressive_death);
}
