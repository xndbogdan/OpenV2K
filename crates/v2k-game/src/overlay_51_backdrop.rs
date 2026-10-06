//! Overlay-51 results backdrop from `DAT_004d1204` and Section-1 percents.
//!
//! `FUN_00454ff0` allocates one `DAT_004fe600` node per type `1..=count`.
//! Campaign slot is the previous record's `+4` word (`[esi-8]`). Flags come
//! from that slot's `+0xD8` bits, and `session+0x34 == type` sets blink `0x10`.
//! `FUN_00454460` then walks nodes with `type < 0x25`: bit `0x10` shows only
//! on odd `+0x270 / 100` ticks. `FUN_00454c70` places those objects at
//! `(S1[layout].xy * S1[46].xy) / 100` and blits sprite 3765 after each tile.
//! Flag 8/0x20 then blit sprite 532 (`DAT_004fe62c+0x850`) from the tile
//! center plus overlay-3 S1 31/32. When bit 4 is clear, `FUN_0047aa20`
//! stretches sprite 583 (`DAT_004fe62c+0x91c`) over the 3765 rectangle.
//! `FUN_00454460` then walks `DAT_004fe5f0` and `FUN_00454520` stretches
//! sprite 3764 between saved/visited nodes using overlay-3 Section-14
//! record 3's 38×7 neighbor matrix, gated by `FUN_0042edc0` bits 9..=15.
//! Sprite 3763 caps fire when the recovered `+0x20`/`+0x24` words keep bit 0.

use crate::model_color::{sprite_blend, sprite_flat_shade_row};
use crate::power_up_contact::{PlayerCampaignProgress, RETAIL_CONTROL_PAIR_LINK_COLUMNS};
use crate::resource_cache::ResourceCache;
use v2k_formats::fixed_math::retail_integer_sqrt;
use v2k_formats::linkage::LinkageTable;
use v2k_render::WorldSpriteBlend;

/// Overlay that authors the thirty backdrop tiles.
pub const OVERLAY_51_SYSTEM_LEVEL: u32 = 51;
/// Overlay-3 local index of `DAT_004FE624+0xB8` (global S1 46).
pub const OVERLAY_51_SCALE_LOCAL_INDEX: usize = 33;
/// First overlay-51 Section-1 point (`DAT_004d1204` type 1 / global 48).
pub const OVERLAY_51_S1_BASE: u16 = 48;
/// First global sprite id authored by overlay 51.
pub const OVERLAY_51_FIRST_SPRITE_ID: u16 = 3733;
/// Last unique backdrop tile sprite (`DAT_004d1204` type 13 / 15..=17 / 29 / 32).
pub const OVERLAY_51_LAST_TILE_SPRITE_ID: u16 = 3762;
/// `DAT_004fe62c+0x3AD4`. `FUN_00454c70` blits this after every type tile.
pub const OVERLAY_51_TILE_MARKER_SPRITE_ID: u16 = 3765;
/// `FUN_00454460` admits `type < 0x25`. `FUN_00454ff0` starts at type 1.
pub const OVERLAY_51_FIRST_OBJECT_TYPE: u32 = 1;
pub const OVERLAY_51_OBJECT_TYPE_LIMIT: u32 = 0x25;
/// `DAT_004fe62c+0x850`. Hidden/time-trophy extras share this sprite.
pub const OVERLAY_51_FLAG_EXTRA_SPRITE_ID: u16 = 532;
/// Overlay-3 local of `DAT_004FE624+0xB0`. Flag 8 offset.
pub const OVERLAY_51_HIDDEN_TROPHY_OFFSET_LOCAL: usize = 31;
/// Overlay-3 local of `DAT_004FE624+0xB4`. Flag 0x20 offset.
pub const OVERLAY_51_TIME_TROPHY_OFFSET_LOCAL: usize = 32;
/// `DAT_004fe62c+0x91c`. `FUN_0047aa20` fill when `(flags & 4) == 0`.
pub const OVERLAY_51_FLAG_BOX_SPRITE_ID: u16 = 583;
/// `DAT_004fe62c+0x3acc`. `FUN_00454520` end-cap when a link word keeps bit 0.
pub const OVERLAY_51_PAIR_CAP_SPRITE_ID: u16 = 3763;
/// `DAT_004fe62c+0x3ad0`. `FUN_00454af0` ribbon between the two tile origins.
pub const OVERLAY_51_PAIR_RIBBON_SPRITE_ID: u16 = 3764;
/// Overlay-3 Section-14 record consumed as `DAT_004fe658+0xc`.
pub const OVERLAY_51_PAIR_GRAPH_RECORD: usize = 3;
/// Header u32s at that record's `+0x10` payload.
pub const OVERLAY_51_PAIR_GRAPH_ROWS: usize = 38;
pub const OVERLAY_51_PAIR_GRAPH_COLUMNS: usize = 7;
/// Overlay-3 S1 local of `DAT_004FE624+0xBC`. High-res value is `(0, 0)`.
pub const OVERLAY_51_LINK_OFFSET_LOCAL: usize = 34;
/// Overlay-2 S0 locals of `*DAT_004fe620` / `DAT_004fe620[1]`.
pub const OVERLAY_51_SPRITE_META_SYSTEM_LEVEL: u32 = 2;

/// One `DAT_004d1204` 8-byte record: S1 global, sprite, next/slot word, pad.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Overlay51TypeRecord {
    pub section1_global: u16,
    pub sprite_id: u16,
    pub next: u16,
}

/// Types `0..=35` from `DAT_004d1204`. Type 0 is the unused head.
pub const OVERLAY_51_TYPE_RECORDS: &[Overlay51TypeRecord] = &[
    Overlay51TypeRecord {
        section1_global: 0,
        sprite_id: 0,
        next: 1,
    },
    Overlay51TypeRecord {
        section1_global: 48,
        sprite_id: 3733,
        next: 2,
    },
    Overlay51TypeRecord {
        section1_global: 49,
        sprite_id: 3734,
        next: 3,
    },
    Overlay51TypeRecord {
        section1_global: 50,
        sprite_id: 3735,
        next: 4,
    },
    Overlay51TypeRecord {
        section1_global: 51,
        sprite_id: 3736,
        next: 5,
    },
    Overlay51TypeRecord {
        section1_global: 52,
        sprite_id: 3737,
        next: 6,
    },
    Overlay51TypeRecord {
        section1_global: 53,
        sprite_id: 3738,
        next: 7,
    },
    Overlay51TypeRecord {
        section1_global: 54,
        sprite_id: 3739,
        next: 8,
    },
    Overlay51TypeRecord {
        section1_global: 55,
        sprite_id: 3740,
        next: 9,
    },
    Overlay51TypeRecord {
        section1_global: 56,
        sprite_id: 3741,
        next: 10,
    },
    Overlay51TypeRecord {
        section1_global: 57,
        sprite_id: 3742,
        next: 11,
    },
    Overlay51TypeRecord {
        section1_global: 58,
        sprite_id: 3743,
        next: 12,
    },
    Overlay51TypeRecord {
        section1_global: 59,
        sprite_id: 3744,
        next: 0,
    },
    Overlay51TypeRecord {
        section1_global: 77,
        sprite_id: 3762,
        next: 14,
    },
    Overlay51TypeRecord {
        section1_global: 60,
        sprite_id: 3745,
        next: 0,
    },
    Overlay51TypeRecord {
        section1_global: 77,
        sprite_id: 3762,
        next: 0,
    },
    Overlay51TypeRecord {
        section1_global: 77,
        sprite_id: 3762,
        next: 0,
    },
    Overlay51TypeRecord {
        section1_global: 77,
        sprite_id: 3762,
        next: 18,
    },
    Overlay51TypeRecord {
        section1_global: 61,
        sprite_id: 3746,
        next: 19,
    },
    Overlay51TypeRecord {
        section1_global: 62,
        sprite_id: 3747,
        next: 20,
    },
    Overlay51TypeRecord {
        section1_global: 63,
        sprite_id: 3748,
        next: 21,
    },
    Overlay51TypeRecord {
        section1_global: 64,
        sprite_id: 3749,
        next: 22,
    },
    Overlay51TypeRecord {
        section1_global: 65,
        sprite_id: 3750,
        next: 23,
    },
    Overlay51TypeRecord {
        section1_global: 66,
        sprite_id: 3751,
        next: 24,
    },
    Overlay51TypeRecord {
        section1_global: 67,
        sprite_id: 3752,
        next: 25,
    },
    Overlay51TypeRecord {
        section1_global: 68,
        sprite_id: 3753,
        next: 26,
    },
    Overlay51TypeRecord {
        section1_global: 69,
        sprite_id: 3754,
        next: 27,
    },
    Overlay51TypeRecord {
        section1_global: 70,
        sprite_id: 3755,
        next: 28,
    },
    Overlay51TypeRecord {
        section1_global: 71,
        sprite_id: 3756,
        next: 0,
    },
    Overlay51TypeRecord {
        section1_global: 77,
        sprite_id: 3762,
        next: 30,
    },
    Overlay51TypeRecord {
        section1_global: 72,
        sprite_id: 3757,
        next: 31,
    },
    Overlay51TypeRecord {
        section1_global: 73,
        sprite_id: 3758,
        next: 0,
    },
    Overlay51TypeRecord {
        section1_global: 77,
        sprite_id: 3762,
        next: 33,
    },
    Overlay51TypeRecord {
        section1_global: 74,
        sprite_id: 3759,
        next: 34,
    },
    Overlay51TypeRecord {
        section1_global: 75,
        sprite_id: 3760,
        next: 35,
    },
    Overlay51TypeRecord {
        section1_global: 76,
        sprite_id: 3761,
        next: 36,
    },
];

/// Unique `(overlay-51 S1 local, sprite id)` pairs from `DAT_004d1204` types
/// 1..=0x24, in first-seen order.
pub const OVERLAY_51_BACKDROP_TILES: &[(usize, u16)] = &[
    (0, 3733),
    (1, 3734),
    (2, 3735),
    (3, 3736),
    (4, 3737),
    (5, 3738),
    (6, 3739),
    (7, 3740),
    (8, 3741),
    (9, 3742),
    (10, 3743),
    (11, 3744),
    (29, 3762),
    (12, 3745),
    (13, 3746),
    (14, 3747),
    (15, 3748),
    (16, 3749),
    (17, 3750),
    (18, 3751),
    (19, 3752),
    (20, 3753),
    (21, 3754),
    (22, 3755),
    (23, 3756),
    (24, 3757),
    (25, 3758),
    (26, 3759),
    (27, 3760),
    (28, 3761),
];

/// `FUN_00454c70` percent scale of one overlay-51 Section-1 point.
pub fn overlay_51_tile_origin(percent: (i16, i16), scale: (i16, i16)) -> (i32, i32) {
    (
        i32::from(percent.0) * i32::from(scale.0) / 100,
        i32::from(percent.1) * i32::from(scale.1) / 100,
    )
}

/// Overlay-51 S1 local index for a `DAT_004d1204` global point.
pub fn overlay_51_s1_local(section1_global: u16) -> Option<usize> {
    usize::try_from(section1_global.checked_sub(OVERLAY_51_S1_BASE)?).ok()
}

/// Campaign slot queried by `FUN_00454ff0` for object `type_id`.
///
/// The builder's `esi` walks type records at `+4`; `[esi-8]` is the previous
/// record's next word. Type 1 therefore reads type 0's `next = 1`.
pub fn overlay_51_campaign_slot(type_id: u32) -> Option<u32> {
    if type_id < OVERLAY_51_FIRST_OBJECT_TYPE || type_id >= OVERLAY_51_OBJECT_TYPE_LIMIT {
        return None;
    }
    OVERLAY_51_TYPE_RECORDS
        .get(type_id as usize - 1)
        .map(|record| u32::from(record.next))
}

/// `FUN_00454ff0` node flags for one overlay-51 object type.
pub fn overlay_51_object_flags(type_id: u32, progress: &PlayerCampaignProgress) -> u32 {
    let slot = overlay_51_campaign_slot(type_id).unwrap_or(0) as usize;
    let bits = progress.control_slot_bits(slot).unwrap_or(0);
    let mut flags = 1u32;
    if bits & 4 != 0 {
        flags = 2;
    }
    if bits & 1 != 0 {
        flags = 2 | 4;
    }
    if bits & 2 != 0 {
        flags |= 8;
    }
    if bits & 8 != 0 {
        flags |= 0x20;
    }
    if progress.current_control_slot() == Some(type_id as usize) {
        flags |= 0x10;
    }
    flags
}

/// Odd `session+0x270 / 100` test from `FUN_00454460`.
pub fn overlay_51_blink_odd(tick: i32) -> bool {
    let sign = tick >> 31;
    let abs = (tick ^ sign).wrapping_sub(sign);
    ((abs & 1) ^ sign) != sign
}

/// Combined `FUN_00454460` walker + `FUN_00454c70` visibility gate.
pub fn overlay_51_object_visible(flags: u32, age_ms: i32) -> bool {
    if flags & 0x10 != 0 && !overlay_51_blink_odd(age_ms / 100) {
        return false;
    }
    (flags & 1) == 0 || (flags & 0x10) != 0
}

/// `FUN_00454c70` draws the sprite-583 quad only when bit 4 is clear.
pub const fn overlay_51_flag_box_visible(flags: u32) -> bool {
    flags & 4 == 0
}

/// `FUN_00454c70` flag-8/0x20 blit origin.
///
/// Center is the type tile origin plus half the tile sprite size; the extra
/// then subtracts half of sprite 532.
pub fn overlay_51_flag_extra_origin(
    tile_origin: (i32, i32),
    tile_size: (u32, u32),
    extra_offset: (i16, i16),
    extra_size: (u32, u32),
) -> (i32, i32) {
    let centered_x = tile_origin.0 + (tile_size.0 as i32 >> 1);
    let centered_y = tile_origin.1 + (tile_size.1 as i32 >> 1);
    (
        centered_x + i32::from(extra_offset.0) - (extra_size.0 as i32 >> 1),
        centered_y + i32::from(extra_offset.1) - (extra_size.1 as i32 >> 1),
    )
}

/// Overlay-3 Section-14 record 3 neighbor matrix (`FUN_00454ff0` `+0x18`).
pub type Overlay51PairGraph = [[u8; OVERLAY_51_PAIR_GRAPH_COLUMNS]; OVERLAY_51_PAIR_GRAPH_ROWS];

/// Decode the 38×7 type-id matrix `FUN_00454ff0` reads from overlay 3 S14[3].
pub fn overlay_51_pair_graph_from_linkage(table: &LinkageTable) -> Option<Overlay51PairGraph> {
    let record = table.records.get(OVERLAY_51_PAIR_GRAPH_RECORD)?;
    let header = record.region1_ptr as usize;
    let payload = record.region3_ptr as usize;
    let width_bytes: [u8; 4] = table.data.get(header..header + 4)?.try_into().ok()?;
    let height_bytes: [u8; 4] = table.data.get(header + 4..header + 8)?.try_into().ok()?;
    let width = u32::from_le_bytes(width_bytes);
    let height = u32::from_le_bytes(height_bytes);
    if width as usize != OVERLAY_51_PAIR_GRAPH_ROWS
        || height as usize != OVERLAY_51_PAIR_GRAPH_COLUMNS
    {
        return None;
    }
    let bytes = table
        .data
        .get(payload..payload + OVERLAY_51_PAIR_GRAPH_ROWS * OVERLAY_51_PAIR_GRAPH_COLUMNS)?;
    let mut graph = [[0u8; OVERLAY_51_PAIR_GRAPH_COLUMNS]; OVERLAY_51_PAIR_GRAPH_ROWS];
    for (row, chunk) in bytes
        .chunks_exact(OVERLAY_51_PAIR_GRAPH_COLUMNS)
        .enumerate()
    {
        graph[row].copy_from_slice(chunk);
    }
    Some(graph)
}

/// `FUN_0042edc0` plus the row-major byte at `dim*(type-1)+column-1`.
pub fn overlay_51_pair_neighbor(
    graph: &Overlay51PairGraph,
    type_id: u32,
    column: u32,
) -> Option<u32> {
    if !(OVERLAY_51_FIRST_OBJECT_TYPE..OVERLAY_51_OBJECT_TYPE_LIMIT).contains(&type_id) {
        return None;
    }
    if !(1..=RETAIL_CONTROL_PAIR_LINK_COLUMNS).contains(&column) {
        return None;
    }
    let other = graph[(type_id as usize) - 1][(column as usize) - 1];
    let other = u32::from(other);
    (other != 0 && other < OVERLAY_51_OBJECT_TYPE_LIMIT).then_some(other)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Overlay51PairEnds {
    left: u32,
    right: u32,
    cap_left: bool,
    cap_right: bool,
}

/// `FUN_00454ff0`'s `DAT_004fe5f0` builder: one undirected link per recovered
/// neighbor byte, with cap bits matching the first-seen / reverse-visit OR.
pub fn overlay_51_pair_links(
    graph: &Overlay51PairGraph,
    progress: &PlayerCampaignProgress,
) -> Vec<(u32, u32, bool, bool)> {
    let mut links: Vec<Overlay51PairEnds> = Vec::new();
    for type_id in OVERLAY_51_FIRST_OBJECT_TYPE..OVERLAY_51_OBJECT_TYPE_LIMIT {
        let flags = overlay_51_object_flags(type_id, progress);
        if flags & 2 == 0 {
            continue;
        }
        let Some(slot) = overlay_51_campaign_slot(type_id) else {
            continue;
        };
        for column in 1..=RETAIL_CONTROL_PAIR_LINK_COLUMNS {
            if !progress.control_slot_pair_neighbor(slot as usize, column) {
                continue;
            }
            let Some(other) = overlay_51_pair_neighbor(graph, type_id, column) else {
                continue;
            };
            if let Some(existing) = links.iter_mut().find(|link| {
                (link.left == type_id && link.right == other)
                    || (link.left == other && link.right == type_id)
            }) {
                if existing.left == type_id {
                    existing.cap_right = true;
                } else if existing.right == type_id {
                    existing.cap_left = true;
                }
                continue;
            }
            links.push(Overlay51PairEnds {
                left: type_id,
                right: other,
                cap_left: false,
                cap_right: true,
            });
        }
    }
    links
        .into_iter()
        .map(|link| (link.left, link.right, link.cap_left, link.cap_right))
        .collect()
}

fn overlay_51_q31_component(component: i32, length: i32) -> i32 {
    if component.wrapping_abs() < length.wrapping_abs() {
        ((i64::from(component) << 31) / i64::from(length)) as i32
    } else {
        (component ^ length) | i32::MAX
    }
}

/// `FUN_00457960` on a 2D delta with a zero Z (the pair-link call site).
fn overlay_51_normalize_delta(dx: i32, dy: i32) -> (i32, i32) {
    let mut vector = [dx, dy, 0];
    let mut magnitude_or = vector.into_iter().fold(0i32, |combined, component| {
        combined | component.wrapping_abs()
    });
    while magnitude_or >= 0x6883 {
        magnitude_or >>= 1;
        for component in &mut vector {
            *component >>= 1;
        }
    }
    let length_squared = vector.into_iter().fold(0u32, |sum, component| {
        sum.wrapping_add(component.wrapping_mul(component) as u32)
    });
    let length = retail_integer_sqrt(length_squared as i32).wrapping_add(1) as i32;
    (
        overlay_51_q31_component(vector[0], length),
        overlay_51_q31_component(vector[1], length),
    )
}

fn overlay_51_q31_perp(component: i32, span: i32, sprite_meta: i32) -> i32 {
    let product = i64::from(component).wrapping_mul(i64::from(span));
    let high = (product >> 32) as i32;
    let sign = if product < 0 { 1 } else { 0 };
    (high << 1 | sign).wrapping_mul(sprite_meta) >> 8
}

fn overlay_51_rotate_q15(
    x: i32,
    y: i32,
    sin_q15: i16,
    cos_q15: i16,
    sprite_meta: (i32, i32),
) -> (i32, i32) {
    let x_out = ((x.wrapping_mul(i32::from(cos_q15)) >> 15)
        .wrapping_sub(y.wrapping_mul(i32::from(sin_q15)) >> 15))
    .wrapping_mul(sprite_meta.0)
        >> 8;
    let y_out = ((x.wrapping_mul(i32::from(sin_q15)) >> 15)
        .wrapping_add(y.wrapping_mul(i32::from(cos_q15)) >> 15))
    .wrapping_mul(sprite_meta.1)
        >> 8;
    (x_out, y_out)
}

/// `FUN_00454af0` ribbon corners, still in authored overlay-51 pixels.
pub fn overlay_51_ribbon_corners(
    a: (i32, i32),
    b: (i32, i32),
    span: i32,
    sprite_meta: (i32, i32),
) -> [(i32, i32); 4] {
    let (nx, ny) = overlay_51_normalize_delta(b.0.wrapping_sub(a.0), b.1.wrapping_sub(a.1));
    let width = span.wrapping_mul(8);
    let px = overlay_51_q31_perp(ny, width, sprite_meta.0);
    let py = overlay_51_q31_perp(nx, width, sprite_meta.1);
    [
        (
            a.0.wrapping_sub(px).wrapping_div(span),
            a.1.wrapping_add(py).wrapping_div(span),
        ),
        (
            b.0.wrapping_sub(px).wrapping_div(span),
            b.1.wrapping_add(py).wrapping_div(span),
        ),
        (
            b.0.wrapping_add(px).wrapping_div(span),
            b.1.wrapping_sub(py).wrapping_div(span),
        ),
        (
            a.0.wrapping_add(px).wrapping_div(span),
            a.1.wrapping_sub(py).wrapping_div(span),
        ),
    ]
}

fn overlay_51_cap_corners(
    origin: (i32, i32),
    sin_q15: i16,
    cos_q15: i16,
    span: i32,
    sprite_meta: (i32, i32),
) -> [(i32, i32); 4] {
    let neg = span.wrapping_mul(-0xd);
    let pos = span.wrapping_mul(0xd);
    let offsets = [(neg, neg), (pos, neg), (pos, pos), (neg, pos)];
    let rotated: [(i32, i32); 4] =
        offsets.map(|(x, y)| overlay_51_rotate_q15(x, y, sin_q15, cos_q15, sprite_meta));
    [
        (
            rotated[0].0.wrapping_add(origin.0).wrapping_div(span),
            rotated[0].1.wrapping_add(origin.1).wrapping_div(span),
        ),
        (
            rotated[1].0.wrapping_add(origin.0).wrapping_div(span),
            rotated[1].1.wrapping_add(origin.1).wrapping_div(span),
        ),
        (
            rotated[2].0.wrapping_add(origin.0).wrapping_div(span),
            rotated[2].1.wrapping_add(origin.1).wrapping_div(span),
        ),
        (
            rotated[3].0.wrapping_add(origin.0).wrapping_div(span),
            rotated[3].1.wrapping_add(origin.1).wrapping_div(span),
        ),
    ]
}

/// `FUN_00454520` endpoints and quads for one `DAT_004fe5f0` node.
pub fn overlay_51_pair_link_geometry(
    left_origin: (i32, i32),
    right_origin: (i32, i32),
    marker_size: (u32, u32),
    link_offset: (i16, i16),
    sprite_meta: (i32, i32),
    cap_left: bool,
    cap_right: bool,
) -> Overlay51PairLinkGeometry {
    let half_w = marker_size.0 as i32 >> 1;
    let half_h = marker_size.1 as i32 >> 1;
    let x_mul = half_w.wrapping_add((sprite_meta.0 << 1) >> 8);
    let y_mul = half_h.wrapping_add(sprite_meta.1 >> 8);
    let ox = i32::from(link_offset.0);
    let oy = i32::from(link_offset.1);
    let ax_s = (half_w + left_origin.0 + ox).wrapping_mul(y_mul);
    let ay_s = (half_h + left_origin.1 + oy).wrapping_mul(x_mul);
    let bx_s = (half_w + right_origin.0 + ox).wrapping_mul(y_mul);
    let by_s = (half_h + right_origin.1 + oy).wrapping_mul(x_mul);
    let span = y_mul.wrapping_mul(x_mul);
    if span == 0 {
        return Overlay51PairLinkGeometry {
            ribbon: [(0, 0); 4],
            cap_left: None,
            cap_right: None,
        };
    }
    let (nx, ny) = overlay_51_normalize_delta(bx_s.wrapping_sub(ax_s), by_s.wrapping_sub(ay_s));
    let nx_q15 = (nx >> 16) as i16;
    let ny_q15 = (ny >> 16) as i16;
    let inset_x = (span.wrapping_mul(i32::from(nx_q15))) >> 15;
    let inset_y = (span.wrapping_mul(i32::from(ny_q15))) >> 15;
    let mut ax = (ax_s.wrapping_add(inset_x)).wrapping_mul(x_mul);
    let mut ay = (ay_s.wrapping_add(inset_y)).wrapping_mul(y_mul);
    let mut bx = (bx_s.wrapping_sub(inset_x)).wrapping_mul(x_mul);
    let mut by = (by_s.wrapping_sub(inset_y)).wrapping_mul(y_mul);
    let (nx2, ny2) = overlay_51_normalize_delta(bx.wrapping_sub(ax), by.wrapping_sub(ay));
    let nx2_q15 = (nx2 >> 16) as i16;
    let ny2_q15 = (ny2 >> 16) as i16;
    let half = span.wrapping_mul(6) >> 1;
    let cap_dx = ((i32::from(nx2_q15).wrapping_mul(half)) >> 15).wrapping_mul(sprite_meta.0) >> 8;
    let cap_dy = ((i32::from(ny2_q15).wrapping_mul(half)) >> 15).wrapping_mul(sprite_meta.1) >> 8;
    if cap_left {
        ax = ax.wrapping_add(cap_dx);
        ay = ay.wrapping_add(cap_dy);
    }
    if cap_right {
        bx = bx.wrapping_sub(cap_dx);
        by = by.wrapping_sub(cap_dy);
    }
    Overlay51PairLinkGeometry {
        ribbon: overlay_51_ribbon_corners((ax, ay), (bx, by), span, sprite_meta),
        cap_left: cap_left
            .then(|| overlay_51_cap_corners((ax, ay), -ny2_q15, -nx2_q15, span, sprite_meta)),
        cap_right: cap_right
            .then(|| overlay_51_cap_corners((bx, by), ny2_q15, nx2_q15, span, sprite_meta)),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Overlay51PairLinkGeometry {
    pub ribbon: [(i32, i32); 4],
    pub cap_left: Option<[(i32, i32); 4]>,
    pub cap_right: Option<[(i32, i32); 4]>,
}

#[derive(Debug, Clone)]
pub struct Overlay51BackdropTile {
    pub local_index: usize,
    pub sprite_id: u16,
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub origin: (i32, i32),
    pub blend: WorldSpriteBlend,
}

#[derive(Debug, Clone)]
pub struct Overlay51BackdropMarker {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub blend: WorldSpriteBlend,
}

#[derive(Debug, Clone)]
pub struct Overlay51FlagExtra {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub blend: WorldSpriteBlend,
    pub hidden_offset: (i16, i16),
    pub time_offset: (i16, i16),
}

#[derive(Debug, Clone, Copy)]
pub struct Overlay51VisibleObject<'a> {
    pub type_id: u32,
    pub flags: u32,
    pub tile: &'a Overlay51BackdropTile,
}

#[derive(Debug, Clone)]
pub struct Overlay51PairLinkDraw<'a> {
    pub ribbon: &'a Overlay51BackdropMarker,
    pub cap: Option<&'a Overlay51BackdropMarker>,
    pub geometry: Overlay51PairLinkGeometry,
}

#[derive(Debug, Clone)]
pub struct Overlay51Backdrop {
    tiles: Vec<Overlay51BackdropTile>,
    marker: Overlay51BackdropMarker,
    flag_extra: Option<Overlay51FlagExtra>,
    flag_box: Option<Overlay51BackdropMarker>,
    pair_ribbon: Option<Overlay51BackdropMarker>,
    pair_cap: Option<Overlay51BackdropMarker>,
    pair_graph: Option<Overlay51PairGraph>,
    sprite_meta: (i32, i32),
    link_offset: (i16, i16),
}

fn decode_overlay_51_sprite(
    cache: &ResourceCache,
    sprite_id: u16,
) -> Option<(Vec<u8>, u32, u32, WorldSpriteBlend)> {
    let (atlas, entry) = cache.global_sprite(sprite_id)?;
    let flags = entry.pal_size as u8;
    let decoded = atlas
        .decode_sprite(entry, usize::from(sprite_flat_shade_row(flags)))
        .ok()?;
    Some((
        decoded.rgba,
        decoded.width as u32,
        decoded.height as u32,
        sprite_blend(flags),
    ))
}

impl Overlay51Backdrop {
    pub fn from_cache(cache: &ResourceCache) -> Option<Self> {
        let scale = cache.system_layout_point(3, OVERLAY_51_SCALE_LOCAL_INDEX)?;
        let mut tiles = Vec::with_capacity(OVERLAY_51_BACKDROP_TILES.len());
        for &(local_index, sprite_id) in OVERLAY_51_BACKDROP_TILES {
            let percent = cache.system_layout_point(OVERLAY_51_SYSTEM_LEVEL, local_index)?;
            let (rgba, width, height, blend) = decode_overlay_51_sprite(cache, sprite_id)?;
            tiles.push(Overlay51BackdropTile {
                local_index,
                sprite_id,
                rgba,
                width,
                height,
                origin: overlay_51_tile_origin(percent, scale),
                blend,
            });
        }
        let (rgba, width, height, blend) =
            decode_overlay_51_sprite(cache, OVERLAY_51_TILE_MARKER_SPRITE_ID)?;
        let flag_extra = decode_overlay_51_sprite(cache, OVERLAY_51_FLAG_EXTRA_SPRITE_ID).and_then(
            |(rgba, width, height, blend)| {
                Some(Overlay51FlagExtra {
                    rgba,
                    width,
                    height,
                    blend,
                    hidden_offset: cache
                        .system_layout_point(3, OVERLAY_51_HIDDEN_TROPHY_OFFSET_LOCAL)?,
                    time_offset: cache
                        .system_layout_point(3, OVERLAY_51_TIME_TROPHY_OFFSET_LOCAL)?,
                })
            },
        );
        let flag_box = decode_overlay_51_sprite(cache, OVERLAY_51_FLAG_BOX_SPRITE_ID).map(
            |(rgba, width, height, blend)| Overlay51BackdropMarker {
                rgba,
                width,
                height,
                blend,
            },
        );
        let pair_ribbon = decode_overlay_51_sprite(cache, OVERLAY_51_PAIR_RIBBON_SPRITE_ID).map(
            |(rgba, width, height, blend)| Overlay51BackdropMarker {
                rgba,
                width,
                height,
                blend,
            },
        );
        let pair_cap = decode_overlay_51_sprite(cache, OVERLAY_51_PAIR_CAP_SPRITE_ID).map(
            |(rgba, width, height, blend)| Overlay51BackdropMarker {
                rgba,
                width,
                height,
                blend,
            },
        );
        let pair_graph = cache
            .system_linkage(3)
            .and_then(overlay_51_pair_graph_from_linkage);
        let sprite_meta = match (
            cache.system_data_value(OVERLAY_51_SPRITE_META_SYSTEM_LEVEL, 0),
            cache.system_data_value(OVERLAY_51_SPRITE_META_SYSTEM_LEVEL, 1),
        ) {
            (Some(x), Some(y)) => (x as i32, y as i32),
            _ => (0, 0),
        };
        let link_offset = cache
            .system_layout_point(3, OVERLAY_51_LINK_OFFSET_LOCAL)
            .unwrap_or((0, 0));
        Some(Self {
            tiles,
            marker: Overlay51BackdropMarker {
                rgba,
                width,
                height,
                blend,
            },
            flag_extra,
            flag_box,
            pair_ribbon,
            pair_cap,
            pair_graph,
            sprite_meta,
            link_offset,
        })
    }

    pub fn tiles(&self) -> &[Overlay51BackdropTile] {
        &self.tiles
    }

    pub fn marker(&self) -> &Overlay51BackdropMarker {
        &self.marker
    }

    pub fn flag_extra(&self) -> Option<&Overlay51FlagExtra> {
        self.flag_extra.as_ref()
    }

    pub fn flag_box(&self) -> Option<&Overlay51BackdropMarker> {
        self.flag_box.as_ref()
    }

    pub fn tile_for_local(&self, local_index: usize) -> Option<&Overlay51BackdropTile> {
        self.tiles
            .iter()
            .find(|tile| tile.local_index == local_index)
    }

    /// `FUN_00454460` live-list visit order: types `1..0x24` with a sprite.
    pub fn visible_objects<'a>(
        &'a self,
        progress: &'a PlayerCampaignProgress,
        age_ms: i32,
    ) -> impl Iterator<Item = Overlay51VisibleObject<'a>> + 'a {
        (OVERLAY_51_FIRST_OBJECT_TYPE..OVERLAY_51_OBJECT_TYPE_LIMIT).filter_map(move |type_id| {
            let record = OVERLAY_51_TYPE_RECORDS.get(type_id as usize)?;
            if record.sprite_id == 0 {
                return None;
            }
            let flags = overlay_51_object_flags(type_id, progress);
            if !overlay_51_object_visible(flags, age_ms) {
                return None;
            }
            let local = overlay_51_s1_local(record.section1_global)?;
            let tile = self.tile_for_local(local)?;
            Some(Overlay51VisibleObject {
                type_id,
                flags,
                tile,
            })
        })
    }

    /// `FUN_00454460`'s second walk: `DAT_004fe5f0` pair-links.
    pub fn visible_pair_links<'a>(
        &'a self,
        progress: &'a PlayerCampaignProgress,
    ) -> Vec<Overlay51PairLinkDraw<'a>> {
        let Some(graph) = self.pair_graph.as_ref() else {
            return Vec::new();
        };
        let Some(ribbon) = self.pair_ribbon.as_ref() else {
            return Vec::new();
        };
        if self.sprite_meta == (0, 0) {
            return Vec::new();
        }
        overlay_51_pair_links(graph, progress)
            .into_iter()
            .filter_map(|(left, right, cap_left, cap_right)| {
                let left_record = OVERLAY_51_TYPE_RECORDS.get(left as usize)?;
                let right_record = OVERLAY_51_TYPE_RECORDS.get(right as usize)?;
                let left_local = overlay_51_s1_local(left_record.section1_global)?;
                let right_local = overlay_51_s1_local(right_record.section1_global)?;
                let left_tile = self.tile_for_local(left_local)?;
                let right_tile = self.tile_for_local(right_local)?;
                Some(Overlay51PairLinkDraw {
                    ribbon,
                    cap: self.pair_cap.as_ref(),
                    geometry: overlay_51_pair_link_geometry(
                        left_tile.origin,
                        right_tile.origin,
                        (self.marker.width, self.marker.height),
                        self.link_offset,
                        self.sprite_meta,
                        cap_left,
                        cap_right,
                    ),
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backdrop_table_covers_each_overlay_51_tile_once() {
        assert_eq!(OVERLAY_51_BACKDROP_TILES.len(), 30);
        let mut sprites: Vec<u16> = OVERLAY_51_BACKDROP_TILES
            .iter()
            .map(|(_, sprite)| *sprite)
            .collect();
        sprites.sort_unstable();
        assert_eq!(
            sprites,
            (OVERLAY_51_FIRST_SPRITE_ID..=OVERLAY_51_LAST_TILE_SPRITE_ID).collect::<Vec<_>>()
        );
        let mut locals: Vec<usize> = OVERLAY_51_BACKDROP_TILES
            .iter()
            .map(|(local, _)| *local)
            .collect();
        locals.sort_unstable();
        assert_eq!(locals, (0..30).collect::<Vec<_>>());
    }

    #[test]
    fn percent_scale_uses_truncating_fun_00454c70_division() {
        assert_eq!(overlay_51_tile_origin((19, 4), (640, 432)), (121, 17));
        assert_eq!(overlay_51_tile_origin((1, 11), (320, 216)), (3, 23));
        assert_eq!(overlay_51_tile_origin((87, 88), (640, 432)), (556, 380));
    }

    #[test]
    fn tile_marker_is_dat_004fe62c_sprite_3765() {
        assert_eq!(OVERLAY_51_TILE_MARKER_SPRITE_ID, 3765);
        assert_eq!(0x3ad4 / 4, u32::from(OVERLAY_51_TILE_MARKER_SPRITE_ID));
    }

    #[test]
    fn dat_004d1204_type_one_is_peasant_slot_one() {
        assert_eq!(OVERLAY_51_TYPE_RECORDS.len(), 36);
        assert_eq!(overlay_51_campaign_slot(1), Some(1));
        assert_eq!(OVERLAY_51_TYPE_RECORDS[1].sprite_id, 3733);
        assert_eq!(overlay_51_s1_local(48), Some(0));
        assert_eq!(overlay_51_campaign_slot(2), Some(2));
        assert_eq!(overlay_51_campaign_slot(13), Some(0));
        assert_eq!(overlay_51_campaign_slot(14), Some(14));
        assert_eq!(overlay_51_campaign_slot(0), None);
        assert_eq!(overlay_51_campaign_slot(0x25), None);
    }

    #[test]
    fn unsaved_worlds_stay_hidden_and_the_current_slot_blinks() {
        let mut progress = PlayerCampaignProgress::new();
        progress.set_current_control_slot(Some(1));
        let unsaved = overlay_51_object_flags(1, &progress);
        assert_eq!(unsaved, 0x11);
        assert!(!overlay_51_object_visible(unsaved, 0));
        assert!(overlay_51_object_visible(unsaved, 100));
        assert!(!overlay_51_object_visible(unsaved, 200));
        assert!(!overlay_51_object_visible(
            overlay_51_object_flags(2, &progress),
            100
        ));
    }

    #[test]
    fn saved_current_world_blinks_and_other_saved_worlds_stay_on() {
        let mut progress = PlayerCampaignProgress::new();
        progress.set_current_control_slot(Some(1));
        assert_eq!(progress.mark_current_world_saved(), Ok(true));
        progress.set_current_control_slot(Some(2));
        assert_eq!(progress.mark_current_world_saved(), Ok(true));
        progress.set_current_control_slot(Some(1));

        let peasant = overlay_51_object_flags(1, &progress);
        assert_eq!(peasant, 0x16);
        assert!(!overlay_51_object_visible(peasant, 0));
        assert!(overlay_51_object_visible(peasant, 100));

        let second = overlay_51_object_flags(2, &progress);
        assert_eq!(second, 6);
        assert!(overlay_51_object_visible(second, 0));
        assert!(overlay_51_object_visible(second, 100));
        assert!(overlay_51_flag_box_visible(0x11));
        assert!(!overlay_51_flag_box_visible(peasant));
        assert!(!overlay_51_flag_box_visible(second));
    }

    #[test]
    fn blink_odd_matches_fun_00454460_signed_abs_test() {
        assert!(!overlay_51_blink_odd(0));
        assert!(overlay_51_blink_odd(1));
        assert!(!overlay_51_blink_odd(2));
        assert!(overlay_51_blink_odd(-1));
        assert!(!overlay_51_blink_odd(-2));
    }

    #[test]
    fn flag_eight_and_0x20_mark_hidden_and_time_trophies() {
        let mut progress = PlayerCampaignProgress::new();
        progress.set_current_control_slot(Some(1));
        assert_eq!(progress.mark_current_world_saved(), Ok(true));
        assert_eq!(overlay_51_object_flags(1, &progress) & 8, 0);
        assert_eq!(overlay_51_object_flags(1, &progress) & 0x20, 0);
        assert_eq!(progress.claim_current_time_trophy(), Ok(true));
        let flags = overlay_51_object_flags(1, &progress);
        assert_eq!(flags & 0x20, 0x20);
        assert_eq!(flags & 8, 0);
    }

    #[test]
    fn flag_box_sprite_is_dat_004fe62c_sprite_583() {
        assert_eq!(OVERLAY_51_FLAG_BOX_SPRITE_ID, 583);
        assert_eq!(0x91c / 4, u32::from(OVERLAY_51_FLAG_BOX_SPRITE_ID));
        assert!(overlay_51_flag_box_visible(1));
        assert!(overlay_51_flag_box_visible(0x11));
        assert!(!overlay_51_flag_box_visible(6));
        assert!(!overlay_51_flag_box_visible(0x16));
    }

    #[test]
    fn flag_extra_origin_centers_sprite_532_on_overlay3_s1_offset() {
        assert_eq!(0x850 / 4, u32::from(OVERLAY_51_FLAG_EXTRA_SPRITE_ID));
        assert_eq!(
            overlay_51_flag_extra_origin((121, 17), (128, 48), (-27, -14), (12, 12)),
            (152, 21)
        );
        assert_eq!(
            overlay_51_flag_extra_origin((121, 17), (128, 48), (27, -14), (12, 12)),
            (206, 21)
        );
    }

    #[test]
    fn pair_sprites_are_dat_004fe62c_3763_and_3764() {
        assert_eq!(OVERLAY_51_PAIR_CAP_SPRITE_ID, 3763);
        assert_eq!(OVERLAY_51_PAIR_RIBBON_SPRITE_ID, 3764);
        assert_eq!(0x3acc / 4, u32::from(OVERLAY_51_PAIR_CAP_SPRITE_ID));
        assert_eq!(0x3ad0 / 4, u32::from(OVERLAY_51_PAIR_RIBBON_SPRITE_ID));
    }

    #[test]
    fn pair_graph_row_zero_lists_types_eighteen_and_two() {
        let mut graph = [[0u8; OVERLAY_51_PAIR_GRAPH_COLUMNS]; OVERLAY_51_PAIR_GRAPH_ROWS];
        graph[0] = [18, 2, 0, 0, 0, 0, 0];
        graph[1] = [3, 18, 0, 0, 0, 0, 0];
        assert_eq!(overlay_51_pair_neighbor(&graph, 1, 1), Some(18));
        assert_eq!(overlay_51_pair_neighbor(&graph, 1, 2), Some(2));
        assert_eq!(overlay_51_pair_neighbor(&graph, 1, 3), None);
        assert_eq!(overlay_51_pair_neighbor(&graph, 2, 1), Some(3));
    }

    #[test]
    fn pair_links_stay_closed_until_fun_0042edc0_bits_are_set() {
        let mut graph = [[0u8; OVERLAY_51_PAIR_GRAPH_COLUMNS]; OVERLAY_51_PAIR_GRAPH_ROWS];
        graph[0] = [18, 2, 0, 0, 0, 0, 0];
        let mut progress = PlayerCampaignProgress::new();
        progress.set_current_control_slot(Some(1));
        assert_eq!(progress.mark_current_world_saved(), Ok(true));
        assert!(overlay_51_pair_links(&graph, &progress).is_empty());
        assert_eq!(progress.set_pair_neighbor_bit(1, 1), Ok(true));
        assert_eq!(
            overlay_51_pair_links(&graph, &progress),
            vec![(1, 18, false, true)]
        );
        assert_eq!(progress.set_pair_neighbor_bit(1, 2), Ok(true));
        assert_eq!(
            overlay_51_pair_links(&graph, &progress),
            vec![(1, 18, false, true), (1, 2, false, true)]
        );
    }

    #[test]
    fn reverse_visit_ors_the_first_cap_bit() {
        let mut graph = [[0u8; OVERLAY_51_PAIR_GRAPH_COLUMNS]; OVERLAY_51_PAIR_GRAPH_ROWS];
        graph[0] = [2, 0, 0, 0, 0, 0, 0];
        graph[1] = [1, 0, 0, 0, 0, 0, 0];
        let mut progress = PlayerCampaignProgress::new();
        progress.set_current_control_slot(Some(1));
        assert_eq!(progress.mark_current_world_saved(), Ok(true));
        assert_eq!(progress.set_pair_neighbor_bit(1, 1), Ok(true));
        progress.set_current_control_slot(Some(2));
        assert_eq!(progress.mark_current_world_saved(), Ok(true));
        assert_eq!(progress.set_pair_neighbor_bit(2, 1), Ok(true));
        assert_eq!(
            overlay_51_pair_links(&graph, &progress),
            vec![(1, 2, true, true)]
        );
    }

    #[test]
    fn pair_ribbon_quad_is_non_degenerate_for_peasant_to_type_two() {
        let geometry = overlay_51_pair_link_geometry(
            (121, 17),
            (6, 47),
            (66, 50),
            (0, 0),
            (512, 512),
            false,
            true,
        );
        let [a, b, c, d] = geometry.ribbon;
        assert_ne!(a, b);
        assert_ne!(b, c);
        assert!(geometry.cap_left.is_none());
        assert!(geometry.cap_right.is_some());
        let _ = d;
    }
}
