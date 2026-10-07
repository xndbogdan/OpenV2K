//! Deterministic asset export built on the canonical `v2k-formats` decoders.
//!
//! This crate owns serialization only. Format interpretation stays in
//! `v2k-formats`, so the game, viewer, and batch extractor cannot drift into
//! separate understandings of V2000 data.

pub mod sounds;
pub use sounds::{export_global_sounds, GlobalSoundExportReport, V2000SoundSources};

use serde::Serialize;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs::File;
use std::io::{BufWriter, Write as IoWrite};
use std::path::{Path, PathBuf};
use v2k_formats::anim_sound::{AnimSoundTable, EntryType};
use v2k_formats::models::{
    face_material, AnimVars, MaterializedModel, ModelCollection, ModelEntry, ModelInstance,
};
use v2k_formats::sprites::DecodedSprite;

pub const MANIFEST_SCHEMA_VERSION: u32 = 1;
const MODEL_MANIFEST_SCHEMA_VERSION: u32 = 2;
const MODEL_RAW_UNITS_PER_WORLD_UNIT: u16 = 256;

pub type ExportResult<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExportReport {
    pub items: usize,
    pub files: usize,
}

#[derive(Serialize)]
struct SpriteManifest<'a> {
    schema_version: u32,
    kind: &'static str,
    source: &'a str,
    shade_level: usize,
    count: usize,
    sprites: Vec<SpriteRecord>,
}

#[derive(Serialize)]
struct SpriteRecord {
    file: String,
    entry_idx: usize,
    index: u16,
    width: u16,
    height: u16,
    render_flags: u16,
    palette_layout: u16,
    packed_dimensions: u32,
}

#[derive(Serialize)]
struct ModelManifest<'a> {
    schema_version: u32,
    kind: &'static str,
    source: &'a str,
    coordinate_units: ModelCoordinateUnits,
    materialization: ModelMaterialization,
    limitations: ModelLimitations,
    count: usize,
    models: Vec<ModelRecord>,
}

#[derive(Serialize)]
struct ModelCoordinateUnits {
    raw_coordinates: &'static str,
    obj_coordinates: &'static str,
    raw_units_per_world_unit: u16,
    uv_coordinates: &'static str,
    instance_orientation: &'static str,
}

#[derive(Serialize)]
struct ModelMaterialization {
    animation_state: &'static str,
    register_semantics: &'static str,
    linked_parent_context: &'static str,
}

#[derive(Serialize)]
struct ModelLimitations {
    view_dependent_type_13: &'static str,
    linked_type_11: &'static str,
    child_hierarchy: &'static str,
    external_frame_type_14: &'static str,
    billboards: &'static str,
    sprite_materials: &'static str,
}

#[derive(Serialize)]
struct ModelRecord {
    file: Option<String>,
    index: usize,
    name: Option<String>,
    vertices: usize,
    triangles: usize,
    shadow_triangles: usize,
    edges: usize,
    billboards: usize,
    instance_count: usize,
    instances: Vec<ModelInstanceRecord>,
    materials: Vec<ModelMaterialRecord>,
    type_11_linked_vertex_records: usize,
    type_13_view_dependent_vertex_records: usize,
    type_14_external_frame_vertex_records: usize,
    linked_faces_may_be_omitted: bool,
    type_13_view_projection_not_applied: bool,
    external_attachment_frame_not_applied: bool,
    child_instances_not_flattened: bool,
    collision_bytes: usize,
    collision_radius_raw: u16,
    collision_radius_world: f64,
    face_val: u16,
    cmd_word_count: u16,
    flags: u8,
}

#[derive(Serialize)]
struct ModelMaterialRecord {
    packed_id: u16,
    material_id: u16,
    source_kind: &'static str,
    obj_name: String,
    triangle_count: usize,
}

#[derive(Serialize)]
struct ModelInstanceRecord {
    target_global_model_id: u16,
    attach_slot_raw: u16,
    attach_position_raw: Option<[f64; 3]>,
    attach_position_world: Option<[f64; 3]>,
    orientation_row_major: [[f64; 3]; 3],
    transform_world: Option<[[f64; 4]; 4]>,
    linked_slots: Vec<u16>,
    register_snapshot: Vec<u16>,
}

#[derive(Serialize)]
struct Section11Manifest<'a> {
    schema_version: u32,
    kind: &'static str,
    source: &'a str,
    entry_count: usize,
    pcm_count: usize,
    alias_count: usize,
    note: &'static str,
    entries: Vec<Section11Record>,
}

#[derive(Serialize)]
struct Section11Record {
    index: usize,
    entry_type: &'static str,
    file: Option<String>,
    pcm_offset: Option<u32>,
    pcm_bytes: Option<u32>,
    alias_target_global_id: Option<u32>,
    frequency_variance_16_16: Option<u32>,
    frequency_multiplier_16_16: Option<u32>,
    volume_multiplier_16_16: Option<u32>,
}

/// Export decoded sprites as RGBA PNGs plus a deterministic manifest.
pub fn export_sprites(
    source: &str,
    output_dir: &Path,
    sprites: &[DecodedSprite],
    shade: usize,
) -> ExportResult<ExportReport> {
    std::fs::create_dir_all(output_dir)?;
    let mut records = Vec::with_capacity(sprites.len());

    for sprite in sprites {
        let entry = &sprite.entry;
        let filename = format!(
            "sprite_{:04}_{:05}_{}x{}.png",
            entry.entry_idx, entry.index, sprite.width, sprite.height,
        );
        write_rgba_png(
            &output_dir.join(&filename),
            sprite.width as u32,
            sprite.height as u32,
            &sprite.rgba,
        )?;
        records.push(SpriteRecord {
            file: filename,
            entry_idx: entry.entry_idx,
            index: entry.index,
            width: sprite.width,
            height: sprite.height,
            render_flags: entry.pal_size,
            palette_layout: entry.shade_count,
            packed_dimensions: entry.flags,
        });
    }

    write_manifest(
        output_dir,
        &SpriteManifest {
            schema_version: MANIFEST_SCHEMA_VERSION,
            kind: "sprites",
            source,
            shade_level: shade,
            count: records.len(),
            sprites: records,
        },
    )?;

    Ok(ExportReport {
        items: sprites.len(),
        files: sprites.len() + 1,
    })
}

/// Export every default-frame runtime-materialized model with geometry as OBJ
/// and retain every Section-8 entry in the manifest, including
/// instance-only/empty records.
///
/// Geometry is re-interpreted with [`AnimVars::default`] so embedded register
/// operations execute exactly as they do at runtime. OBJ positions use the
/// gameplay `raw / 256` world-unit conversion; they never use the cached
/// legacy `ModelEntry::to_obj` (`raw / 100`) representation.
pub fn export_models(
    source: &str,
    output_dir: &Path,
    collection: &ModelCollection,
) -> ExportResult<ExportReport> {
    std::fs::create_dir_all(output_dir)?;
    let mut records = Vec::with_capacity(collection.all_entries.len());
    let mut obj_count = 0usize;

    for model in &collection.all_entries {
        let materialized = model.materialize(&AnimVars::default());
        let file = if materialized.vertices.is_empty() {
            None
        } else {
            let name_suffix = model
                .name
                .as_deref()
                .map(sanitize_component)
                .filter(|name| !name.is_empty())
                .map(|name| format!("_{name}"))
                .unwrap_or_default();
            let filename = format!("model_{:03}{name_suffix}.obj", model.index);
            std::fs::write(
                output_dir.join(&filename),
                materialized_model_to_obj(model, &materialized)?,
            )?;
            obj_count += 1;
            Some(filename)
        };
        let type_11_linked_vertex_records = count_vertex_records(model, 11);
        let type_13_view_dependent_vertex_records = count_vertex_records(model, 13);
        let type_14_external_frame_vertex_records = count_vertex_records(model, 14);
        let instances = materialized
            .instances
            .iter()
            .map(model_instance_record)
            .collect::<Vec<_>>();
        records.push(ModelRecord {
            file,
            index: model.index,
            name: model.name.clone(),
            vertices: materialized.vertices.len(),
            triangles: materialized.triangles.len(),
            shadow_triangles: materialized.shadow_triangles.len(),
            edges: materialized.edges.len(),
            billboards: materialized.billboards.len(),
            instance_count: instances.len(),
            instances,
            materials: model_material_records(&materialized.face_materials),
            type_11_linked_vertex_records,
            type_13_view_dependent_vertex_records,
            type_14_external_frame_vertex_records,
            linked_faces_may_be_omitted: type_11_linked_vertex_records != 0,
            type_13_view_projection_not_applied: type_13_view_dependent_vertex_records != 0,
            external_attachment_frame_not_applied: type_14_external_frame_vertex_records != 0,
            child_instances_not_flattened: !materialized.instances.is_empty(),
            collision_bytes: model.collision_program.len(),
            collision_radius_raw: model.collision_radius_raw,
            collision_radius_world: f64::from(model.collision_radius_raw)
                / f64::from(MODEL_RAW_UNITS_PER_WORLD_UNIT),
            face_val: model.face_val,
            cmd_word_count: model.cmd_word_count,
            flags: model.flags,
        });
    }

    let item_count = records.len();
    write_manifest(
        output_dir,
        &ModelManifest {
            schema_version: MODEL_MANIFEST_SCHEMA_VERSION,
            kind: "models",
            source,
            coordinate_units: ModelCoordinateUnits {
                raw_coordinates: "signed Section-8 model coordinates",
                obj_coordinates: "gameplay world units (raw / 256)",
                raw_units_per_world_unit: MODEL_RAW_UNITS_PER_WORLD_UNIT,
                uv_coordinates: "runtime face UVs preserved without flipping the V axis",
                instance_orientation: "row-major child-to-parent model-space basis",
            },
            materialization: ModelMaterialization {
                animation_state: "AnimVars::default() (frame-zero callback and caller registers)",
                register_semantics: "embedded command-stream register operations execute in runtime order",
                linked_parent_context: "none at root export",
            },
            limitations: ModelLimitations {
                view_dependent_type_13: "type-13 sky-pin vertices retain their referenced local position; the runtime view-space Y pin and terrain-projected footprint are not representable in a static OBJ",
                linked_type_11: "type-11 vertices require an op-0x0E parent linked-slot table; root exports have no parent context, so affected faces can be omitted",
                child_hierarchy: "op-0x0E children are listed with global target ids, attachment positions, orientations, remap slots, and register snapshots but are not flattened into the parent OBJ",
                external_frame_type_14: "type-14 vertices retain authored external-frame coordinates because the live ctx+0x5C attachment frame is unavailable",
                billboards: "screen-facing billboard quads are represented only by OBJ points and manifest counts",
                sprite_materials: "sprite material ids and face UVs are preserved, but texture image files are exported separately by the sprite exporter",
            },
            count: item_count,
            models: records,
        },
    )?;

    Ok(ExportReport {
        items: item_count,
        files: obj_count + 1,
    })
}

fn count_vertex_records(model: &ModelEntry, type_flag: i16) -> usize {
    model
        .records
        .iter()
        .filter(|record| record[0] == type_flag)
        .count()
}

fn model_material_records(face_materials: &[u16]) -> Vec<ModelMaterialRecord> {
    let mut counts = BTreeMap::<u16, usize>::new();
    for &packed in face_materials {
        *counts.entry(packed).or_default() += 1;
    }
    counts
        .into_iter()
        .map(|(packed_id, triangle_count)| {
            let (material_id, is_sprite) = face_material(packed_id);
            ModelMaterialRecord {
                packed_id,
                material_id,
                source_kind: if is_sprite {
                    "global_section3_sprite"
                } else {
                    "section7_palette"
                },
                obj_name: obj_material_name(packed_id),
                triangle_count,
            }
        })
        .collect()
}

fn obj_material_name(packed: u16) -> String {
    let (material_id, is_sprite) = face_material(packed);
    if is_sprite {
        format!("sprite_{material_id:05}")
    } else {
        format!("palette_{material_id:05}")
    }
}

fn raw_position_to_world(position: [f64; 3]) -> [f64; 3] {
    position.map(|axis| axis / f64::from(MODEL_RAW_UNITS_PER_WORLD_UNIT))
}

fn model_instance_record(instance: &ModelInstance) -> ModelInstanceRecord {
    let attach_position_world = instance.attach_pos.map(raw_position_to_world);
    let transform_world = attach_position_world.map(|translation| {
        [
            [
                instance.orientation[0][0],
                instance.orientation[0][1],
                instance.orientation[0][2],
                translation[0],
            ],
            [
                instance.orientation[1][0],
                instance.orientation[1][1],
                instance.orientation[1][2],
                translation[1],
            ],
            [
                instance.orientation[2][0],
                instance.orientation[2][1],
                instance.orientation[2][2],
                translation[2],
            ],
            [0.0, 0.0, 0.0, 1.0],
        ]
    });
    ModelInstanceRecord {
        target_global_model_id: instance.model_id,
        attach_slot_raw: instance.attach_slot,
        attach_position_raw: instance.attach_pos,
        attach_position_world,
        orientation_row_major: instance.orientation,
        transform_world,
        linked_slots: instance.linked_slots.clone(),
        register_snapshot: instance.registers.to_vec(),
    }
}

fn materialized_model_to_obj(
    model: &ModelEntry,
    materialized: &MaterializedModel,
) -> ExportResult<String> {
    if materialized.face_materials.len() != materialized.triangles.len() {
        return Err(format!(
            "model {} has {} triangles but {} material ids",
            model.index,
            materialized.triangles.len(),
            materialized.face_materials.len(),
        )
        .into());
    }
    if materialized.face_uvs.len() != materialized.triangles.len() {
        return Err(format!(
            "model {} has {} triangles but {} UV triplets",
            model.index,
            materialized.triangles.len(),
            materialized.face_uvs.len(),
        )
        .into());
    }

    let vertex_count = materialized.vertices.len();
    for (face_index, triangle) in materialized
        .triangles
        .iter()
        .chain(&materialized.shadow_triangles)
        .enumerate()
    {
        if triangle
            .iter()
            .any(|&index| usize::from(index) >= vertex_count)
        {
            return Err(format!(
                "model {} face {} references a vertex outside 0..{}",
                model.index, face_index, vertex_count,
            )
            .into());
        }
    }
    for (edge_index, edge) in materialized.edges.iter().enumerate() {
        if edge
            .vertices
            .iter()
            .any(|&index| usize::from(index) >= vertex_count)
        {
            return Err(format!(
                "model {} edge {} references a vertex outside 0..{}",
                model.index, edge_index, vertex_count,
            )
            .into());
        }
    }
    for (billboard_index, billboard) in materialized.billboards.iter().enumerate() {
        if usize::from(billboard.vertex) >= vertex_count {
            return Err(format!(
                "model {} billboard {} references a vertex outside 0..{}",
                model.index, billboard_index, vertex_count,
            )
            .into());
        }
    }

    let object_name = model
        .name
        .as_deref()
        .map(sanitize_component)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| format!("model_{:03}", model.index));
    let mut output = String::new();
    writeln!(output, "# Canonical V2000 Section-8 diagnostic model")?;
    writeln!(
        output,
        "# frame: AnimVars::default(); embedded register operations executed"
    )?;
    writeln!(output, "# coordinates: gameplay world units (raw / 256)")?;
    writeln!(
        output,
        "# UVs: runtime values preserved verbatim; V is not flipped for OBJ conventions"
    )?;
    writeln!(
        output,
        "# op-0x0E child instances are recorded below and in manifest.json; they are not flattened"
    )?;
    writeln!(output, "o {object_name}")?;

    for vertex in &materialized.vertices {
        let world = raw_position_to_world(*vertex);
        writeln!(output, "v {:.9} {:.9} {:.9}", world[0], world[1], world[2])?;
    }
    for triangle_uvs in &materialized.face_uvs {
        for uv in triangle_uvs {
            writeln!(output, "vt {:.9} {:.9}", uv[0], uv[1])?;
        }
    }

    if !materialized.triangles.is_empty() {
        writeln!(output, "g {object_name}__body")?;
        let mut active_material = None;
        for (face_index, triangle) in materialized.triangles.iter().enumerate() {
            let packed_material = materialized.face_materials[face_index];
            if active_material != Some(packed_material) {
                let (material_id, is_sprite) = face_material(packed_material);
                writeln!(
                    output,
                    "# material packed=0x{packed_material:04X} id={material_id} source={}",
                    if is_sprite { "sprite" } else { "palette" }
                )?;
                writeln!(output, "usemtl {}", obj_material_name(packed_material))?;
                active_material = Some(packed_material);
            }
            let texture_index = face_index * 3 + 1;
            writeln!(
                output,
                "f {}/{} {}/{} {}/{}",
                usize::from(triangle[0]) + 1,
                texture_index,
                usize::from(triangle[1]) + 1,
                texture_index + 1,
                usize::from(triangle[2]) + 1,
                texture_index + 2,
            )?;
        }
    }

    if !materialized.shadow_triangles.is_empty() {
        writeln!(output, "g {object_name}__type13_shadow_unprojected")?;
        writeln!(output, "usemtl v2k_type13_shadow_unprojected")?;
        for triangle in &materialized.shadow_triangles {
            writeln!(
                output,
                "f {} {} {}",
                usize::from(triangle[0]) + 1,
                usize::from(triangle[1]) + 1,
                usize::from(triangle[2]) + 1
            )?;
        }
    }

    if !materialized.edges.is_empty() {
        writeln!(output, "g {object_name}__edges")?;
        for edge in &materialized.edges {
            writeln!(
                output,
                "l {} {}",
                usize::from(edge.vertices[0]) + 1,
                usize::from(edge.vertices[1]) + 1
            )?;
        }
    }

    if materialized.triangles.is_empty()
        && materialized.edges.is_empty()
        && !materialized.vertices.is_empty()
    {
        writeln!(output, "g {object_name}__points")?;
        write!(output, "p")?;
        for vertex_index in 1..=materialized.vertices.len() {
            write!(output, " {vertex_index}")?;
        }
        writeln!(output)?;
    }

    if !materialized.billboards.is_empty() {
        writeln!(output, "g {object_name}__billboard_anchors")?;
        for billboard in &materialized.billboards {
            writeln!(
                output,
                "# billboard anchor=v{} id={} size={} angle={} source={}",
                usize::from(billboard.vertex) + 1,
                billboard.id,
                billboard.size,
                billboard.angle,
                if billboard.textured {
                    "sprite"
                } else {
                    "palette"
                }
            )?;
            writeln!(output, "p {}", usize::from(billboard.vertex) + 1)?;
        }
    }

    for instance in &materialized.instances {
        writeln!(
            output,
            "# child target_global_model_id={} attach_slot={} attach_position_raw={:?} orientation_row_major={:?} linked_slots={:?}",
            instance.model_id,
            instance.attach_slot,
            instance.attach_pos,
            instance.orientation,
            instance.linked_slots,
        )?;
    }

    Ok(output)
}

/// Export one Section-11 table as a diagnostic WAV set and raw table manifest.
/// Prefer [`export_global_sounds`] for a canonical V2000 export: type-5 targets
/// are global ids and cannot be resolved correctly from an isolated table.
pub fn export_section11(
    source: &str,
    output_dir: &Path,
    table: &AnimSoundTable,
) -> ExportResult<ExportReport> {
    std::fs::create_dir_all(output_dir)?;
    let mut records = Vec::with_capacity(table.entries.len());
    let mut exported = 0usize;

    for entry in &table.entries {
        let (entry_type, file) = match entry.entry_type {
            EntryType::Unused => ("unused", None),
            EntryType::Alias => ("alias", None),
            EntryType::DataBlob => {
                let filename = format!("section11_{:03}.wav", entry.index);
                let file = if let Some(wav) = table.blob_as_wav(entry) {
                    std::fs::write(output_dir.join(&filename), wav)?;
                    exported += 1;
                    Some(filename)
                } else {
                    None
                };
                ("pcm", file)
            }
        };
        let is_pcm = entry.entry_type == EntryType::DataBlob;
        let is_alias = entry.entry_type == EntryType::Alias;
        records.push(Section11Record {
            index: entry.index,
            entry_type,
            file,
            pcm_offset: is_pcm.then_some(entry.offset_or_index),
            pcm_bytes: is_pcm.then_some(entry.size_or_scale),
            alias_target_global_id: is_alias.then_some(entry.offset_or_index),
            frequency_variance_16_16: is_alias.then_some(entry.size_or_scale),
            frequency_multiplier_16_16: is_alias.then_some(entry.param1),
            volume_multiplier_16_16: is_alias.then_some(entry.param2),
        });
    }

    write_manifest(
        output_dir,
        &Section11Manifest {
            schema_version: MANIFEST_SCHEMA_VERSION,
            kind: "section11",
            source,
            entry_count: table.entries.len(),
            pcm_count: table.type1_count(),
            alias_count: table.type5_count(),
            note: "Single-table diagnostic export; type-5 targets are GLOBAL ids. Use the global sound export to resolve aliases.",
            entries: records,
        },
    )?;

    Ok(ExportReport {
        items: exported,
        files: exported + 1,
    })
}

pub fn source_key(source: &str) -> String {
    let path = PathBuf::from(source);
    let filename = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(source);
    let candidate = filename
        .strip_suffix(".OVL")
        .or_else(|| filename.strip_suffix(".ovl"))
        .unwrap_or(filename);
    let key = sanitize_component(candidate);
    if key.is_empty() {
        "source".to_string()
    } else {
        key
    }
}

fn sanitize_component(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn write_manifest<T: Serialize>(output_dir: &Path, manifest: &T) -> ExportResult<()> {
    let file = File::create(output_dir.join("manifest.json"))?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, manifest)?;
    writer.flush()?;
    Ok(())
}

fn write_rgba_png(path: &Path, width: u32, height: u32, rgba: &[u8]) -> ExportResult<()> {
    let expected = width as usize * height as usize * 4;
    if rgba.len() != expected {
        return Err(format!(
            "{} has {} RGBA bytes, expected {expected}",
            path.display(),
            rgba.len(),
        )
        .into());
    }
    let file = File::create(path)?;
    let mut output = BufWriter::new(file);
    {
        let mut encoder = png::Encoder::new(&mut output, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(rgba)?;
        writer.finish()?;
    }
    output.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use v2k_formats::anim_sound;
    use v2k_formats::models::StreamStats;
    use v2k_formats::sprites::SpriteEntry;

    fn test_dir(label: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("v2k-extract-{label}-{}", std::process::id(),));
        let _ = std::fs::remove_dir_all(&path);
        path
    }

    fn synthetic_model(records: Vec<[i16; 4]>, cmd_words: Vec<u16>) -> ModelEntry {
        ModelEntry {
            index: 0,
            cmd_word_count: cmd_words.len() as u16,
            extra_count: 0,
            flags: 0x40,
            slot_count: (records.len() * 2) as u16,
            face_val: 4,
            radius: 0,
            collision_radius_raw: 128,
            collision_program: Vec::new(),
            records,
            normal_pool: vec![[0, 0, 0, 32767]],
            cmd_words,
            has_view_commands: false,
            vertices: vec![[99_900.0, 0.0, 0.0]],
            vertex_type_flags: vec![0],
            vertex_projection: vec![v2k_formats::models::ModelVertexProjection::Position],
            vertex_clip: Vec::new(),
            vertex_surface_origin: Vec::new(),
            triangles: Vec::new(),
            face_vertices: Vec::new(),
            normals: Vec::new(),
            face_cull: Vec::new(),
            face_materials: Vec::new(),
            face_uvs: Vec::new(),
            face_corner_normals: Vec::new(),
            face_shading: Vec::new(),
            shadow_triangles: Vec::new(),
            edges: Vec::new(),
            billboards: Vec::new(),
            instances: Vec::new(),
            painter_program: Vec::new(),
            name: Some("synthetic model".to_string()),
        }
    }

    fn model_collection(model: ModelEntry) -> ModelCollection {
        ModelCollection {
            sub_blocks: Vec::new(),
            all_entries: vec![model],
            stats: StreamStats::default(),
        }
    }

    #[test]
    fn source_keys_are_safe_and_stable() {
        assert_eq!(source_key("Overlay/0X14XX.OVL"), "0X14XX");
        assert_eq!(source_key("PRELOAD.DAT[2]"), "PRELOAD_DAT_2_");
    }

    #[test]
    fn exports_sprite_png_and_manifest() {
        let output = test_dir("sprite");
        let sprite = DecodedSprite {
            entry: SpriteEntry {
                entry_idx: 2,
                index: 7,
                pal_size: 0x10,
                shade_count: 16,
                tex_offset: 0,
                pal_offset: 0,
                flags: 0x0002_0001,
            },
            width: 1,
            height: 1,
            rgba: vec![1, 2, 3, 255],
        };
        let report = export_sprites("test.ovl", &output, &[sprite], 15).unwrap();
        assert_eq!(report, ExportReport { items: 1, files: 2 });
        assert!(output.join("sprite_0002_00007_1x1.png").is_file());
        let manifest = std::fs::read_to_string(output.join("manifest.json")).unwrap();
        assert!(manifest.contains("\"render_flags\": 16"));
        let _ = std::fs::remove_dir_all(output);
    }

    #[test]
    fn model_export_uses_runtime_materialization_world_units_and_face_uvs() {
        let output = test_dir("model-runtime");
        let records = vec![[0, 0, 0, 0], [0, 256, 0, 0], [8, 0xC0, 0, 2]];
        let words = vec![
            0x0D, 0, 0x3D, 0, // r0 = packed immediate 0x8000 + 0
            0x83, 17, 0, 0, 2, 4, // textured triangle using the tf-8 vertex
            0,
        ];
        let report = export_models(
            "test.ovl",
            &output,
            &model_collection(synthetic_model(records, words)),
        )
        .unwrap();
        assert_eq!(report, ExportReport { items: 1, files: 2 });

        let obj = std::fs::read_to_string(output.join("model_000_synthetic_model.obj")).unwrap();
        assert!(obj.contains("embedded register operations executed"));
        assert!(obj.contains("v 1.000000000 0.000000000 0.000000000"));
        assert!(obj.contains("v 0.500000000 0.000000000 0.000000000"));
        assert!(!obj.contains("999.000"));
        assert!(obj.contains("usemtl sprite_00017"));
        assert!(obj.contains("vt 1.000000000 0.000000000"));
        assert!(obj.contains("vt 1.000000000 1.000000000"));
        assert!(obj.contains("f 1/1 2/2 3/3"));

        let manifest: serde_json::Value =
            serde_json::from_slice(&std::fs::read(output.join("manifest.json")).unwrap()).unwrap();
        assert_eq!(manifest["schema_version"], 2);
        assert_eq!(
            manifest["coordinate_units"]["raw_units_per_world_unit"],
            256
        );
        assert!(manifest["materialization"]["register_semantics"]
            .as_str()
            .unwrap()
            .contains("runtime order"));
        assert_eq!(manifest["models"][0]["materials"][0]["packed_id"], 0x8011);
        assert_eq!(manifest["models"][0]["materials"][0]["material_id"], 17);
        assert_eq!(
            manifest["models"][0]["materials"][0]["source_kind"],
            "global_section3_sprite"
        );
        assert_eq!(manifest["models"][0]["collision_radius_world"], 0.5);
        let _ = std::fs::remove_dir_all(output);
    }

    #[test]
    fn model_manifest_preserves_child_target_transform_links_and_registers() {
        let output = test_dir("model-instance");
        let records = vec![
            [0, 256, 512, 768],
            [11, 0, 0, 0],
            [13, 0, 0, 0],
            [14, 1, 2, 3],
        ];
        let words = vec![
            0x0D, 0, 0x3D, 0, // r0 = 0x8000 before the child is emitted
            0x0E, 0x10, 317, 8, 0, // child 317, reflected Y, attached at slot 0
            0,
        ];
        export_models(
            "test.ovl",
            &output,
            &model_collection(synthetic_model(records, words)),
        )
        .unwrap();

        let manifest: serde_json::Value =
            serde_json::from_slice(&std::fs::read(output.join("manifest.json")).unwrap()).unwrap();
        let model = &manifest["models"][0];
        assert_eq!(model["instance_count"], 1);
        assert_eq!(model["child_instances_not_flattened"], true);
        assert_eq!(model["type_11_linked_vertex_records"], 1);
        assert_eq!(model["type_13_view_dependent_vertex_records"], 1);
        assert_eq!(model["type_14_external_frame_vertex_records"], 1);
        assert_eq!(model["linked_faces_may_be_omitted"], true);
        assert_eq!(model["type_13_view_projection_not_applied"], true);
        assert_eq!(model["external_attachment_frame_not_applied"], true);
        let instance = &model["instances"][0];
        assert_eq!(instance["target_global_model_id"], 317);
        assert_eq!(instance["attach_slot_raw"], 0);
        assert_eq!(
            instance["attach_position_raw"],
            serde_json::json!([256.0, 512.0, 768.0])
        );
        assert_eq!(
            instance["attach_position_world"],
            serde_json::json!([1.0, 2.0, 3.0])
        );
        assert_eq!(instance["orientation_row_major"][1][1], -1.0);
        assert_eq!(instance["transform_world"][0][3], 1.0);
        assert_eq!(instance["transform_world"][1][3], 2.0);
        assert_eq!(instance["transform_world"][2][3], 3.0);
        assert_eq!(instance["linked_slots"], serde_json::json!([0]));
        assert_eq!(instance["register_snapshot"][0], 0x8000);
        assert!(manifest["limitations"]["child_hierarchy"]
            .as_str()
            .unwrap()
            .contains("not flattened"));
        assert!(manifest["limitations"]["view_dependent_type_13"]
            .as_str()
            .unwrap()
            .contains("view-space Y pin"));
        assert!(manifest["limitations"]["linked_type_11"]
            .as_str()
            .unwrap()
            .contains("parent linked-slot table"));
        let _ = std::fs::remove_dir_all(output);
    }

    #[test]
    fn section11_diagnostic_manifest_names_sound_fields() {
        let mut bytes = vec![0u8; 64];
        bytes[0..4].copy_from_slice(&1u32.to_le_bytes());
        bytes[4..8].copy_from_slice(&60u32.to_le_bytes());
        bytes[8..12].copy_from_slice(&4u32.to_le_bytes());
        bytes[20..24].copy_from_slice(&5u32.to_le_bytes());
        bytes[24..28].copy_from_slice(&0u32.to_le_bytes());
        bytes[28..32].copy_from_slice(&65_536u32.to_le_bytes());
        bytes[40..44].copy_from_slice(&99u32.to_le_bytes());
        let table = anim_sound::parse_anim_sound(&bytes).unwrap();
        let output = test_dir("section11");
        let report = export_section11("test.ovl", &output, &table).unwrap();
        assert_eq!(report, ExportReport { items: 1, files: 2 });
        assert!(output.join("section11_000.wav").is_file());
        let manifest = std::fs::read_to_string(output.join("manifest.json")).unwrap();
        assert!(manifest.contains("type-5 targets are GLOBAL ids"));
        assert!(manifest.contains("\"entry_type\": \"alias\""));
        assert!(manifest.contains("\"volume_multiplier_16_16\""));
        let _ = std::fs::remove_dir_all(output);
    }
}
