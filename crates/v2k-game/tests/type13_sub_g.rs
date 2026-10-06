use v2k_formats::terrain::{TerrainCell, TerrainGrid};
use v2k_game::{
    common_mover::type9_attitude::Type9BodyBasis,
    entity_collision_state::RetailRuntimeValue,
    search_attack_live::TYPE13_SEARCH_ATTACK_SUB_G,
    sub_g_runtime::{plan_type13_sub_g_frame, SubG06070RuntimeState, Type13SubGFrameRequest},
};

fn flat_dry_terrain() -> TerrainGrid {
    TerrainGrid {
        header: [-6_144 << 8, 0, 0, 0, 0],
        cells: vec![
            TerrainCell {
                height: 0,
                attribute: 0,
                terrain_type: 0,
            };
            256 * 256
        ],
    }
}

fn initialized_runtime(target_clearance_raw: i32) -> SubG06070RuntimeState {
    let mut runtime = SubG06070RuntimeState::from_1b8c0_constructor(&TYPE13_SEARCH_ATTACK_SUB_G, 0);
    runtime.apply_shared_06070_sub_g_branch(target_clearance_raw, 0);
    runtime
}

fn frame<'a>(
    terrain: &'a TerrainGrid,
    runtime: SubG06070RuntimeState,
    position_raw: [i16; 3],
    velocity_raw: [i16; 3],
    elapsed_micros: u32,
) -> Type13SubGFrameRequest<'a> {
    Type13SubGFrameRequest {
        descriptor: &TYPE13_SEARCH_ATTACK_SUB_G,
        runtime,
        reverse_write: None,
        position_raw,
        velocity_raw,
        pitch_raw: 0,
        roll_raw: 1_000,
        retained_body_basis: Type9BodyBasis::from_angle_words(0, 0, 0),
        active_model_extent_raw: 480,
        self_mass_raw: 100,
        attached_cargo_mass: 0,
        capability_flags: 8,
        terrain,
        retail_tick: 0,
        elapsed_micros,
    }
}

#[test]
fn first_frame_publishes_before_rate_and_clamps_velocity() {
    let terrain = flat_dry_terrain();
    let outcome = plan_type13_sub_g_frame(frame(
        &terrain,
        initialized_runtime(217),
        [0, 3_000, 0],
        [2_000, 2_000, -2_000],
        20_000,
    ))
    .expect("normal Type-13 Sub-G frame");

    assert_eq!(outcome.velocity_raw, [1_500, 1_500, -1_500]);
    assert_eq!(outcome.pitch_raw, 986);
    assert_eq!(outcome.roll_raw, 926);
    assert_eq!(outcome.sound, None);
    assert_eq!(
        outcome.runtime.rate_raw_at_0x34(),
        RetailRuntimeValue::Known(50)
    );
    assert_eq!(
        outcome.runtime.clearance_flag_at_0x3d(),
        RetailRuntimeValue::Known(1)
    );
    assert_eq!(
        outcome.runtime.audio_gate_at_0x3e(),
        RetailRuntimeValue::Known(0)
    );
    assert_eq!(
        outcome.runtime.source_raw_at_0x20(),
        RetailRuntimeValue::Known(0)
    );
    let RetailRuntimeValue::Known(outputs) = outcome.runtime.animation_outputs_raw() else {
        panic!("constructor-resolved animation bindings must publish");
    };
    assert_eq!(outputs[0], 0, "first phase advances at prior rate zero");
    assert_eq!(outputs[1], 0);
    assert_eq!(outputs[2], 0);
    assert_eq!(outputs[5], 0);
    assert_eq!(outputs[6], 0);
}

#[test]
fn phase_band_uses_full_strength_and_emits_sound_once() {
    let terrain = flat_dry_terrain();
    let mut runtime = initialized_runtime(455);
    let mut velocity_raw = [0; 3];
    let mut sounds = Vec::new();

    for _ in 0..9 {
        let outcome = plan_type13_sub_g_frame(frame(
            &terrain,
            runtime,
            [0, -1_000, 0],
            velocity_raw,
            32_768,
        ))
        .expect("normal Type-13 Sub-G frame");
        runtime = outcome.runtime;
        velocity_raw = outcome.velocity_raw;
        if let Some(sound) = outcome.sound {
            sounds.push(sound);
        }
    }

    assert_eq!(sounds.len(), 1);
    assert_eq!(sounds[0].sound_id, 69);
    assert_eq!(sounds[0].rate_q16, 0x0000_aaaa);
    assert_eq!(
        runtime.source_raw_at_0x20(),
        RetailRuntimeValue::Known(208),
        "bound selector one uses 100 * 130^2 / 90^2 without quarter scaling"
    );
    assert_eq!(runtime.audio_gate_at_0x3e(), RetailRuntimeValue::Known(1));
}

#[test]
fn collision_pulse_is_published_before_aa60_decay() {
    let terrain = flat_dry_terrain();
    let mut runtime = initialized_runtime(217);
    runtime.apply_collision_pulse_1b9d0();

    let outcome = plan_type13_sub_g_frame(frame(&terrain, runtime, [0, 3_000, 0], [0; 3], 20_000))
        .expect("normal Type-13 Sub-G frame");
    let RetailRuntimeValue::Known(outputs) = outcome.runtime.animation_outputs_raw() else {
        panic!("constructor-resolved animation bindings must publish");
    };
    assert_eq!(outputs[6], 0x3000);
    assert_eq!(
        outcome.runtime.decaying_output_raw_at_0x30(),
        RetailRuntimeValue::Known(11_820)
    );
}
