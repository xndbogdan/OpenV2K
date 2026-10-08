//! Byte-exact receipts for the software particle producer.
//!
//! `fixtures/software_raster/particles.json` records what the unchanged
//! retail particle drawer `FUN_0043D410` queued for generated particles,
//! class descriptors, frames, sprite records, projection words and fog
//! planes (produced by private RE tooling that executes the original
//! instructions). The test rebuilds the drawer's inputs, including frame
//! selection and the record-address size jitter, and compares a digest of
//! the queued bytes. Set `V2K_PARTICLE_RECEIPTS` to a `--full` fixture for
//! per-byte reports.

use std::io::Read;

use serde_json::Value;
use v2k_render::software::particle::{queue_particle, Particle, ParticleScene, ParticleShadow};
use v2k_render::software::terrain::{GroundMaterial, GroundProjection};
use v2k_render::software::PrimitiveQueue;

const FIXTURE: &str = include_str!("fixtures/software_raster/particles.json");

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

fn number(value: &Value, key: &str) -> i64 {
    value[key]
        .as_i64()
        .unwrap_or_else(|| panic!("missing number {key}"))
}

fn numbers(value: &Value) -> Vec<i64> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_i64().unwrap())
        .collect()
}

fn sprite_size(seed: u32, index: u32) -> (u16, u16) {
    let value = mix(seed ^ 0x33, index);
    (8 + (value & 0x1F) as u16, 8 + ((value >> 8) & 0x1F) as u16)
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

fn run_case(case: &Value, sprites: &Value) -> Result<(), String> {
    let name = case["name"].as_str().unwrap();
    let seed = number(case, "seed") as u32;
    let axes = case["axes"].as_array().unwrap();
    let axes: [[i32; 3]; 3] = std::array::from_fn(|row| {
        let row = numbers(&axes[row]);
        std::array::from_fn(|column| row[column] as i32)
    });
    let pair = |key: &str| {
        let values = numbers(&case[key]);
        [values[0] as i32, values[1] as i32]
    };
    let translation = numbers(&case["translation"]);
    let fade = numbers(&case["fade"]);
    let bounds = pair("bounds");
    let scene = ParticleScene {
        projection: GroundProjection {
            axes_q31: axes,
            translation: std::array::from_fn(|i| translation[i] as i32),
            focal: pair("focal"),
            bounds: [bounds[0] as u32, bounds[1] as u32],
            centre: pair("centre"),
            fade: std::array::from_fn(|i| fade[i] as i32),
            wet_clock: case["wet"]
                .as_bool()
                .unwrap_or(false)
                .then(|| number(case, "clock") as u32),
        },
        eye: {
            let eye = numbers(&case["eye"]);
            std::array::from_fn(|i| eye[i] as i16)
        },
        fog_near: number(case, "fog_near") as i32,
        far: number(case, "far") as i32,
        fog_colour: number(case, "fog_colour") as u32,
    };
    // Frame selection: ((rate * age) >> 6) % count.
    let frames: Vec<[i64; 2]> = case["frames"]
        .as_array()
        .unwrap()
        .iter()
        .map(|frame| {
            let frame = numbers(frame);
            [frame[0], frame[1]]
        })
        .collect();
    let index =
        ((number(case, "rate") as u32 * number(case, "age") as u32) >> 6) as usize % frames.len();
    let [sprite, frame_size] = frames[index];
    let frame_size = frame_size as u16;
    // Size jitter by the record address: an unsigned division.
    let mut draw = number(case, "draw_scale") as i32;
    let jitter = number(case, "jitter") as i8;
    if jitter != 0 {
        let phase = (number(case, "particle_address") as u32 >> 5) & 0xF;
        draw = draw
            .wrapping_add((phase.wrapping_mul(draw as u32) / (i32::from(jitter) as u32)) as i32);
    }
    let (width, height) = sprite_size(seed, sprite as u32);
    let record_base = number(sprites, "base") as u32;
    let record_stride = number(sprites, "stride") as u32;
    let position = numbers(&case["position"]);
    let shadow_size = number(case, "shadow_size") as u8;
    let particle = Particle {
        position: std::array::from_fn(|i| position[i] as u16 as i16),
        scale: draw.wrapping_mul(i32::from(frame_size as i16)),
        sprite: GroundMaterial {
            id: record_base + sprite as u32 * record_stride,
            width,
            height,
        },
        flags: number(case, "flags") as u8,
        sort_bias: number(case, "sort_bias") as i16,
        frame_size,
        shadow: (shadow_size != 0).then(|| ParticleShadow {
            size: shadow_size,
            ground: number(case, "ground") as u16 as i16,
            colour: number(case, "shadow_colour") as u32,
        }),
    };
    let mut queue = PrimitiveQueue::with_base(
        number(case, "arena_bytes") as usize,
        number(case, "arena_base") as u32,
    )
    .unwrap();
    if case["fifo"].as_bool().unwrap() {
        queue.set_scope_sorted(false);
    }
    queue_particle(&mut queue, &scene, &particle).map_err(|error| format!("{name}: {error:?}"))?;
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
fn retail_particle_drawer_matches_its_native_receipts() {
    let text = match std::env::var_os("V2K_PARTICLE_RECEIPTS") {
        Some(path) => std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{}: {error}", path.to_string_lossy())),
        None => FIXTURE.to_owned(),
    };
    let fixture: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(fixture["format"], "v2k-software-particle-receipts/1");
    let cases = fixture["cases"].as_array().unwrap();
    let drawn = cases
        .iter()
        .filter(|case| number(case, "allocated_bytes") > 0)
        .count();
    assert!(drawn >= 60, "too few drawn particles in the receipts");
    let failures: Vec<String> = cases
        .iter()
        .filter_map(|case| run_case(case, &fixture["sprite_records"]).err())
        .collect();
    assert!(
        failures.is_empty(),
        "{}",
        failures.join(
            "
"
        )
    );
}
