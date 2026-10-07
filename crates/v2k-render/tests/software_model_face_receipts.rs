//! Byte-exact receipts for the software model face constructors.
//!
//! `fixtures/software_raster/model_faces.json` records what every face
//! opcode of the retail near and fog constructor tables queued for generated
//! vertex/normal caches, palette dwords and command words (produced by
//! private RE tooling that executes the original instructions). The test
//! regenerates the caches from the recorded seeds, runs the port into an
//! arena at the same address and compares a digest of the allocated bytes.
//! Set `V2K_MODEL_FACE_RECEIPTS` to a `--full` fixture for per-byte reports.

use std::io::Read;

use serde_json::Value;
use v2k_render::software::model::{
    construct_face, FaceContext, FacePass, ModelCorner, ModelNormal,
};
use v2k_render::software::PrimitiveQueue;

const FIXTURE: &str = include_str!("fixtures/software_raster/model_faces.json");

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

fn normal(seed: u32, index: u32) -> ModelNormal {
    ModelNormal {
        shade: mix(seed ^ 0x606, index),
        culled: mix(seed ^ 0x505, index) & 7 == 0,
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

fn run_case(case: &Value, fixture: &Value) -> Result<(), String> {
    let name = case["name"].as_str().unwrap();
    let seed = number(case, "seed") as u32;
    let corners: Vec<ModelCorner> = (0..number(fixture, "vertices") as u32)
        .map(|index| corner(seed, index))
        .collect();
    let normals: Vec<ModelNormal> = (0..number(fixture, "normals") as u32)
        .map(|index| normal(seed, index))
        .collect();
    let palette = |mat: i16| mix(seed ^ 0x707, (mat as u32) & 0xFF);
    let sprites = &fixture["sprite_records"];
    let (base, stride) = (
        number(sprites, "base") as u32,
        number(sprites, "stride") as u32,
    );
    let sprite = |mat: i16| base + ((mat as u32) & 0xFF) * stride;
    let context = FaceContext {
        corners: &corners,
        normals: &normals,
        palette: &palette,
        sprite: &sprite,
        fog_colour: number(case, "fog_colour") as u32,
    };
    let mut queue = PrimitiveQueue::with_base(
        number(fixture, "arena_bytes") as usize,
        number(fixture, "arena_base") as u32,
    )
    .unwrap();
    if case["fifo"].as_bool().unwrap() {
        queue.set_scope_sorted(false);
    }
    let words: Vec<i16> = case["words"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_i64().unwrap() as i16)
        .collect();
    let pass = if case["fog"].as_bool().unwrap() {
        FacePass::Fog
    } else {
        FacePass::Near
    };
    let consumed = construct_face(
        &mut queue,
        &context,
        pass,
        number(case, "opcode") as u8,
        &words,
    )
    .map_err(|error| format!("{name}: {error:?}"))?;
    if consumed != Some(number(case, "consumed_words") as usize) {
        return Err(format!(
            "{name}: consumed {consumed:?} want {}",
            number(case, "consumed_words")
        ));
    }
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
        let want = inflate(full);
        report += &format!("; got {:02X?}; want {:02X?}", allocated, want);
    }
    Err(report)
}

#[test]
fn retail_face_constructors_match_their_native_receipts() {
    let text = match std::env::var_os("V2K_MODEL_FACE_RECEIPTS") {
        Some(path) => std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{}: {error}", path.to_string_lossy())),
        None => FIXTURE.to_owned(),
    };
    let fixture: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(fixture["format"], "v2k-software-model-face-receipts/1");
    let cases = fixture["cases"].as_array().unwrap();
    let failures: Vec<String> = cases
        .iter()
        .filter_map(|case| run_case(case, &fixture).err())
        .collect();
    assert!(
        failures.is_empty(),
        "{} of {} receipts failed:\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n")
    );
}
