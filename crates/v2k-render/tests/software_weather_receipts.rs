//! Byte-exact receipts for the weather's terrain-light writers.
//!
//! `fixtures/software_raster/weather_light.json` records the 32x32 light
//! window after the unchanged retail shadow pattern (`FUN_0040FA20`), light
//! pattern (`FUN_0040FD90`) or lightning flash (`FUN_0040FCB0`) ran on a
//! generated window, origin and weather clock (produced by private RE tooling
//! that executes the original instructions). The test regenerates each input
//! from its seed and compares every cell.

use serde_json::Value;
use v2k_render::TerrainLightWindow;

const FIXTURE: &str = include_str!("fixtures/software_raster/weather_light.json");

fn mix(seed: u32, index: u32) -> u32 {
    let mut x = seed
        .wrapping_mul(0x9E37_79B9)
        .wrapping_add(index.wrapping_mul(0x85EB_CA6B))
        .wrapping_add(0x632B_E5AB);
    x ^= x >> 16;
    x = x.wrapping_mul(0x7FEB_352D);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846C_A68B);
    x ^= x >> 16;
    x
}

#[test]
fn weather_light_writers_match_retail_receipts() {
    let fixture: Value = serde_json::from_str(FIXTURE).unwrap();
    assert_eq!(fixture["format"], "v2k-weather-light-receipts/1");
    let cases = fixture["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 18);
    for case in cases {
        let seed = case["seed"].as_u64().unwrap() as u32;
        let origin = [
            case["origin"][0].as_u64().unwrap() as u8,
            case["origin"][1].as_u64().unwrap() as u8,
        ];
        let clock = case["clock"].as_u64().unwrap() as u32;
        let values = std::array::from_fn(|i| mix(seed ^ 0x5A, i as u32) as u8 as i8);
        let mut window = TerrainLightWindow::from_window(values, origin);
        let writer = case["writer"].as_str().unwrap();
        match writer {
            "shadow" => window.add_weather_shadow_pattern(clock),
            "light" => window.add_weather_light_pattern(clock),
            "flash" => window.add_flash(),
            other => panic!("unknown writer {other}"),
        }
        let expected = case["window"].as_str().unwrap();
        let actual: String = window
            .window()
            .0
            .iter()
            .map(|&cell| format!("{:02x}", cell as u8))
            .collect();
        assert_eq!(
            actual, expected,
            "{writer} seed {seed} origin {origin:?} clock {clock}"
        );
    }
}
