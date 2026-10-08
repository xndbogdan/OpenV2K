//! Exact receipts for the software type-1 vertex projector.
//!
//! `fixtures/software_raster/model_midpoints.json` records the complete cache
//! entry the retail projector (`FUN_0046DC00`) wrote from two generated,
//! already projected source entries (produced by private RE tooling that
//! executes the original instructions). Fields retail leaves alone keep the
//! fixture's sentinel.

use serde_json::Value;
use v2k_render::software::model::{screen_midpoint, ModelCorner};

const FIXTURE: &str = include_str!("fixtures/software_raster/model_midpoints.json");

fn corner(value: &Value) -> ModelCorner {
    let words = |key: &str| -> Vec<i64> {
        value[key]
            .as_array()
            .expect("array")
            .iter()
            .map(|word| word.as_i64().expect("integer"))
            .collect()
    };
    let view = words("view");
    let screen = words("screen");
    ModelCorner {
        view: [view[0] as i32, view[1] as i32, view[2] as i32],
        screen: [screen[0] as i16, screen[1] as i16],
        clip: value["clip"].as_u64().expect("clip") as u8,
        fade: value["fade"].as_u64().expect("fade") as u8,
    }
}

#[test]
fn type1_projector_matches_native_receipts() {
    let fixture: Value = serde_json::from_str(FIXTURE).expect("fixture JSON");
    assert_eq!(
        fixture["format"], "v2k-software-model-midpoint-receipts/1",
        "fixture format"
    );
    let sentinel = corner(&fixture["sentinel"]);
    let cases = fixture["cases"].as_array().expect("cases");
    assert!(cases.len() >= 50, "receipt coverage shrank");
    let mut rejected = 0;
    for case in cases {
        let name = case["name"].as_str().expect("name");
        let bounds = case["bounds"].as_array().expect("bounds");
        let bounds = [0, 1].map(|axis| bounds[axis].as_u64().expect("bound") as u32);
        let actual = screen_midpoint(corner(&case["a"]), corner(&case["b"]), sentinel, bounds);
        let expected = corner(&case["out"]);
        assert_eq!(actual, expected, "{name}");
        rejected += usize::from(expected.clip == 0x40);
    }
    assert!(rejected >= 9, "near-rejection receipts missing");
}
