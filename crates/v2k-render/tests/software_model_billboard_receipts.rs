//! Byte-exact receipts for the software model billboard constructors.
//!
//! `fixtures/software_raster/model_billboards.json` records what the retail
//! near and fog billboard constructors (`0x68` flat, `0x78` sprite) queued
//! for generated vertex caches, palette dwords, sprite sizes, lens words and
//! decoded size/angle operands (produced by private RE tooling that executes
//! the original instructions). Set `V2K_MODEL_BILLBOARD_RECEIPTS` to a
//! `--full` fixture for per-byte reports.

use std::io::Read;

use serde_json::Value;
use v2k_render::software::model::{
    construct_billboard, BillboardCommand, FaceContext, FacePass, ModelCorner,
};
use v2k_render::software::terrain::GroundProjection;
use v2k_render::software::PrimitiveQueue;

const FIXTURE: &str = include_str!("fixtures/software_raster/model_billboards.json");

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

fn digest(bytes: &[u8]) -> u64 {
    let mut digest = 0xCBF2_9CE4_8422_2325u64;
    for &byte in bytes {
        digest = (digest ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01B3);
    }
    digest
}

fn outcode(x: i32, y: i32) -> u8 {
    let h = if (0..640).contains(&x) {
        2
    } else if x >= 0 {
        4
    } else {
        1
    };
    let v = if (0..480).contains(&y) {
        0x10
    } else if y >= 0 {
        0x20
    } else {
        8
    };
    h | v
}

/// The generator's warm vertex cache entry.
fn corner(seed: u32, index: u32) -> ModelCorner {
    let [a, b, c, d] = [0x101, 0x202, 0x303, 0x404].map(|key| mix(seed ^ key, index));
    let x = (a & 0x3FF) as i32 - 0x100;
    let y = (b & 0x3FF) as i32 - 0x140;
    let view = [
        (c & 0xFFFF) as i32 - 0x8000,
        ((c >> 16) & 0xFFFF) as i32 - 0x8000,
        0x40 + (d & 0x1FFF) as i32,
    ];
    let mut clip = outcode(x, y);
    if (d >> 13) & 7 == 0 {
        clip = 0x40;
    }
    if (d >> 16) & 1 != 0 {
        clip |= 0x80;
    }
    let fade = if (d >> 28) & 3 == 0 {
        0xFF
    } else {
        ((d >> 20) & 0xFF) as u8
    };
    ModelCorner {
        view,
        screen: [x as i16, y as i16],
        clip,
        fade,
    }
}

fn number(value: &Value, key: &str) -> i64 {
    value[key]
        .as_i64()
        .unwrap_or_else(|| panic!("missing number {key}"))
}

fn inflate(text: &str) -> Vec<u8> {
    fn value(c: u8) -> u32 {
        match c {
            b'A'..=b'Z' => u32::from(c - b'A'),
            b'a'..=b'z' => u32::from(c - b'a') + 26,
            b'0'..=b'9' => u32::from(c - b'0') + 52,
            b'+' => 62,
            b'/' => 63,
            _ => panic!("invalid base64 byte {c}"),
        }
    }
    let bytes: Vec<u8> = text.bytes().filter(|&c| c != b'=').collect();
    let mut compressed = Vec::new();
    for chunk in bytes.chunks(4) {
        let mut acc = 0u32;
        for (i, &c) in chunk.iter().enumerate() {
            acc |= value(c) << (18 - 6 * i);
        }
        compressed.extend_from_slice(&acc.to_be_bytes()[1..1 + chunk.len() * 6 / 8]);
    }
    let mut out = Vec::new();
    flate2::read::ZlibDecoder::new(&compressed[..])
        .read_to_end(&mut out)
        .unwrap();
    out
}

fn sprite_size(seed: u32, index: u32) -> (u16, u16) {
    let value = mix(seed ^ 0x808, index);
    (4 + (value & 0x3F) as u16, 4 + ((value >> 8) & 0x3F) as u16)
}

fn pair(value: &Value) -> [i32; 2] {
    let values: Vec<i64> = value
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_i64().unwrap())
        .collect();
    [values[0] as i32, values[1] as i32]
}

fn run_case(case: &Value, fixture: &Value) -> Result<(), String> {
    let name = case["name"].as_str().unwrap();
    let seed = number(case, "seed") as u32;
    let corners: Vec<ModelCorner> = (0..number(fixture, "vertices") as u32)
        .map(|index| corner(seed, index))
        .collect();
    let palette = |mat: i16| mix(seed ^ 0x707, (mat as u32) & 0xFF);
    let sprites = &fixture["sprite_records"];
    let (base, stride) = (
        number(sprites, "base") as u32,
        number(sprites, "stride") as u32,
    );
    let sprite = |mat: i16| base + ((mat as u32) & 0xFF) * stride;
    let size = |mat: i16| sprite_size(seed, (mat as u32) & 0xFF);
    let context = FaceContext {
        corners: &corners,
        normals: &[],
        palette: &palette,
        sprite: &sprite,
        fog_colour: number(case, "fog_colour") as u32,
    };
    let lens_words = &fixture["lens"];
    let bounds = pair(&lens_words["bounds"]);
    let lens = GroundProjection {
        axes_q31: [[0; 3]; 3],
        translation: [0; 3],
        focal: pair(&lens_words["focal"]),
        bounds: [bounds[0] as u32, bounds[1] as u32],
        centre: pair(&lens_words["centre"]),
        fade: [0, i32::MAX, i32::MAX],
        wet_clock: None,
    };
    let mut queue = PrimitiveQueue::with_base(
        number(fixture, "arena_bytes") as usize,
        number(fixture, "arena_base") as u32,
    )
    .unwrap();
    if case["fifo"].as_bool().unwrap() {
        queue.set_scope_sorted(false);
    }
    let pass = if case["fog"].as_bool().unwrap() {
        FacePass::Fog
    } else {
        FacePass::Near
    };
    let command = BillboardCommand {
        vertex: number(case, "vertex") as usize,
        id: number(case, "id") as i16,
        size: number(case, "size") as i32,
        angle: number(case, "angle") as u16,
        textured: number(case, "opcode") == 0x78,
    };
    construct_billboard(&mut queue, &context, &lens, &size, pass, command)
        .map_err(|error| format!("{name}: {error:?}"))?;
    let (_, allocated) = queue.allocated();
    let want_len = number(case, "allocated_bytes") as usize;
    let want_digest = u64::from_str_radix(case["arena_digest"].as_str().unwrap(), 16).unwrap();
    if allocated.len() == want_len && digest(allocated) == want_digest {
        return Ok(());
    }
    let mut report = format!(
        "{name}: allocated {} bytes want {want_len}, digest {:016x} want {want_digest:016x}",
        allocated.len(),
        digest(allocated)
    );
    if let Some(full) = case.get("arena").and_then(Value::as_str) {
        report += &format!("; got {:02X?}; want {:02X?}", allocated, inflate(full));
    }
    Err(report)
}

#[test]
fn retail_billboard_constructors_match_their_native_receipts() {
    let text = match std::env::var_os("V2K_MODEL_BILLBOARD_RECEIPTS") {
        Some(path) => std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{}: {error}", path.to_string_lossy())),
        None => FIXTURE.to_owned(),
    };
    let fixture: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(fixture["format"], "v2k-software-model-billboard-receipts/1");
    let cases = fixture["cases"].as_array().unwrap();
    let failures: Vec<String> = cases
        .iter()
        .filter_map(|case| run_case(case, &fixture).err())
        .collect();
    assert!(
        failures.is_empty(),
        "{} of {} receipts failed:
{}",
        failures.len(),
        cases.len(),
        failures.join(
            "
"
        )
    );
}
