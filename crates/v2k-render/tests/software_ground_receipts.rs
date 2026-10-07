//! Byte-exact receipts for the software ground producer.
//!
//! `fixtures/software_raster/ground.json` records the primitive-queue arena
//! the unchanged retail opaque-terrain pass `FUN_0042F980` filled for
//! generated terrain, light, infection and camera inputs (produced by private
//! RE tooling that executes the original instructions). The test regenerates
//! the inputs from the recorded seeds, runs the port into an arena at the same
//! address and compares a 64-bit FNV-1a digest of the allocated bytes, so
//! keys, links, record layouts and allocation order are all covered. Set
//! `V2K_GROUND_RECEIPTS` to a `--full` fixture for per-byte reports.

use std::io::Read;

use serde_json::Value;
use v2k_formats::terrain::{TerrainCell, TerrainGrid};
use v2k_render::software::terrain::{
    draw_ground, ground_lead, scan_counts, GroundMaterial, GroundProjection, GroundScene,
};
use v2k_render::software::PrimitiveQueue;
use v2k_render::terrain_tiles::build_lookup;

const FIXTURE: &str = include_str!("fixtures/software_raster/ground.json");

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

/// The generator's Section-10 cells.
fn terrain(seed: u32, infection: u32, darkness: [i16; 2]) -> TerrainGrid {
    let mut cells = Vec::with_capacity(256 * 256);
    for x in 0..256u32 {
        for z in 0..256u32 {
            let value = mix(seed, x * 256 + z);
            let coarse = mix(seed ^ 0x51, (x >> 3) * 64 + (z >> 3));
            let height = ((coarse & 0x3F) as i32 - 0x20) + ((value >> 8) & 7) as i32 - 3;
            let material = (value >> 12) % 5;
            let shade = (value >> 16) & 7;
            let infected = infection != 0 && ((value >> 20) & 7) < infection;
            let kind = material | (shade << 5) | if infected { 0x10 } else { 0 };
            cells.push(TerrainCell {
                height: height as u8,
                attribute: (value >> 24) as u8,
                terrain_type: kind as u8,
            });
        }
    }
    let darkness_word = (u32::from(darkness[1] as u16) << 16) | u32::from(darkness[0] as u16);
    TerrainGrid {
        header: [0, 0, 0, 0, darkness_word as i32],
        cells,
    }
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
    let infection = case["style"]
        .get("infection")
        .and_then(Value::as_i64)
        .unwrap_or(0) as u32;
    let darkness = numbers(&case["darkness"]);
    let grid = terrain(seed, infection, [darkness[0] as i16, darkness[1] as i16]);
    let light: [i8; 1024] =
        std::array::from_fn(|i| ((mix(seed ^ 0x77, i as u32) % 7) as i32 - 3) as i8);
    let selectors: [u8; 256] = std::array::from_fn(|i| mix(seed ^ 0x99, i as u32) as u8);
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
    let projection = GroundProjection {
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
    };
    let lead = ground_lead(axes[2][1]);
    if i64::from(lead) != number(case, "lead") {
        return Err(format!("{name}: lead {lead} want {}", number(case, "lead")));
    }
    let scan = numbers(&case["scan"]);
    let (rows, points) = scan_counts(scan[0] as i32, scan[1] as i32);
    let eye = numbers(&case["eye"]);
    let shade_words = numbers(&case["shade_words"]);
    let offsets = numbers(&case["infection_offsets"]);
    let light_origin = numbers(&case["light_origin"]);
    let record_base = number(sprites, "base") as u32;
    let record_stride = number(sprites, "stride") as u32;
    let sprite = |index: u32| {
        let (width, height) = sprite_size(seed, index);
        GroundMaterial {
            id: record_base + index * record_stride,
            width,
            height,
        }
    };
    let tiles = build_lookup();
    let scene = GroundScene {
        grid: &grid,
        projection,
        eye: std::array::from_fn(|i| eye[i] as i16),
        lead,
        screen_height: numbers(&case["screen"])[1] as i16,
        shade_words: std::array::from_fn(|i| shade_words[i] as u32),
        fog_colour: number(case, "fog_colour") as u32,
        cap_colour: number(case, "cap_colour") as u32,
        rows,
        points,
        light: &light,
        light_origin: [light_origin[0] as u8, light_origin[1] as u8],
        infection_offsets: std::array::from_fn(|i| offsets[i] as i8),
        infection_selectors: &selectors,
        tiles: &tiles,
        tile_base: number(case, "tile_base_word") as u32,
        infection_base: number(case, "infection_base") as u32,
        sprite: &sprite,
    };
    let mut queue = PrimitiveQueue::with_base(
        number(case, "arena_bytes") as usize,
        number(case, "arena_base") as u32,
    )
    .unwrap();
    draw_ground(&mut queue, &scene).map_err(|error| format!("{name}: {error:?}"))?;
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
        if let Some(at) = allocated.iter().zip(&want).position(|(a, b)| a != b) {
            let from = at.saturating_sub(16) & !3;
            report += &format!(
                "; first difference at arena +0x{:X}: got {:02X?} want {:02X?}",
                at + 0x18,
                &allocated[from..(from + 48).min(allocated.len())],
                &want[from..(from + 48).min(want.len())]
            );
        }
    }
    Err(report)
}

#[test]
fn retail_ground_scan_matches_its_native_receipts() {
    let text = match std::env::var_os("V2K_GROUND_RECEIPTS") {
        Some(path) => std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{}: {error}", path.to_string_lossy())),
        None => FIXTURE.to_owned(),
    };
    let fixture: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(fixture["format"], "v2k-software-ground-receipts/1");
    let failures: Vec<String> = fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|case| run_case(case, &fixture["sprite_records"]).err())
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
