use super::*;
use v2k_formats::terrain::TerrainCell;

fn descriptor() -> SubFSwimmingDescriptor {
    SubFSwimmingDescriptor {
        clearance_base_raw: -256,
        clearance_random_span_raw: 512,
        target_speed_base_raw: 100,
        animation_mode_raw: 0,
        variable_selectors: [1, 0, -1, 2, 2, 7],
        reserved_at_0x0d: 0,
    }
}

#[test]
fn constructor_matches_pe_conditional_draws_and_signed_selector_order() {
    // Actual PE424220, mocked allocator/RNG/40A950 leaves. The third draw
    // may yield a speed BELOW50 even though the second passed the floor gate.
    for (span, words, expected_clearance, expected_speed) in [
        (-512, vec![0x8000, 0xffff], -512, 50),
        (512, vec![0x8000, 0, 0xffff], 0, -155),
    ] {
        let mut descriptor = descriptor();
        descriptor.clearance_random_span_raw = span;
        let mut random = words.into_iter();
        let mut selectors = Vec::new();
        let runtime = SubFSwimmingRuntime::construct(
            descriptor,
            &mut || random.next().unwrap(),
            &mut |selector| {
                selectors.push(selector);
                Some((selector as u8 & 7) as usize)
            },
        );
        assert!(random.next().is_none());
        assert_eq!(selectors, [1, -1, 2, 2, 7]);
        assert_eq!(
            runtime.variable_bindings,
            [Some(1), None, Some(7), Some(2), Some(2), Some(7)]
        );
        assert_eq!(runtime.clearance_raw, expected_clearance);
        assert_eq!(runtime.target_speed_raw, expected_speed);
        assert_eq!(runtime.phase_bias_raw, 0x3000);
        assert_eq!(runtime.target_position_raw, [0; 3]);
        assert_eq!(runtime.phase_raw, 0);
        assert_eq!(runtime.forward_acceleration_raw, 0);
    }
}

#[test]
fn task_resets_preserve_component_history_and_have_no_rng() {
    let mut runtime = SubFSwimmingRuntime::construct(descriptor(), &mut || 0xffff, &mut |_| None);
    runtime.target_position_raw = [10, -20, 30];
    runtime.phase_raw = 0xf000_1234;
    runtime.smoothed_turn_raw = -123;
    runtime.set_reversal_raw(255);
    runtime.set_follow_target_mode_raw(2);
    assert_eq!(
        (
            runtime.reversal_raw,
            runtime.follow_target_mode_raw,
            runtime.phase_bias_raw
        ),
        (255, 2, 0x1000)
    );
    runtime.reset_shared_task();
    assert_eq!(
        (
            runtime.reversal_raw,
            runtime.follow_target_mode_raw,
            runtime.phase_bias_raw
        ),
        (0, 0, 0xe000)
    );
    assert_eq!(runtime.phase_raw, 0xf000_1234);
    assert_eq!(runtime.smoothed_turn_raw, -123);
    assert_eq!(runtime.target_position_raw, [10, -20, 30]);
    assert_eq!(runtime.target_speed_raw, 50);
}

fn oracle_terrain() -> TerrainGrid {
    TerrainGrid {
        header: [0; 5],
        cells: (0..65536)
            .map(|i| TerrainCell {
                height: ((((i / 256) * 3 + (i % 256) * 5) % 127) as i32 - 80) as u8,
                attribute: 0,
                terrain_type: 0,
            })
            .collect(),
    }
}

#[test]
fn callback_matches_original_executable_vectors() {
    // Original PE4236D0 executed in Unicorn; seed424220, samples1/2/3/15/17/91/599
    // of600 matching cases. This is executable arithmetic evidence, not a TTD
    // fish trajectory. Fixtures include all modes, negative phase, reversals,
    // targetY, signed terrain, dt0-like subquantum steps, sea/beach, alias writes
    // and the dying branch. Layout before -> after is decoded explicitly below.
    const CASES: [&str; 7] = [
        "1 20000 33947653 105 -1000 -25676 1008 23730 -115 116 315 3319 -2555 1718212512 1093905892 680295284 -24569 -265 23184 2043559695 12288 -1261 -15 -14 427 498 0 1 0 1 2 3 4 5 13302 37083 63281 26982 32251 65057 47341 56840 -25676 -1100 23730 -17 124 347 3985 -2365 2043558525 12288 -1261 50 -21 427 498 0 1 86 37083 63281 26982 32251 35600 47341 56840",
        "2 1024 33947653 794 2000 28787 1833 2139 -110 -67 -176 1165 29275 -1798401220 620764443 996037433 29969 1044 862 2990843526 4096 1987 -16 -7 676 556 0 0 0 1 2 3 4 5 38458 42053 60749 58669 61851 32964 36470 30252 28787 1833 2139 -115 -65 -173 1155 29161 2990843536 4096 1987 50 4 676 556 0 0 65460 0 60749 58669 61851 17506 36470 30252",
        "0 1 33947653 135 4000 20092 -1000 -7889 169 -366 289 18368 -15132 260278171 896619759 1933911656 19543 -1531 -8235 705883315 4096 351 -8 16 667 936 0 1 0 1 -1 3 4 5 12917 28028 53617 1175 27608 54916 33330 19480 20092 -1000 -7889 169 -299 289 12288 -15132 705883315 4096 351 50 -10 667 936 0 1 65292 62462 53617 62814 64000 28482 33330 19480",
        "0 1 33947653 53 -1000 -12656 2035 -21653 361 463 -109 -23881 15504 -1174470589 1786587741 -201020120 -12009 2098 -21950 1559158568 57344 1858 49 -16 382 998 0 1 1 2 1 -1 6 1 16456 28797 48908 26110 23557 42573 33061 23119 -12656 -8 -21653 361 250 -108 0 16384 1559158568 57344 1858 50 1 382 998 0 1 16456 55200 50592 26110 23557 42573 1536 23119",
        "2 1024 16384 747 -1000 -7132 2528 -21521 -199 304 187 13742 -500 1632042579 873324304 1088773480 -8343 1388 -22013 2397615460 57344 -1004 -26 18 448 853 0 1 0 1 2 3 4 5 29341 38858 63906 5032 12567 37366 3905 60124 -7132 2528 -21521 -199 304 187 13716 -499 2397615460 57344 -1004 -26 18 448 853 0 1 29341 38858 63906 5032 12567 37366 3905 60124",
        "1 125000 33947653 121 4000 -6618 -2634 9058 -299 -319 -498 21685 -1731 -653460569 -725588807 -1912641153 -7104 -2478 9195 2974156799 4096 1513 4 -7 -138 384 1 1 0 1 2 3 4 5 31417 5969 41966 17011 50963 37164 28786 17890 -6618 -2634 9058 -59 -11 160 9360 -906 2974158751 4096 1513 -50 2 -138 384 1 1 20 5969 41966 17011 50963 19606 28786 17890",
        "2 125000 33947653 768 -1000 -2858 1624 -24363 133 281 -336 15171 4001 -279580313 1969527860 808999919 -3062 2349 -25438 3691661890 12288 1522 -48 -15 707 612 0 1 0 1 2 3 4 5 50323 55025 48340 13059 30588 24573 33473 37349 -2858 -1100 -24363 30 997 -43 9326 2095 3691663110 12288 1522 50 12 707 612 0 1 119 0 48340 13059 30588 15358 33473 37349",
    ];
    let mut terrain = oracle_terrain();
    for (case, line) in CASES.into_iter().enumerate() {
        let mut values = line.split_whitespace().map(|v| v.parse::<i64>().unwrap());
        let mut next = || values.next().unwrap();
        let mode = next() as u8;
        let elapsed_micros = next() as u32;
        let state_flags = next() as u32;
        let active_model_extent_raw = next() as u16;
        terrain.header[0] = next() as i32 * 256;
        let mut body = SubFSwimmingBody {
            state_flags,
            position_raw: std::array::from_fn(|_| next() as i16),
            velocity_raw: std::array::from_fn(|_| next() as i16),
            pitch_raw: next() as i16,
            roll_raw: next() as i16,
            forward_q31: std::array::from_fn(|_| next() as i32),
        };
        let mut runtime = SubFSwimmingRuntime {
            variable_bindings: [None; 6],
            target_position_raw: std::array::from_fn(|_| next() as i16),
            phase_raw: next() as u32,
            phase_bias_raw: next() as i32,
            smoothed_turn_raw: next() as i32,
            forward_acceleration_raw: next() as i32,
            vertical_acceleration_raw: next() as i32,
            clearance_raw: next() as i32,
            target_speed_raw: next() as i32,
            reversal_raw: next() as u8,
            follow_target_mode_raw: next() as u8,
        };
        runtime.variable_bindings = std::array::from_fn(|_| {
            let index = next();
            (index >= 0).then_some(index as usize)
        });
        let mut bank: [u16; 8] = std::array::from_fn(|_| next() as u16);
        let mut descriptor = descriptor();
        descriptor.animation_mode_raw = mode;
        apply_sub_f_swimming_raw(
            descriptor,
            &mut runtime,
            &mut body,
            SubFSwimmingFrame {
                terrain: &terrain,
                elapsed_micros,
                active_model_extent_raw,
            },
            &mut bank,
        )
        .unwrap();
        let got: Vec<i64> = body
            .position_raw
            .into_iter()
            .chain(body.velocity_raw)
            .chain([body.pitch_raw, body.roll_raw])
            .map(i64::from)
            .chain([
                i64::from(runtime.phase_raw),
                i64::from(runtime.phase_bias_raw),
                i64::from(runtime.smoothed_turn_raw),
                i64::from(runtime.forward_acceleration_raw),
                i64::from(runtime.vertical_acceleration_raw),
                i64::from(runtime.clearance_raw),
                i64::from(runtime.target_speed_raw),
                i64::from(runtime.reversal_raw),
                i64::from(runtime.follow_target_mode_raw),
            ])
            .chain(bank.map(i64::from))
            .collect();
        assert_eq!(got, values.collect::<Vec<_>>(), "original PE vector {case}");
    }
}

#[test]
fn unsupported_or_unbound_execution_changes_nothing() {
    let terrain = oracle_terrain();
    let mut body = SubFSwimmingBody {
        state_flags: 1,
        position_raw: [0, -2000, 0],
        velocity_raw: [1, 2, 3],
        pitch_raw: 4,
        roll_raw: 5,
        forward_q31: [0, 0, i32::MAX],
    };
    let original_body = body;
    let mut runtime =
        SubFSwimmingRuntime::construct(descriptor(), &mut || 0xffff, &mut |_| Some(0));
    let original_runtime = runtime.clone();
    let mut bank = [17];
    let frame = SubFSwimmingFrame {
        terrain: &terrain,
        elapsed_micros: 20_000,
        active_model_extent_raw: 100,
    };
    let mut descriptor = descriptor();
    descriptor.animation_mode_raw = 3;
    assert_eq!(
        apply_sub_f_swimming_raw(descriptor, &mut runtime, &mut body, frame, &mut bank),
        Err(SubFSwimmingError::UnsupportedAnimationMode(3))
    );
    assert_eq!(runtime, original_runtime);
    assert_eq!(body, original_body);
    assert_eq!(bank, [17]);
    descriptor.animation_mode_raw = 0;
    assert_eq!(
        apply_sub_f_swimming_raw(descriptor, &mut runtime, &mut body, frame, &mut []),
        Err(SubFSwimmingError::VariableBindingOutsideBank(0))
    );
    assert_eq!(runtime, original_runtime);
    assert_eq!(body, original_body);
}

#[test]
fn follow_height_uses_strict_unwrapped_xz_range() {
    let terrain = TerrainGrid {
        header: [1000 * 256, 0, 0, 0, 0],
        cells: vec![
            TerrainCell {
                height: (-100_i8) as u8,
                attribute: 0,
                terrain_type: 0
            };
            65536
        ],
    };
    for (x, target_x, expected_vertical) in [(0, 0x4ff, 2), (0, 0x500, 0), (32760, -32760, 0)] {
        let mut runtime =
            SubFSwimmingRuntime::construct(descriptor(), &mut || 0xffff, &mut |_| None);
        runtime.clearance_raw = 2100;
        runtime.set_follow_target_mode_raw(1);
        runtime.target_position_raw = [target_x, -936, 0];
        let mut body = SubFSwimmingBody {
            state_flags: 1,
            position_raw: [x, -1000, 0],
            velocity_raw: [0; 3],
            pitch_raw: 0,
            roll_raw: 0,
            forward_q31: [0, 0, i32::MAX],
        };
        apply_sub_f_swimming_raw(
            descriptor(),
            &mut runtime,
            &mut body,
            SubFSwimmingFrame {
                terrain: &terrain,
                elapsed_micros: 0,
                active_model_extent_raw: 100,
            },
            &mut [],
        )
        .unwrap();
        assert_eq!(runtime.vertical_acceleration_raw, expected_vertical);
    }
}
