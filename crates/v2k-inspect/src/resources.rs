use serde::Serialize;

use crate::process::{plausible_heap_pointer, u32_at, Process};

const RESOURCE_POOL_GLOBALS: usize = 0x004F_E620;
const CURRENT_TERRAIN_INDEX: usize = 0x004F_EC40;
const WAVE_ENABLE: usize = 0x004F_ECE4;
const SECTION_COUNT: usize = 0x004C_E0A0;
const LEVEL_COUNT: usize = 0x004C_E0A4;
const LEVEL_DESCRIPTOR_TABLE: usize = 0x004D_0020;
const ACTIVE_VARIANT: usize = 0x004F_BE2C;
const EXPECTED_SECTIONS: usize = 15;
const EXPECTED_LEVELS: usize = 53;
const LEVEL_DESCRIPTOR_BYTES: usize = 0x10 + EXPECTED_SECTIONS * 8;

const RESOURCE_CACHE_TABLE_POINTER: usize = 0x004D_C9F4;
const RESOURCE_CACHE_TABLE_COUNT: usize = 0x004D_C9F8;
const CACHE_CLASS_BYTES: usize = 0x18;
const CACHE_CLASS_CAPACITY_OFFSET: usize = 0x10;
const CACHE_CLASS_ENTRIES_OFFSET: usize = 0x14;
const CACHE_ENTRY_BYTES: usize = 0x0C;

/// Passive reconstruction of `ResourceCache_Lookup`'s handle-to-object path.
/// Inspectors use the stored handle check to reject a recycled slot rather than
/// accidentally interpreting a different live resource as the requested one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResourceCacheResolution {
    pub handle: u32,
    pub class_index: u32,
    pub slot_index: u32,
    pub table_pointer: u32,
    pub class_capacity: u32,
    pub entries_pointer: u32,
    pub stored_handle: u32,
    pub object_pointer: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResourcePools {
    pub section_pointers: [u32; 15],
    pub strings: u32,
    pub sprites: u32,
    pub fonts: u32,
    pub shade_ramps: u32,
    pub colors: u32,
    pub models: u32,
    pub terrain: u32,
    pub sounds: u32,
    pub entity_types: u32,
    pub current_terrain_index: u32,
    pub wave_enable: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SectionRange {
    pub section: u32,
    pub global_base: u32,
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LoadedOverlay {
    pub level: u32,
    pub descriptor_pointer: u32,
    pub load_state: u32,
    pub descriptor_source_level: u32,
    pub sections: Vec<SectionRange>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResourceCatalog {
    pub section_count: u32,
    pub level_count: u32,
    pub active_variant: u32,
    pub section_totals: [u32; EXPECTED_SECTIONS],
    pub loaded_overlays: Vec<LoadedOverlay>,
}

impl ResourceCatalog {
    pub fn loaded_level_ids(&self) -> Vec<u32> {
        self.loaded_overlays
            .iter()
            .map(|overlay| overlay.level)
            .collect()
    }
}

pub fn read(process: &Process) -> Result<ResourcePools, String> {
    let mut section_pointers = [0u32; 15];
    for (section, pointer) in section_pointers.iter_mut().enumerate() {
        *pointer = process.read_u32(RESOURCE_POOL_GLOBALS + section * 4)?;
    }
    Ok(ResourcePools {
        strings: section_pointers[2],
        sprites: section_pointers[3],
        fonts: section_pointers[4],
        shade_ramps: section_pointers[6],
        colors: section_pointers[7],
        models: section_pointers[8],
        terrain: section_pointers[10],
        sounds: section_pointers[11],
        entity_types: section_pointers[12],
        section_pointers,
        current_terrain_index: process.read_u32(CURRENT_TERRAIN_INDEX)?,
        wave_enable: process.read_u32(WAVE_ENABLE)?,
    })
}

pub fn resolve_resource_handle(
    process: &Process,
    handle: u32,
) -> Result<ResourceCacheResolution, String> {
    let (class_index, slot_index) = handle_indices(handle);
    let class_count = process.read_u32(RESOURCE_CACHE_TABLE_COUNT)?;
    if class_index == 0 || class_index >= class_count || slot_index == 0 {
        return Err(format!(
            "resource handle {handle:08X} has invalid cache class/slot {class_index}/{slot_index} for {class_count} classes"
        ));
    }

    let table_pointer = process.read_u32(RESOURCE_CACHE_TABLE_POINTER)?;
    require_heap_pointer(table_pointer, "resource-cache table")?;
    let class_address = table_pointer as usize + class_index as usize * CACHE_CLASS_BYTES;
    let class_capacity = process.read_u32(class_address + CACHE_CLASS_CAPACITY_OFFSET)?;
    if slot_index >= class_capacity {
        return Err(format!(
            "resource-cache slot {slot_index} exceeds class {class_index} capacity {class_capacity}"
        ));
    }

    let entries_pointer = process.read_u32(class_address + CACHE_CLASS_ENTRIES_OFFSET)?;
    require_heap_pointer(entries_pointer, "resource-cache entries")?;
    let entry_address = entries_pointer as usize + slot_index as usize * CACHE_ENTRY_BYTES;
    let entry = process.read_bytes(entry_address, CACHE_ENTRY_BYTES)?;
    let stored_handle = u32_at(&entry, 0x00);
    if stored_handle != handle {
        return Err(format!(
            "resource-cache entry {class_index}/{slot_index} stores {stored_handle:08X}, expected {handle:08X}"
        ));
    }
    let object_pointer = u32_at(&entry, 0x04);
    require_heap_pointer(object_pointer, "resource-cache object")?;

    Ok(ResourceCacheResolution {
        handle,
        class_index,
        slot_index,
        table_pointer,
        class_capacity,
        entries_pointer,
        stored_handle,
        object_pointer,
    })
}

fn handle_indices(handle: u32) -> (u32, u32) {
    (handle >> 26, (handle >> 16) & 0x3FF)
}

fn require_heap_pointer(pointer: u32, label: &str) -> Result<(), String> {
    if plausible_heap_pointer(pointer as usize) {
        Ok(())
    } else {
        Err(format!("{label} pointer {pointer:08X} is implausible"))
    }
}

/// Read the PRELOAD-derived catalog that assigns every global resource ID to
/// its source level and section. Each of the 53 descriptor records contains
/// `{global_base,count}` for all 15 sections; its first dword is nonzero while
/// that overlay is loaded. Sampling this separately at 10 Hz is sufficient for
/// load/unload transitions and avoids burdening the 50 Hz entity pass.
pub fn read_catalog(process: &Process) -> Result<ResourceCatalog, String> {
    let section_count = process.read_u32(SECTION_COUNT)?;
    let level_count = process.read_u32(LEVEL_COUNT)?;
    if section_count as usize != EXPECTED_SECTIONS || level_count as usize != EXPECTED_LEVELS {
        return Err(format!(
            "unexpected runtime catalog dimensions {section_count}x{level_count}; expected {EXPECTED_SECTIONS}x{EXPECTED_LEVELS}"
        ));
    }

    let descriptor_table = process.read_bytes(LEVEL_DESCRIPTOR_TABLE, EXPECTED_LEVELS * 4)?;
    let mut loaded_overlays = Vec::new();
    let mut section_totals = [0u32; EXPECTED_SECTIONS];
    for level in 0..EXPECTED_LEVELS {
        let descriptor_pointer = u32_at(&descriptor_table, level * 4);
        if !plausible_heap_pointer(descriptor_pointer as usize) {
            return Err(format!(
                "level {level} descriptor pointer {descriptor_pointer:08X} is implausible"
            ));
        }
        let descriptor = process.read_bytes(descriptor_pointer as usize, LEVEL_DESCRIPTOR_BYTES)?;
        let load_state = u32_at(&descriptor, 0x00);
        let descriptor_source_level = u32_at(&descriptor, 0x0C);
        let mut sections = Vec::with_capacity(EXPECTED_SECTIONS);
        for (section, total) in section_totals.iter_mut().enumerate() {
            let offset = 0x10 + section * 8;
            let global_base = u32_at(&descriptor, offset);
            let count = u32_at(&descriptor, offset + 4);
            *total = (*total).max(global_base.saturating_add(count));
            sections.push(SectionRange {
                section: section as u32,
                global_base,
                count,
            });
        }
        if load_state != 0 {
            loaded_overlays.push(LoadedOverlay {
                level: level as u32,
                descriptor_pointer,
                load_state,
                descriptor_source_level,
                sections,
            });
        }
    }

    Ok(ResourceCatalog {
        section_count,
        level_count,
        active_variant: process.read_u32(ACTIVE_VARIANT)?,
        section_totals,
        loaded_overlays,
    })
}

#[cfg(test)]
mod tests {
    use super::handle_indices;

    #[test]
    fn resource_handle_indices_follow_retail_bit_fields() {
        let handle = (3 << 26) | (0x155 << 16) | 0xBEEF;
        assert_eq!(handle_indices(handle), (3, 0x155));
    }
}
