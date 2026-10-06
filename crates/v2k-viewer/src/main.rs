//! V2000 asset workbench and command-line exporter.
//!
//! Usage:
//!   cargo run -p v2k-viewer
//!   cargo run -p v2k-viewer -- --mode models --model hovercraft
//!   cargo run -p v2k-viewer -- Overlay/1X3XX.OVL --mode sprites
//!   cargo run -p v2k-viewer -- Overlay/0X2XX.OVL --mode audio -o audio_out

mod workbench;

use clap::{Parser, ValueEnum};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use v2k_formats::palette::{BRIGHTEST_SHADE, SHADE_LEVELS};
use v2k_formats::sprites::DecodedSprite;

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum ViewerMode {
    Sprites,
    Models,
    Audio,
    Saves,
    Terrain,
}

#[derive(Parser)]
#[command(name = "v2k-viewer", about = "V2000 asset debugging workbench")]
struct Cli {
    /// Optional OVL to place at the top of the GUI cache and open first.
    ovl_path: Option<PathBuf>,

    /// V2000 directory containing Overlay/.
    #[arg(long, short = 'd', default_value = ".")]
    data_dir: PathBuf,

    /// Viewer mode
    #[arg(long, value_enum, default_value_t = ViewerMode::Models)]
    mode: ViewerMode,

    /// Sprite shade row, from 0 (darkest) through the brightest authored row.
    #[arg(long, default_value_t = BRIGHTEST_SHADE, value_parser = parse_shade)]
    shade: usize,

    /// Output directory for exported files (sprites/models/audio/terrain mode)
    #[arg(long, short)]
    output: Option<PathBuf>,

    /// Find this model name or canonical global id when the workbench opens.
    #[arg(long)]
    model: Option<String>,

    /// System-graphics OVL variant (0=320x240 low, 1=640x480 high, 2=800x600,
    /// 3=1024x768). Omit to infer it from an explicit OVL, or default to 1.
    #[arg(long, value_parser = clap::value_parser!(u32).range(0..=3))]
    variant: Option<u32>,
}

struct FileCli {
    ovl_path: PathBuf,
    mode: ViewerMode,
    shade: usize,
    output: Option<PathBuf>,
}

fn parse_shade(value: &str) -> Result<usize, String> {
    let shade = value
        .parse::<usize>()
        .map_err(|_| format!("invalid shade level {value:?}"))?;
    if shade < SHADE_LEVELS {
        Ok(shade)
    } else {
        Err(format!(
            "shade level must be between 0 and {BRIGHTEST_SHADE}"
        ))
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let interactive = cli.output.is_none()
        && matches!(
            cli.mode,
            ViewerMode::Sprites | ViewerMode::Models | ViewerMode::Terrain
        );
    if interactive {
        return workbench::run(workbench::Options {
            data_dir: cli.data_dir,
            initial_source: cli.ovl_path,
            initial_mode: cli.mode,
            shade: cli.shade,
            model_query: cli.model,
            variant: cli.variant,
        });
    }

    let ovl_path = cli
        .ovl_path
        .ok_or("an OVL path is required for command-line export/audio/save modes")?;
    let cli = FileCli {
        ovl_path,
        mode: cli.mode,
        shade: cli.shade,
        output: cli.output,
    };

    match cli.mode {
        ViewerMode::Sprites => run_sprite_export(&cli),
        ViewerMode::Models => run_model_export(&cli),
        ViewerMode::Audio => run_audio_export(&cli),
        ViewerMode::Saves => run_saves_export(&cli),
        ViewerMode::Terrain => run_terrain_export(&cli),
    }
}

// ── Sprite export ───────────────────────────────────────────────────────────

fn run_sprite_export(cli: &FileCli) -> Result<(), Box<dyn std::error::Error>> {
    let data = std::fs::read(&cli.ovl_path)?;
    let ovl = v2k_formats::ovl::OvlFile::parse(&data)?;

    let atlas = v2k_formats::sections::parse_sprites(&ovl)?;
    let sprites: Vec<DecodedSprite> = atlas.decode_all_strict(cli.shade).map_err(|error| {
        format!(
            "failed to decode every sprite in {}: {error}",
            cli.ovl_path.display()
        )
    })?;

    if sprites.is_empty() {
        eprintln!("No sprites found in {}", cli.ovl_path.display());
        std::process::exit(1);
    }

    export_sprites_png(cli, &sprites)
}

// ── Model export ────────────────────────────────────────────────────────────

fn run_model_export(cli: &FileCli) -> Result<(), Box<dyn std::error::Error>> {
    let data = std::fs::read(&cli.ovl_path)?;
    let ovl = v2k_formats::ovl::OvlFile::parse(&data)?;

    let collection = v2k_formats::sections::parse_models(&ovl)
        .map_err(|e| format!("No Section 8 models in {}: {e}", cli.ovl_path.display()))?;

    if collection.all_entries.is_empty() {
        eprintln!("No model entries in {}", cli.ovl_path.display());
        std::process::exit(1);
    }

    export_models_obj(cli, &collection)
}

// ── Model OBJ export ─────────────────────────────────────────────────────

fn export_models_obj(
    cli: &FileCli,
    collection: &v2k_formats::models::ModelCollection,
) -> Result<(), Box<dyn std::error::Error>> {
    let stem = cli
        .ovl_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");

    let out_dir = cli
        .output
        .clone()
        .unwrap_or_else(|| PathBuf::from(format!("{stem}_models")));

    let report = v2k_extract::export_models(stem, &out_dir, collection)?;
    println!(
        "\nExported {} OBJ files + manifest.json to {}",
        report.files.saturating_sub(1),
        out_dir.display(),
    );
    Ok(())
}

// ── Audio export ────────────────────────────────────────────────────────────

fn run_audio_export(cli: &FileCli) -> Result<(), Box<dyn std::error::Error>> {
    let data = std::fs::read(&cli.ovl_path)?;
    let ovl = v2k_formats::ovl::OvlFile::parse(&data)?;

    let table = v2k_formats::sections::parse_anim_sound(&ovl).map_err(|e| {
        format!(
            "No Section 11 (anim/sound) in {}: {e}",
            cli.ovl_path.display()
        )
    })?;

    let stem = cli
        .ovl_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");

    let out_dir = cli
        .output
        .clone()
        .unwrap_or_else(|| PathBuf::from(format!("{stem}_audio")));

    // Summary table
    println!(
        "Section 11: {} entries ({} data blobs, {} aliases)",
        table.entries.len(),
        table.type1_count(),
        table.type5_count(),
    );
    println!();
    println!(
        "{:<6} {:<6} {:>10} {:>10}",
        "Index", "Type", "Size", "Duration"
    );
    println!("{}", "-".repeat(36));

    for entry in &table.entries {
        let type_str = match entry.entry_type {
            v2k_formats::anim_sound::EntryType::Unused => "t0",
            v2k_formats::anim_sound::EntryType::DataBlob => "t1",
            v2k_formats::anim_sound::EntryType::Alias => "t5",
        };
        let (size_str, dur_str) =
            if entry.entry_type == v2k_formats::anim_sound::EntryType::DataBlob {
                let size = entry.size_or_scale as usize;
                let dur = v2k_formats::wav::duration_secs(size, &v2k_formats::wav::V2K_AUDIO);
                (format_size(size), format!("{dur:.2}s"))
            } else {
                ("-".to_string(), "-".to_string())
            };
        println!(
            "{:<6} {:<6} {:>10} {:>10}",
            entry.index, type_str, size_str, dur_str
        );
    }

    // Export type=1 blobs as WAV
    let blob_entries: Vec<_> = table
        .entries
        .iter()
        .filter(|e| e.entry_type == v2k_formats::anim_sound::EntryType::DataBlob)
        .collect();

    if blob_entries.is_empty() {
        println!("\nNo data blobs to export.");
        return Ok(());
    }

    let report = v2k_extract::export_section11(stem, &out_dir, &table)?;
    println!(
        "\nExported {} WAV files + manifest.json to {}",
        report.items,
        out_dir.display()
    );

    Ok(())
}

// ── Sprite PNG export ────────────────────────────────────────────────────

fn export_sprites_png(
    cli: &FileCli,
    sprites: &[DecodedSprite],
) -> Result<(), Box<dyn std::error::Error>> {
    let stem = cli
        .ovl_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");

    let out_dir = cli
        .output
        .clone()
        .unwrap_or_else(|| PathBuf::from(format!("{stem}_sprites")));

    let report = v2k_extract::export_sprites(stem, &out_dir, sprites, cli.shade)?;
    println!(
        "\nExported {} PNG files + manifest.json to {}",
        report.items,
        out_dir.display(),
    );
    Ok(())
}

fn format_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

// ── Save slot export ────────────────────────────────────────────────────────

fn run_saves_export(cli: &FileCli) -> Result<(), Box<dyn std::error::Error>> {
    // Look for every playable retail slot in the containing directory. Menu
    // row 14 is reserved for settings and is not an ordinary Slot14 file.
    let search_dir = if cli.ovl_path.is_dir() {
        cli.ovl_path.clone()
    } else {
        cli.ovl_path
            .parent()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
    };

    let mut found = 0;

    println!("V2000 Save Slot Decoder");
    println!("{}", "=".repeat(60));
    println!("Scanning {} for save files...\n", search_dir.display());

    for index in 0..v2k_formats::saves::SAVE_SLOT_COUNT {
        let name = format!("Slot{index:02}");
        let path = search_dir.join(&name);
        if !path.exists() {
            continue;
        }
        let data = std::fs::read(&path)?;
        match v2k_formats::saves::SaveSlot::parse(&data) {
            Ok(slot) => {
                found += 1;
                let gs = &slot.game_state;
                let valid_str = if slot.is_valid() {
                    "OK (retail-recoverable)"
                } else {
                    "CORRUPT"
                };
                println!("  {name}:");
                println!(
                    "    Magic:       0x{:08X} ({})",
                    slot.magic,
                    if slot.magic == 5 { "OK" } else { "UNEXPECTED" }
                );
                println!(
                    "    Save label:  \"{}\" (logical={}, global={:?}, saveable={:?})",
                    gs.display_name,
                    gs.logical_level_id,
                    gs.global_level_id(),
                    gs.saveable_global_level_id()
                );
                println!("    Player XYZ:  {:?} (signed 8.8)", gs.player.position_raw);
                println!("    Velocity:    {:?} (signed 8.8)", gs.player.velocity_raw);
                println!(
                    "    Angles:      heading={:#06X} pitch={:#06X} roll={:#06X}",
                    gs.player.heading_raw, gs.player.pitch_raw, gs.player.roll_raw
                );
                println!("    Health:      {}", gs.player.health_raw);
                println!("    Field 38:    {} (0x{:X})", gs.field_38, gs.field_38);
                println!(
                    "    Raw 40/44/48: {}, {}, {}",
                    gs.field_40, gs.field_44, gs.field_48
                );
                println!(
                    "    Raw 4C/50/54: {}, {}, {}",
                    gs.field_4c, gs.field_50, gs.field_54
                );
                println!(
                    "    Raw 60/64/68: {}, {}, {}",
                    gs.field_60, gs.field_64, gs.field_68
                );
                println!(
                    "    State CRC:   0x{:08X} ({})",
                    slot.checksum,
                    if slot.checksums_match {
                        "A=B"
                    } else {
                        "MISMATCH"
                    }
                );
                println!(
                    "    CRC copies:  header={:?} state={:?} tail={:?}",
                    slot.header_crc_valid, slot.state_crc_valid, slot.tail_crc_valid
                );
                println!(
                    "    Selected:    header={:?} state={:?} tail={:?}",
                    slot.selected_header_copy, slot.selected_state_copy, slot.selected_tail_copy
                );
                println!("    Validity:    {valid_str}");
                if !slot.trailing_fields.is_empty() {
                    println!("    Non-zero unmapped state fields:");
                    for &(off, val) in &slot.trailing_fields {
                        println!("      0x{off:03X}: {val} (0x{val:X})");
                    }
                }
                println!();
            }
            Err(e) => {
                eprintln!("  {name}: ERROR — {e}\n");
            }
        }
    }

    if found == 0 {
        eprintln!("No save slot files found in {}", search_dir.display());
        std::process::exit(1);
    }

    println!("{}", "=".repeat(60));
    println!("Total: {found} slot(s) parsed");

    Ok(())
}

// ── Terrain export ──────────────────────────────────────────────────────────

fn run_terrain_export(cli: &FileCli) -> Result<(), Box<dyn std::error::Error>> {
    let data = std::fs::read(&cli.ovl_path)?;
    let ovl = v2k_formats::ovl::OvlFile::parse(&data)?;

    let grid = v2k_formats::sections::parse_terrain(&ovl)
        .map_err(|e| format!("No Section 10 (terrain) in {}: {e}", cli.ovl_path.display()))?;

    let stem = cli
        .ovl_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");

    let out_dir = cli
        .output
        .clone()
        .unwrap_or_else(|| PathBuf::from(format!("{stem}_terrain")));

    std::fs::create_dir_all(&out_dir)?;

    let size = v2k_formats::terrain::GRID_SIZE;
    let (h_min, h_max) = grid.height_range();

    // Grayscale heightmap (L8)
    let mut gray = Vec::with_capacity(size * size);
    for z in 0..size {
        for x in 0..size {
            gray.push(grid.cell(x, z).unwrap().height);
        }
    }
    let height_file = format!("{stem}_height.png");
    write_gray_png(&out_dir.join(&height_file), size as u32, size as u32, &gray)?;

    // RGB composite (R=height, G=attribute, B=terrain_type)
    let mut rgb = Vec::with_capacity(size * size * 3);
    for z in 0..size {
        for x in 0..size {
            let c = grid.cell(x, z).unwrap();
            rgb.push(c.height);
            rgb.push(c.attribute);
            rgb.push(c.terrain_type);
        }
    }
    let rgb_file = format!("{stem}_rgb.png");
    write_rgb_png(&out_dir.join(&rgb_file), size as u32, size as u32, &rgb)?;

    // Count unique terrain types
    let mut type_set = [false; 256];
    for c in &grid.cells {
        type_set[c.terrain_type as usize] = true;
    }
    let type_count = type_set.iter().filter(|&&b| b).count();

    println!("Terrain: {}x{} grid", size, size);
    println!("  Height range: [{h_min}-{h_max}]");
    println!("  Terrain types: {type_count} unique");
    println!(
        "  Header: [{}, {}, {}, {}, {}]",
        grid.header[0], grid.header[1], grid.header[2], grid.header[3], grid.header[4]
    );

    write_terrain_manifest(&out_dir, stem, &grid, &height_file, &rgb_file, type_count)?;

    println!(
        "\nExported 2 PNG files + manifest.json to {}",
        out_dir.display(),
    );

    Ok(())
}

fn write_terrain_manifest(
    out_dir: &Path,
    stem: &str,
    grid: &v2k_formats::terrain::TerrainGrid,
    height_file: &str,
    rgb_file: &str,
    type_count: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::fmt::Write;
    let (h_min, h_max) = grid.height_range();
    let size = v2k_formats::terrain::GRID_SIZE;
    let mut json = String::new();
    writeln!(json, "{{")?;
    writeln!(json, "  \"source\": \"{stem}\",")?;
    writeln!(json, "  \"height_file\": \"{height_file}\",")?;
    writeln!(json, "  \"rgb_file\": \"{rgb_file}\",")?;
    writeln!(json, "  \"grid_size\": {size},")?;
    writeln!(json, "  \"height_min\": {h_min},")?;
    writeln!(json, "  \"height_max\": {h_max},")?;
    writeln!(json, "  \"terrain_types\": {type_count},")?;
    writeln!(
        json,
        "  \"header\": [{}, {}, {}, {}, {}]",
        grid.header[0], grid.header[1], grid.header[2], grid.header[3], grid.header[4]
    )?;
    write!(json, "}}")?;
    std::fs::write(out_dir.join("manifest.json"), &json)?;
    Ok(())
}

fn write_gray_png(
    path: &Path,
    width: u32,
    height: u32,
    data: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    let file = std::fs::File::create(path)?;
    let w = BufWriter::new(file);
    let mut encoder = png::Encoder::new(w, width, height);
    encoder.set_color(png::ColorType::Grayscale);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(data)?;
    Ok(())
}

fn write_rgb_png(
    path: &Path,
    width: u32,
    height: u32,
    data: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    let file = std::fs::File::create(path)?;
    let w = BufWriter::new(file);
    let mut encoder = png::Encoder::new(w, width, height);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(data)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shade_parser_accepts_the_full_authored_range() {
        assert_eq!(parse_shade("0"), Ok(0));
        assert_eq!(
            parse_shade(&BRIGHTEST_SHADE.to_string()),
            Ok(BRIGHTEST_SHADE)
        );
        assert!(parse_shade(&SHADE_LEVELS.to_string()).is_err());
    }
}
