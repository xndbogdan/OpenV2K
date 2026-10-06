//! Solid world backgrounds selected by the current full-frame request.

use crate::main_base_abort::MainBaseAbortFrameRequest;
use crate::resource_cache::ResourceCache;

/// `FUN_0042F270` queues a complete-frame rectangle filled by `FUN_0047B9F0`.
pub struct SkyBackground {
    pub color: [f32; 3],
}

pub fn build_sky_background(cache: &ResourceCache) -> Option<SkyBackground> {
    build_sky_background_for_index(cache, cache.level_desc()?.sky_color_index)
}

/// Resolve an explicit request colour. Main Base abort presentation supplies
/// Section 13 `+0x58`; ordinary world presentation supplies `+0x54`.
pub fn build_sky_background_for_index(
    cache: &ResourceCache,
    color_index: u16,
) -> Option<SkyBackground> {
    let entry = cache.master_color_palette()?.get(color_index as usize)?;
    Some(SkyBackground {
        color: [
            entry.r as f32 / 255.0,
            entry.g as f32 / 255.0,
            entry.b as f32 / 255.0,
        ],
    })
}

/// Select the optional model from the current submitted request. `Some` with
/// model zero is an authored suppression, so it must not fall back to the
/// normal descriptor model.
pub fn submitted_sky_model_id(
    cache: &ResourceCache,
    frame_request: Option<MainBaseAbortFrameRequest>,
) -> Option<usize> {
    let selected = match frame_request {
        Some(request) => request.word_0xb2,
        None => cache.level_desc()?.sky_model,
    };
    (selected != 0).then_some(selected as usize)
}
