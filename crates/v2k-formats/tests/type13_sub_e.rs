//! First-world type-13 Section-12 Sub-E used by class-7 ADE0 Aim.
//!
//! PRELOAD system level 2 contributes global types 0 and 1, so local record
//! 11 in `*X3XX.OVL` is cumulative entity type 13 (`ptersect`, model 291).
//! Sub-E is nonvisual and stays identical across retail presentation tiers.
//! The demo build keeps the same method and cadence but authors sound 77.

use std::path::PathBuf;

use v2k_formats::collision::{
    BehaviorChoice, CommonAxisDescriptor, ProjectileEmitterDescriptor, SubDSteeringDescriptor,
};

fn overlay_dir() -> PathBuf {
    v2k_test_support::retail_dir().join("Overlay")
}

fn demo_overlay_dir() -> PathBuf {
    v2k_test_support::demo_dir().join("OVERLAY")
}

fn load_ovl(name: &str) -> v2k_formats::ovl::OvlFile {
    load_from(overlay_dir(), name)
}

fn load_demo_ovl(name: &str) -> v2k_formats::ovl::OvlFile {
    load_from(demo_overlay_dir(), name)
}

fn load_from(dir: PathBuf, name: &str) -> v2k_formats::ovl::OvlFile {
    let path = dir.join(name);
    let data = std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    v2k_formats::ovl::OvlFile::parse(&data)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// Exact first-world type-13 Sub-E payload recovered from Section 12.
const TYPE13_PROJECTILE_EMITTER: ProjectileEmitterDescriptor = ProjectileEmitterDescriptor {
    projectile_method: 10,
    random_interval_us: 600_000,
    spread_raw: 128,
    aim_threshold_raw: 20_000,
    speed_override_raw: 0,
    target_axis_tolerance_raw: 0x0F00,
    sound_id: 75,
    raw_word_at_0x12: 26,
    alternate_emitter_raw: 0,
    stochastic_gate_mode: 0,
    auxiliary_command: 0,
    variable_bindings: [0; 4],
};

#[v2k_test_support::retail_test]
fn first_world_type13_retains_authored_sub_e_across_retail_tiers() {
    assert_type13_sub_e(false);
}

#[v2k_test_support::demo_test]
fn first_world_type13_retains_authored_sub_e_across_demo_tiers() {
    assert_type13_sub_e(true);
}

fn assert_type13_sub_e(demo: bool) {
    let tiers = if demo { 0..=1 } else { 0..=3 };
    for tier in tiers {
        let load: fn(&str) -> v2k_formats::ovl::OvlFile =
            if demo { load_demo_ovl } else { load_ovl };
        let system = load(&format!("{tier}X3XX.OVL"));
        let collision = v2k_formats::sections::parse_collision(&system).unwrap();
        let type13 = &collision.entries[11];
        let build = if demo { "demo" } else { "retail" };
        let label = format!("{build} tier {tier}");

        assert_eq!(type13.index, 11, "{label}: local record");
        assert_eq!(type13.model_ids, [291; 4], "{label}: ptersect models");
        assert_eq!(
            type13
                .subsections
                .iter()
                .map(|sub| sub.name)
                .collect::<Vec<_>>(),
            ["D", "E", "G", "K", "L"],
            "{label}: component topology"
        );
        let mut expected = TYPE13_PROJECTILE_EMITTER;
        if demo {
            expected.sound_id = 77;
        }
        assert_eq!(
            type13.projectile_emitter_descriptor(),
            Some(expected),
            "{label}: Sub-E"
        );
        assert_eq!(
            type13.search_attack_optional_prelude_sound_id(),
            None,
            "{label}: ADE0 prelude +0x9A"
        );
        assert_eq!(
            type13.search_attack_aim_sound_id(),
            None,
            "{label}: ADE0 Aim +0x9C"
        );
        assert_eq!(
            type13.search_attack_aim_sound_period_raw(),
            0,
            "{label}: ADE0 Aim +0xA8"
        );
        assert_eq!(
            type13.common_axis_descriptor(),
            CommonAxisDescriptor {
                strict_axis_limit_raw: 0x1900,
                raw_word_at_0x04: 0x0C05,
            },
            "{label}: common-axis"
        );
        assert_eq!(
            type13.sub_d_steering_descriptor(),
            Some(SubDSteeringDescriptor {
                steering_divisor_raw: 64,
                couple_yaw_into_roll_raw: 1,
                enable_pitch_steering_raw: 0,
                forward_probe_raw: 0x0200,
                lateral_probe_raw: 0x0100,
                classifier_flags: 0,
                reserved_at_0x0b: 0,
            }),
            "{label}: Sub-D"
        );
        let mut expected_g = [
            0, 0, 0, 0, 100, 0, 0, 0, 0, 0, 0, 0, 200, 0, 69, 0, 50, 0, 0, 0, 130, 0, 0, 0, 44, 1,
            0, 0, 0, 64, 0, 0, 220, 5, 0, 0, 1, 2, 3, 4, 5, 8, 9, 0, 0, 0, 0, 32, 0, 0, 0, 0, 1, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 240,
            0, 17, 230, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 16, 210, 0, 0, 0, 1, 0, 0, 0,
        ];
        if demo {
            expected_g[14] = 71;
        }
        assert_eq!(type13.sub_g_payload(), Some(expected_g), "{label}: Sub-G");
        assert_eq!(type13.sub_k_payload(), Some([11, 10]), "{label}: Sub-K");
        assert_eq!(
            type13.sub_l_payload(),
            Some([6, 7, 64, 31, 64, 31]),
            "{label}: Sub-L"
        );
        assert_eq!(
            type13.behavior_choices,
            [
                BehaviorChoice {
                    weight_rule_id: 1,
                    weight_multiplier: 1,
                    behavior_class_id: 5,
                },
                BehaviorChoice {
                    weight_rule_id: 1,
                    weight_multiplier: 3,
                    behavior_class_id: 7,
                },
            ],
            "{label}: Always x1 Move About Aimlessly / Always x3 Search And Attack"
        );
        assert_eq!(type13.behavior_rule_ref, 3, "{label}: +0x11C");
        assert_eq!(type13.alternate_behavior_class_ref, 1, "{label}: +0x124");
    }
}
