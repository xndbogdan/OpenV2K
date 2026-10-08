//! Byte-exact receipts for the software raster.
//!
//! `fixtures/software_raster/receipts.json` records what the unchanged retail
//! fill-slot handlers wrote for generated inputs (produced by private RE
//! tooling that executes the original instructions). Inputs are regenerated
//! here from the recorded seeds with the generator's `mix` hash, so the
//! fixture holds no retail data: only parameters, packets, and per case a
//! 64-bit FNV-1a digest of the output surface with its changed-pixel count.
//!
//! A failing digest names the case; the last passing revision of the port
//! reproduces its expected pixels for a diff. For porting work the private
//! generator can also write a fixture with every output surface (`--full`);
//! point `V2K_RASTER_RECEIPTS` at it to get per-pixel reports.

use std::io::Read;

use serde_json::Value;
use v2k_render::software::{
    ClipRect, FillSlot, MaterialView, PixelFormat, SoftwareRaster, Surface565, TexelTable,
    WordImageSource, ATLAS_STRIDE,
};

const FIXTURE: &str = include_str!("fixtures/software_raster/receipts.json");

/// 64-bit FNV-1a over the surface words' little-endian bytes.
fn surface_digest(words: &[u16]) -> u64 {
    let mut digest = 0xCBF2_9CE4_8422_2325u64;
    for byte in words.iter().flat_map(|word| word.to_le_bytes()) {
        digest = (digest ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01B3);
    }
    digest
}

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

fn base64_decode(text: &str) -> Vec<u8> {
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
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    for chunk in bytes.chunks(4) {
        let mut acc = 0u32;
        for (i, &c) in chunk.iter().enumerate() {
            acc |= value(c) << (18 - 6 * i);
        }
        let produced = chunk.len() * 6 / 8;
        out.extend_from_slice(&acc.to_be_bytes()[1..1 + produced]);
    }
    out
}

fn inflate_words(text: &str) -> Vec<u16> {
    let compressed = base64_decode(text);
    let mut decoder = flate2::read::ZlibDecoder::new(&compressed[..]);
    let mut bytes = Vec::new();
    decoder.read_to_end(&mut bytes).unwrap();
    bytes
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect()
}

fn hex_bytes(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect()
}

fn number(value: &Value, key: &str) -> u64 {
    value[key]
        .as_u64()
        .unwrap_or_else(|| panic!("missing number {key}"))
}

struct GeneratedMaterial {
    flags: u16,
    shade_count: u16,
    width: u16,
    height: u16,
    atlas: Vec<u8>,
    palette: Vec<u16>,
}

fn generate_material(spec: &Value, origin: usize, palette_entries: usize) -> GeneratedMaterial {
    let flags = number(spec, "flags") as u16;
    let width = number(spec, "width") as u16;
    let height = number(spec, "height") as u16;
    let seed = number(spec, "seed") as u32;
    let range = number(spec, "texel_range") as u32;
    let raw = flags & 2 != 0;
    let mut atlas = vec![0u8; origin + (usize::from(height) + 8) * ATLAS_STRIDE];
    for y in 0..u32::from(height) {
        for x in 0..u32::from(width) {
            let value = mix(seed, y * u32::from(width) + x);
            let row = origin + y as usize * ATLAS_STRIDE;
            if raw {
                let low = value & 0x7FFF;
                let mut word = (((low & 0x7FE0) << 1) | (low & 0x1F)) as u16;
                if (value >> 20) & 7 == 0 {
                    word = 0;
                }
                let at = row + 2 * x as usize;
                atlas[at..at + 2].copy_from_slice(&word.to_le_bytes());
            } else {
                atlas[row + x as usize] = ((value >> 8) % range) as u8;
            }
        }
    }
    let palette = if raw {
        Vec::new()
    } else {
        (0..palette_entries as u32)
            .map(|i| (mix(seed ^ 0xA5A5, i) & 0xFFDF) as u16)
            .collect()
    };
    GeneratedMaterial {
        flags,
        shade_count: number(spec, "shade_count") as u16,
        width,
        height,
        atlas,
        palette,
    }
}

/// Word-image elements as the generator builds them.
fn image_dwords(seed: u32, count: u32) -> Vec<u32> {
    (0..count)
        .map(|i| {
            let value = mix(seed ^ 0x5151, i);
            match value >> 28 {
                0 => 0,
                1 => value & 0xFFFF_0000,
                _ => value & 0xFFFF,
            }
        })
        .collect()
}

struct OneImage(Vec<u32>);

impl WordImageSource for OneImage {
    fn word_image(&self, _id: u32) -> &[u32] {
        &self.0
    }
}

#[derive(Default)]
struct Outcome {
    passed: usize,
    failures: Vec<String>,
    /// `(raw table, row)` of every span row the polygon receipts bound.
    rows: std::collections::BTreeSet<(bool, usize)>,
}

fn run_case(case: &Value, origin: usize, palette_entries: usize, outcome: &mut Outcome) {
    let name = case["name"].as_str().unwrap();
    let slot_offset = number(case, "slot") as u16;
    let Some(slot) = FillSlot::from_offset(slot_offset) else {
        outcome
            .failures
            .push(format!("{name}: slot +{slot_offset:04X} is not ported"));
        return;
    };
    let surface_spec = &case["surface"];
    let width = number(surface_spec, "width") as u32;
    let height = number(surface_spec, "height") as u32;
    let pitch = number(surface_spec, "pitch") as usize;
    let seed = number(surface_spec, "seed") as u32;
    let mut surface = Surface565::with_pitch(width, height, pitch);
    for (index, pixel) in surface.pixels.iter_mut().enumerate() {
        *pixel = (mix(seed, index as u32) & 0xFFFF) as u16;
    }
    let initial = surface.pixels.clone();
    let clip: Vec<i16> = case["clip"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_i64().unwrap() as i16)
        .collect();
    let generated: Vec<GeneratedMaterial> = case["materials"]
        .as_array()
        .unwrap()
        .iter()
        .map(|spec| generate_material(spec, origin, palette_entries))
        .collect();
    let materials: Vec<MaterialView<'_>> = generated
        .iter()
        .map(|m| MaterialView {
            flags: m.flags,
            shade_count: m.shade_count,
            width: m.width,
            height: m.height,
            atlas: &m.atlas,
            origin,
            palette: &m.palette,
        })
        .collect();
    let packet = hex_bytes(case["packet"].as_str().unwrap());
    let mut raster = SoftwareRaster::new(
        ClipRect {
            x0: clip[0],
            y0: clip[1],
            x1: clip[2],
            y1: clip[3],
        },
        PixelFormat::RGB565,
        number(case, "handler_esp") as u32,
    );
    if let Some(colour) = case["fog_colour"].as_u64() {
        raster.fog_mut().select(PixelFormat::RGB565, colour as u32);
    }
    let images = OneImage(match case.get("image") {
        Some(image) => image_dwords(number(image, "seed") as u32, number(image, "count") as u32),
        None => Vec::new(),
    });
    raster.draw(slot, &packet, &mut surface, &materials, &images);
    if slot.scan_depth().is_some() {
        let row = raster.span_row();
        outcome
            .rows
            .insert((row.table == TexelTable::Raw, row.index));
    }
    if let (Some(depth), Some(frame)) = (slot.scan_depth(), case["scan_frames"][0].as_u64()) {
        let handler_esp = number(case, "handler_esp") as u32;
        assert_eq!(
            handler_esp.wrapping_sub(depth),
            frame as u32,
            "{name}: scan converter frame"
        );
    }

    let changed = surface
        .pixels
        .iter()
        .zip(initial.iter())
        .filter(|(after, before)| after != before)
        .count();
    let want_changed = number(case, "changed_pixels") as usize;
    let digest = surface_digest(&surface.pixels);
    let want_digest = u64::from_str_radix(case["expected_digest"].as_str().unwrap(), 16).unwrap();
    let mut mismatches = 0usize;
    let mut first = None;
    if let Some(xor) = case.get("expected_xor").and_then(Value::as_str) {
        let expected_xor = inflate_words(xor);
        assert_eq!(expected_xor.len(), initial.len(), "{name}: fixture size");
        let row_words = pitch / 2;
        for (index, (&got, (&xor, &before))) in surface
            .pixels
            .iter()
            .zip(expected_xor.iter().zip(initial.iter()))
            .enumerate()
        {
            let want = xor ^ before;
            if got != want {
                mismatches += 1;
                if first.is_none() {
                    first = Some((index % row_words, index / row_words, got, want, before));
                }
            }
        }
    }
    let dither = raster.dither_word();
    let want_dither = number(case, "expected_dither") as u16;
    let clip_now = raster.clip();
    let want_clip: Vec<i64> = case["expected_clip"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_i64().unwrap())
        .collect();
    let clip_ok = want_clip
        == [
            i64::from(clip_now.x0),
            i64::from(clip_now.y0),
            i64::from(clip_now.x1),
            i64::from(clip_now.y1),
        ];
    if digest == want_digest && dither == want_dither && clip_ok {
        outcome.passed += 1;
    } else {
        let mut report = format!("{name}:");
        if digest != want_digest {
            report += &format!(" surface digest {digest:016x} want {want_digest:016x}, {changed} pixels changed want {want_changed}");
        }
        if mismatches != 0 {
            report += &format!(", {mismatches} pixel mismatches");
        }
        if let Some((x, y, got, want, before)) = first {
            report +=
                &format!(", first at ({x},{y}) got {got:04X} want {want:04X} (was {before:04X})");
        }
        if dither != want_dither {
            report += &format!(", dither {dither:04X} want {want_dither:04X}");
        }
        if !clip_ok {
            report += &format!(", clip {clip_now:?} want {want_clip:?}");
        }
        outcome.failures.push(report);
    }
}

#[test]
fn retail_fill_slots_match_their_native_receipts() {
    let text = match std::env::var_os("V2K_RASTER_RECEIPTS") {
        Some(path) => std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{}: {error}", path.to_string_lossy())),
        None => FIXTURE.to_owned(),
    };
    let fixture: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(fixture["format"], "v2k-software-raster-receipts/4");
    assert_eq!(
        fixture["pixel_format"],
        serde_json::json!([7, 2, 4]),
        "receipts assume the retail RGB565 channel words"
    );
    let origin = number(&fixture, "atlas_origin") as usize;
    let palette_entries = number(&fixture, "palette_entries") as usize;
    let mut outcome = Outcome::default();
    for case in fixture["cases"].as_array().unwrap() {
        run_case(case, origin, palette_entries, &mut outcome);
    }
    assert!(
        outcome.failures.is_empty(),
        "{} receipts passed, {} failed:\n{}",
        outcome.passed,
        outcome.failures.len(),
        outcome.failures.join("\n")
    );
    // Untextured handlers bind raw rows 0..=2 and 12..=14; textured rows of
    // both tables are 4..=11, 16..=23 and 28..=35. No handler binds rows 3,
    // 15 or 24..=27.
    let mut bound: std::collections::BTreeSet<(bool, usize)> = [0, 1, 2, 12, 13, 14]
        .into_iter()
        .map(|row| (true, row))
        .collect();
    for row in (4..12).chain(16..24).chain(28..36) {
        bound.insert((false, row));
        bound.insert((true, row));
    }
    assert_eq!(outcome.rows, bound, "span rows the receipts bound");
}
