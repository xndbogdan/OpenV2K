//! Retail `0x22` model-edge sprite quads (recovered near-pass construction).
//!
//! `FUN_00459000` resolves two near-table endpoints (i16 screen pixels plus an
//! 8.8 raw depth), builds a tapered screen-space quad around them, outcode
//! clips through `DAT_004c5268`, and queues one `FUN_0047AA20` primitive whose
//! `+0x1098` device slot is `FUN_0047EF10`. The corner arithmetic below
//! reproduces that construction (integer sqrt, Q15 direction, truncating
//! divisions, i16 width casts). Inputs arrive from the port's float projection
//! rounded to whole pixels, matching the near-table shorts.
//!
//! Depth is retail's **8.8 raw domain** (world × 256), the same dwords the
//! near table stores and `FUN_004594C0` divides by. The `0x40` raw near clip
//! (`FUN_0046CD90`), the `0x3F` midpoint width guard, the `perp == 0 → 1`
//! hairline, and the `0x2000` halving cap are reproduced here. The fill is the
//! same textured-quad policy already used for 0x78 billboards and 0x83 faces:
//! implicit UVs `(0,0)`, `(Umax,0)`, `(Umax,Vmax)`, `(0,Vmax)` and Section-3
//! blend flags. Live GL paints those pixel corners in a HUD-style ortho
//! (`glOrtho(0,W,H,0,-1,1)`, Y-down) with window Z matched to the 3D
//! perspective pass. Palette-style `0x02` hairlines use `FUN_00458c60`
//! (two endpoints, same LUT) and `LAB_0047a740` / device `+0x1024`.

use crate::projection::{
    retail_projected_point, NativeScreenProjection, ProjectionCompatibilityReason,
    ProjectionEffect, SceneProjectionAuthority,
};
use crate::renderer::{FaceMaterial, WorldSpriteBlend};
use v2k_formats::models::{ModelEdgeEndpointSnapshot, ModelSlotClip};

/// Camera projection values needed to move between pixels and view units.
///
/// `scale_x/scale_y` map view units at unit depth to pixels (`m00*W/2`,
/// `m11*H/2` from the camera's projection matrix); `center_x/center_y` are the
/// corresponding principal points in top-down pixel coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenIntrinsics {
    pub width_px: i32,
    pub height_px: i32,
    pub scale_x: f32,
    pub scale_y: f32,
    pub center_x: f32,
    pub center_y: f32,
    /// Camera clip planes used to match screen-fill window Z to the 3D pass.
    pub clip_near: f32,
    pub clip_far: f32,
    /// Scene-owned integer screen phase, applied once before construction.
    pub projection_effect: ProjectionEffect,
}

impl ScreenIntrinsics {
    /// Rebuild the screen-space projection terms retained by the GL backend.
    ///
    /// `Camera::projection_matrix` stores the offsets in the Z column. For a
    /// point in front of the eye (`view_z = -depth`), that moves the
    /// top-origin optical centre by `(-off_x, +off_y)`. Keeping this conversion
    /// here gives every edge consumer the same sign convention as the 3-D
    /// projection matrix instead of duplicating it at backend call sites.
    pub fn from_projection_terms(
        logical_size: [u32; 2],
        projection_terms: [f32; 4],
        clip_planes: [f32; 2],
    ) -> Option<Self> {
        let [logical_width, logical_height] = logical_size;
        let width_px = i32::try_from(logical_width).ok()?;
        let height_px = i32::try_from(logical_height).ok()?;
        if width_px <= 0 || height_px <= 0 {
            return None;
        }
        let [m00, m11, off_x, off_y] = projection_terms;
        let width = logical_width as f32;
        let height = logical_height as f32;
        Some(Self {
            width_px,
            height_px,
            scale_x: m00 * width * 0.5,
            scale_y: m11 * height * 0.5,
            center_x: width * 0.5 * (1.0 - off_x),
            center_y: height * 0.5 * (1.0 + off_y),
            clip_near: clip_planes[0],
            clip_far: clip_planes[1],
            projection_effect: ProjectionEffect::None,
        })
    }

    /// Project one world point to integer screen pixels plus forward depth.
    ///
    /// `basis_rows` are the retained view basis rows (right, up, -forward).
    /// Depth is returned in retail's 8.8 raw domain (world × 256, rounded).
    /// `FUN_0046CD90` sets clip bit `0x40` when that raw depth is `< 0x40`;
    /// `FUN_00459000` then drops the whole record, so this returns `None`.
    /// The logical-viewport adapter's dry GL bodies use uncapped perspective.
    /// Their edges must use the same pixels: independently halving far-off
    /// endpoints can taper an invisible edge back into a large viewport.
    /// Other authorities retain the retail cap, as does underwater projection
    /// whose GL vertex shader applies that cap before the scene phase.
    pub fn project(
        &self,
        world: [f32; 3],
        cam_pos: [f32; 3],
        basis_rows: [[f32; 3]; 3],
        authority: SceneProjectionAuthority,
    ) -> Option<(i32, i32, i32)> {
        let delta = [
            world[0] - cam_pos[0],
            world[1] - cam_pos[1],
            world[2] - cam_pos[2],
        ];
        let depth = -(dot3(basis_rows[2], delta));
        let raw_depth = (depth * 256.0).round() as i32;
        if raw_depth < 0x40 {
            return None;
        }
        let vx = dot3(basis_rows[0], delta);
        let vy = dot3(basis_rows[1], delta);
        let sx = self.center_x + self.scale_x * vx / depth;
        let sy = self.center_y - self.scale_y * vy / depth;
        let screen = (sx as i32, sy as i32);
        if authority
            == SceneProjectionAuthority::Compatibility(
                ProjectionCompatibilityReason::LogicalViewportAdapter,
            )
            && self.projection_effect == ProjectionEffect::None
        {
            Some((screen.0, screen.1, raw_depth))
        } else {
            let screen = retail_projected_point(screen.0, screen.1, self.projection_effect);
            Some((screen.x, screen.y, raw_depth))
        }
    }

    /// Place a screen pixel at 8.8 raw depth `d` back into world space.
    pub fn unproject(
        &self,
        sx: f32,
        sy: f32,
        d: f32,
        cam_pos: [f32; 3],
        basis_rows: [[f32; 3]; 3],
    ) -> [f32; 3] {
        let Some(view) = self.view_from_screen(sx, sy, d as i32) else {
            return cam_pos;
        };
        let fwd = neg(basis_rows[2]);
        let d = -view[2];
        [
            cam_pos[0] + basis_rows[0][0] * view[0] + basis_rows[1][0] * view[1] + fwd[0] * d,
            cam_pos[1] + basis_rows[0][1] * view[0] + basis_rows[1][1] * view[1] + fwd[1] * d,
            cam_pos[2] + basis_rows[0][2] * view[0] + basis_rows[1][2] * view[1] + fwd[2] * d,
        ]
    }

    /// Invert [`Self::project`] into OpenGL view space.
    ///
    /// Live fill no longer uses this: `FUN_0047EF10` paints constructor pixels
    /// in a HUD-style ortho. The inverse remains the round-trip check that
    /// [`Self::project`] and the gameplay projection matrix agree, and it
    /// still feeds [`Self::unproject`]. `z` is `-world_depth`.
    pub fn view_from_screen(&self, sx: f32, sy: f32, raw_depth: i32) -> Option<[f32; 3]> {
        if self.scale_x == 0.0 || self.scale_y == 0.0 {
            return None;
        }
        let d = raw_depth as f32 / 256.0;
        Some([
            (sx - self.center_x) * d / self.scale_x,
            (self.center_y - sy) * d / self.scale_y,
            -d,
        ])
    }

    /// Perspective `ndc.z` for an 8.8 raw depth, matching `Camera::projection_matrix`.
    pub fn ndc_z_from_raw_depth(self, raw_depth: i32) -> f32 {
        let near = if self.clip_near > 0.0 {
            self.clip_near
        } else {
            0.1
        };
        let far = if self.clip_far > near {
            self.clip_far
        } else {
            500.0
        };
        let d = (raw_depth as f32 / 256.0).max(near);
        let vz = -d;
        let nf = 1.0 / (near - far);
        let clip_z = (far + near) * nf * vz + 2.0 * far * near * nf;
        clip_z / d
    }

    /// Eye Z for `glOrtho(..., -1, 1)` so `ndc.z` matches the 3D perspective pass.
    pub fn ortho_fill_z(self, raw_depth: i32) -> f32 {
        -self.ndc_z_from_raw_depth(raw_depth)
    }
}

/// Implicit UVs for `FUN_0047EF10` on stored corners A, B, C, D.
///
/// Retail's payload order is A, C, D, B with `(0,0)`, `(Umax,0)`,
/// `(Umax,Vmax)`, `(0,Vmax)`. On the constructor's A/B/C/D that is A `(0,0)`,
/// B `(0,1)`, C `(1,0)`, D `(1,1)`.
pub const EDGE_QUAD_UVS: [[f32; 2]; 4] = [[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]];

/// Triangle split matching `FUN_0047EF10`: A-C-D and A-D-B.
///
/// `[0,1,2, 0,2,3]` would split along A-C and leave a hole along the −perp
/// edge B-D.
pub const EDGE_QUAD_TRIANGLES: [[usize; 3]; 2] = [[0, 2, 3], [0, 3, 1]];

/// Missing-material fallback: white, untextured, masked, brightest flat row.
const FALLBACK_EDGE_MATERIAL: FaceMaterial = FaceMaterial {
    color: [1.0, 1.0, 1.0],
    palette_rgb555: None,
    emissive: [0.0, 0.0, 0.0],
    texture: None,
    blend: WorldSpriteBlend::Masked,
    flat_shade_row: 31,
};

/// `DAT_004c5268` at VA `0x004C5268` in `V2000.EXE` (256 bytes, 50 non-zero).
/// Non-zero queues; unanimous left/right/top/bottom drop; bit `0x40` is always
/// reject and bit `0x80` duplicates the low 128.
#[rustfmt::skip]
const DAT_004C5268: [u8; 256] = [
    // 0x00
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    // 0x10
    0x00, 0x00, 0x01, 0x01, 0x00, 0x01, 0x01, 0x01, 0x00, 0x00, 0x01, 0x01, 0x00, 0x01, 0x01, 0x01,
    // 0x20
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x01, 0x00, 0x01, 0x01, 0x01,
    // 0x30
    0x00, 0x00, 0x01, 0x01, 0x00, 0x01, 0x01, 0x01, 0x00, 0x00, 0x01, 0x01, 0x00, 0x01, 0x01, 0x01,
    // 0x40
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    // 0x50
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    // 0x60
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    // 0x70
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    // 0x80
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    // 0x90
    0x00, 0x00, 0x01, 0x01, 0x00, 0x01, 0x01, 0x01, 0x00, 0x00, 0x01, 0x01, 0x00, 0x01, 0x01, 0x01,
    // 0xA0
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x01, 0x00, 0x01, 0x01, 0x01,
    // 0xB0
    0x00, 0x00, 0x01, 0x01, 0x00, 0x01, 0x01, 0x01, 0x00, 0x00, 0x01, 0x01, 0x00, 0x01, 0x01, 0x01,
    // 0xC0
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    // 0xD0
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    // 0xE0
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    // 0xF0
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

/// The four screen-space corners of one `0x22` edge record plus its width
/// projection midpoint depth. The GL adapter also uses this depth; native
/// sorted A/B queues instead retain the first endpoint depth. Returns `None` when `DAT_004c5268[or] == 0` (unanimously off
/// one edge of `DAT_004feed8`/`feedc`) or when the taper denominator
/// degenerates. Partially off-screen records still queue — `FUN_0047EF10`
/// clips during fill. Endpoint depths are 8.8 raw.
#[allow(clippy::too_many_arguments)]
pub fn build_edge_quad(
    start_px: (i32, i32),
    end_px: (i32, i32),
    start_depth: i32,
    end_depth: i32,
    intr: &ScreenIntrinsics,
    width_start: u16,
    width_end: u16,
    size_operand: i32,
) -> Option<([[f32; 2]; 4], i32)> {
    build_edge_quad_impl(
        start_px,
        end_px,
        start_depth,
        end_depth,
        intr,
        width_start,
        width_end,
        size_operand,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
fn build_edge_quad_impl(
    start_px: (i32, i32),
    end_px: (i32, i32),
    start_depth: i32,
    end_depth: i32,
    intr: &ScreenIntrinsics,
    width_start: u16,
    width_end: u16,
    size_operand: i32,
    native_projection: Option<NativeScreenProjection>,
) -> Option<([[f32; 2]; 4], i32)> {
    // Near-table slots are i16. Projected endpoints that do not fit cannot
    // live there without wrapping, which turns a far-off foot into a
    // viewport sliver. Retail's span filler clips; a wrapped short is not a
    // clip. dx/dy below then match the stored shorts, so FUN_00457730's i32
    // sum-of-squares stays in range.
    let start = (near_table_pixel(start_px.0)?, near_table_pixel(start_px.1)?);
    let end = (near_table_pixel(end_px.0)?, near_table_pixel(end_px.1)?);
    let dx = i32::from(end.0) - i32::from(start.0);
    let dy = i32::from(end.1) - i32::from(start.1);
    // FUN_00457730(dx*dx+dy*dy) + 1. Two i16 squares fit i32; keep the
    // i64-then-narrow so a 65535-wide short delta matches the old wrap.
    let len =
        integer_sqrt((i64::from(dx) * i64::from(dx) + i64::from(dy) * i64::from(dy)) as i32) + 1;

    // Q15 direction along the edge.
    let dir_x = ((dx as i64) << 15) / (len as i64);
    let dir_y = ((dy as i64) << 15) / (len as i64);

    // Midpoint depth scales the width projection and the GL window-depth
    // adapter. Native A22/B22 sorted records keep the first endpoint key.
    let mid_depth = (start_depth + end_depth) / 2;
    // Retail calls FUN_004594C0 only when the raw midpoint depth exceeds
    // 0x3F; nearer records keep unwritten (zero) half-widths.
    let half_width = if mid_depth > 0x3F {
        native_projection.map_or_else(
            || perspective_half_width(size_operand, mid_depth, intr),
            |projection| projection.half_width(size_operand, mid_depth),
        )
    } else {
        (0, 0)
    };

    // Tip shift: t = (wEnd*len) / ((wStart-wEnd)*2), applied along the
    // direction. Both quad ends move by +/-t relative to their endpoints.
    let denom = (width_start as i64 - width_end as i64) * 2;
    if denom == 0 {
        return None;
    }
    let t = ((width_end as i64) * (len as i64)) / denom;
    let pull_x = (t * dir_x) >> 15;
    let pull_y = (t * dir_y) >> 15;

    let perp_x = -((half_width.0 as i64 * dir_y) >> 15);
    let perp_y = (half_width.1 as i64 * dir_x) >> 15;
    // FUN_00459000's degenerate guards: a zero perpendicular component
    // becomes 1 so near/flat records still rasterize as a hairline.
    let perp_x = if perp_x == 0 { 1 } else { perp_x };
    let perp_y = if perp_y == 0 { 1 } else { perp_y };

    // Corner layout verbatim from FUN_00459000: A/B hang off the start point
    // minus the pull, C/D hang off the end point plus the pull. Screen x/y
    // are i16 pixels (`(short)` in the near-table slots and outcode compares).
    let corners_i = [
        [
            i64::from(start.0) + perp_x - pull_x,
            i64::from(start.1) + perp_y - pull_y,
        ],
        [
            i64::from(start.0) - perp_x - pull_x,
            i64::from(start.1) - perp_y - pull_y,
        ],
        [
            i64::from(end.0) + pull_x + perp_x,
            i64::from(end.1) + pull_y + perp_y,
        ],
        [
            i64::from(end.0) + pull_x - perp_x,
            i64::from(end.1) + pull_y - perp_y,
        ],
    ];
    // Near-table slots are i16. Wrapping a 100k-pixel tip-shift into -30k
    // still looks "straddling" to DAT_004c5268, and GL then rasterizes a
    // viewport-wide sliver that looks like GPU artifacting. Retail's span
    // filler clips; a wrapped short is not a clip.
    if corners_i.iter().any(|c| {
        c[0] < i64::from(i16::MIN)
            || c[0] > i64::from(i16::MAX)
            || c[1] < i64::from(i16::MIN)
            || c[1] > i64::from(i16::MAX)
    }) {
        return None;
    }
    let corners_i16 = corners_i.map(|c| [c[0] as i16, c[1] as i16]);
    if !quad_survives_outcode_lut(corners_i16, intr.width_px, intr.height_px) {
        return None;
    }
    let corners = corners_i16.map(|c| [f32::from(c[0]), f32::from(c[1])]);
    Some((corners, mid_depth))
}

/// `FUN_004594C0`: project one authored width operand into integer screen
/// half-widths at 8.8 raw `depth`. The float divide reproduces retail's
/// single truncating division (its `>>12` operand swap only avoids 32-bit
/// overflow), and the halving loop is retail's exact guard: both components
/// halve while their OR reaches `0x2000`.
fn perspective_half_width(value: i32, depth: i32, intr: &ScreenIntrinsics) -> (i16, i16) {
    if depth == 0 {
        return (0, 0);
    }
    let mut x = ((value as f32) * intr.scale_x / (depth as f32)) as i32;
    let mut y = ((value as f32) * intr.scale_y / (depth as f32)) as i32;
    while (x | y) > 0x1FFF {
        x >>= 1;
        y >>= 1;
    }
    (x as i16, y as i16)
}

/// Combined-outcode lookup through `DAT_004c5268`. Non-zero queues.
fn quad_survives_outcode_lut(corners: [[i16; 2]; 4], width: i32, height: i32) -> bool {
    let mut combined = 0u8;
    for corner in corners {
        combined |= corner_outcode(corner[0], corner[1], width, height);
    }
    DAT_004C5268[usize::from(combined)] != 0
}

/// Per-corner outcode after `(short)` truncation.
///
/// X: 1 left, 2 inside (`(uint)(int)(short)x < width`), 4 right.
/// Y: 8 top, 0x10 inside, 0x20 bottom.
fn corner_outcode(x: i16, y: i16, width: i32, height: i32) -> u8 {
    let x_code = if (x as i32 as u32) < width as u32 {
        2
    } else if x >= 0 {
        4
    } else {
        1
    };
    let y_code = if (y as i32 as u32) < height as u32 {
        0x10
    } else if y >= 0 {
        0x20
    } else {
        8
    };
    x_code | y_code
}

/// Bit-by-bit integer square root (`FUN_00457730`); zero for inputs `< 1`.
pub fn integer_sqrt(value: i32) -> i32 {
    if value < 1 {
        return 0;
    }
    let mut remainder = value as u32;
    let mut bit = 1u32 << 30;
    while bit > remainder {
        bit >>= 2;
    }
    let mut result = 0u32;
    while bit != 0 {
        let attempt = result + bit;
        if remainder >= attempt {
            remainder -= attempt;
            result = (result >> 1) + bit;
        } else {
            result >>= 1;
        }
        bit >>= 2;
    }
    result as i32
}

fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// One constructor-queued `0x22` record: `FUN_00459000` screen-space corners,
/// the GL adapter's 8.8 raw midpoint depth, and the sprite/face material consumed
/// by the selected fill. The B/fogged fill's both-`0xFF` endpoint rejection is
/// model-policy-dependent and occurs later in the GL backend.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgeQuadSubmission {
    pub screen_corners: [[f32; 2]; 4],
    /// Projected endpoint depths in retail's 8.8 raw domain: `[start, end]`.
    ///
    /// The fogged B-pass copies the start byte to corners A/B and the end byte
    /// to C/D. Midpoint depth remains the GL adapter depth and must not replace
    /// this lengthwise fade gradient.
    pub endpoint_depths_raw: [i32; 2],
    pub mid_depth_raw: i32,
    pub material: FaceMaterial,
}

/// One submitted `0x02` palette hairline: `FUN_00458c60` screen endpoints,
/// each endpoint's 8.8 raw depth, and the Section-7 colour consumed by
/// `LAB_0047a740` (device `+0x1024` / `LAB_0047b360` on the 16-bpp table).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgeLineSubmission {
    pub start: [f32; 2],
    pub end: [f32; 2],
    pub start_depth_raw: i32,
    pub end_depth_raw: i32,
    pub material: FaceMaterial,
}

/// Recovered `0x22` quads and `0x02` hairlines from one model edge stream.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EdgeSelection {
    pub quads: Vec<EdgeQuadSubmission>,
    pub lines: Vec<EdgeLineSubmission>,
}

/// Run the full recovered gate over one model's edge stream.
///
/// This is the exact decision path the GL backend applies, split out so tests
/// can drive it headlessly against real parsed models. The selector's vertex
/// contract is **world** space; live `draw_tris_gl` converts raw-local
/// endpoints with `position + O · (raw · scale)` before calling. Unresolved
/// widths, unresolvable sprite widths, unresolvable endpoints, raw depth
/// `< 0x40`, and a zero LUT byte each drop their record.
pub fn select_edge_quads(
    edges: &[v2k_formats::models::ModelEdge],
    edge_widths: &[(u16, u16)],
    edge_materials: &[FaceMaterial],
    resolved_vertices: &[Option<[f32; 3]>],
    intr: &ScreenIntrinsics,
    cam_pos: [f32; 3],
    basis_rows: [[f32; 3]; 3],
) -> Vec<EdgeQuadSubmission> {
    select_edge_quads_with_stats(
        edges,
        edge_widths,
        edge_materials,
        resolved_vertices,
        intr,
        cam_pos,
        basis_rows,
    )
    .0
    .quads
}

/// Per-model drop accounting for the recovered gate, used by diagnostics.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EdgeGateStats {
    pub edges: usize,
    pub widths_resolved: usize,
    pub endpoints_ok: usize,
    pub projected: usize,
    /// `0x22` constructor/LUT survivors. A fogged B fill may subsequently
    /// reject a record whose two endpoint fade bytes are both `0xFF`.
    pub submitted: usize,
    pub lines_submitted: usize,
    pub clip_dropped: usize,
    /// `0x22` projected endpoints overflow the near-table i16.
    pub quad_overflow: usize,
    /// `0x22` equal widths (`t` divide-by-zero) or tip-shift corners overflow i16.
    pub quad_taper: usize,
    /// `0x22` `DAT_004c5268` unanimous off-screen.
    pub quad_lut: usize,
    /// `0x02` projected endpoints overflow the near-table i16.
    pub line_overflow: usize,
    /// `0x02` `DAT_004c5268` unanimous off-screen.
    pub line_lut: usize,
    pub native_endpoint_missing: usize,
    pub native_projection_missing: usize,
    pub native_viewport_mismatch: usize,
    pub snapshot_count_mismatch: usize,
}

/// [`select_edge_quads`] plus stage-by-stage drop accounting.
pub fn select_edge_quads_with_stats(
    edges: &[v2k_formats::models::ModelEdge],
    edge_widths: &[(u16, u16)],
    edge_materials: &[FaceMaterial],
    resolved_vertices: &[Option<[f32; 3]>],
    intr: &ScreenIntrinsics,
    cam_pos: [f32; 3],
    basis_rows: [[f32; 3]; 3],
) -> (EdgeSelection, EdgeGateStats) {
    select_model_edges(ModelEdgeRequest {
        edges,
        edge_widths,
        edge_materials,
        intr,
        projection: ModelEdgeProjectionInput::Compatibility {
            authority: SceneProjectionAuthority::default(),
            vertices: resolved_vertices,
            cam_pos,
            basis_rows,
        },
    })
}

pub enum ModelEdgeProjectionInput<'a> {
    Compatibility {
        authority: SceneProjectionAuthority,
        vertices: &'a [Option<[f32; 3]>],
        cam_pos: [f32; 3],
        basis_rows: [[f32; 3]; 3],
    },
    CommandSnapshots {
        snapshots: &'a [ModelEdgeEndpointSnapshot],
        authority: SceneProjectionAuthority,
        compatibility_vertices: &'a [Option<[f32; 3]>],
        cam_pos: [f32; 3],
        basis_rows: [[f32; 3]; 3],
    },
}

pub struct ModelEdgeRequest<'a> {
    pub edges: &'a [v2k_formats::models::ModelEdge],
    pub edge_widths: &'a [(u16, u16)],
    pub edge_materials: &'a [FaceMaterial],
    pub intr: &'a ScreenIntrinsics,
    pub projection: ModelEdgeProjectionInput<'a>,
}

/// Consume the producer's completed endpoint receipt. Native endpoints bypass
/// final-vars world reconstruction/admission; callbacks and RNG cannot replay.
pub fn select_model_edges(request: ModelEdgeRequest<'_>) -> (EdgeSelection, EdgeGateStats) {
    let ModelEdgeRequest {
        edges,
        edge_widths,
        edge_materials,
        intr,
        projection,
    } = request;
    let mut selection = EdgeSelection::default();
    let mut stats = EdgeGateStats {
        edges: edges.len(),
        ..EdgeGateStats::default()
    };
    if edge_widths.len() != edges.len() {
        return (selection, stats);
    }
    for (index, edge) in edges.iter().enumerate() {
        if let v2k_formats::models::ModelEdgeStyle::Sprite { .. } = edge.style {
            let widths = edge_widths[index];
            if widths == (0, 0) {
                continue;
            }
            stats.widths_resolved += 1;
        }
        let compatibility = |vertices: &[Option<[f32; 3]>], cam_pos, basis_rows, authority| {
            let a = vertices
                .get(usize::from(edge.vertices[0]))
                .copied()
                .flatten()?;
            let b = vertices
                .get(usize::from(edge.vertices[1]))
                .copied()
                .flatten()?;
            Some((
                intr.project(a, cam_pos, basis_rows, authority)?,
                intr.project(b, cam_pos, basis_rows, authority)?,
                None,
            ))
        };
        let prepared = match &projection {
            ModelEdgeProjectionInput::Compatibility {
                authority,
                vertices,
                cam_pos,
                basis_rows,
            } => compatibility(vertices, *cam_pos, *basis_rows, *authority),
            ModelEdgeProjectionInput::CommandSnapshots {
                snapshots,
                authority,
                compatibility_vertices,
                cam_pos,
                basis_rows,
            } => {
                let Some(snapshot) = snapshots
                    .get(index)
                    .filter(|_| snapshots.len() == edges.len())
                else {
                    stats.snapshot_count_mismatch += 1;
                    continue;
                };
                match (snapshot, authority) {
                    // A completed native constructor rejection is independent
                    // of scene adaptation and never becomes invented geometry.
                    (ModelEdgeEndpointSnapshot::SourceClipped { .. }, _) => {
                        stats.clip_dropped += 1;
                        continue;
                    }
                    // Scene adaptation is explicit and does not make a native
                    // receipt disappear; it chooses the established world path.
                    (_, SceneProjectionAuthority::Compatibility(_))
                    | (ModelEdgeEndpointSnapshot::Compatibility, _) => {
                        compatibility(compatibility_vertices, *cam_pos, *basis_rows, *authority)
                    }
                    (ModelEdgeEndpointSnapshot::MissingNative { .. }, _) => {
                        stats.native_endpoint_missing += 1;
                        continue;
                    }
                    (
                        ModelEdgeEndpointSnapshot::Native { .. },
                        SceneProjectionAuthority::Missing(_),
                    ) => {
                        stats.native_projection_missing += 1;
                        continue;
                    }
                    (
                        ModelEdgeEndpointSnapshot::Native { endpoints },
                        SceneProjectionAuthority::Native(native),
                    ) => {
                        if native.viewport_pixels() != [intr.width_px, intr.height_px] {
                            stats.native_viewport_mismatch += 1;
                            continue;
                        }
                        if endpoints
                            .iter()
                            .any(|endpoint| endpoint.clip == ModelSlotClip::SurfaceBand)
                        {
                            stats.clip_dropped += 1;
                            continue;
                        }
                        let [a, b] = endpoints.map(|endpoint| native.project(endpoint.view_raw));
                        if (a.clip | b.clip) & 0x40 != 0 {
                            stats.clip_dropped += 1;
                            continue;
                        }
                        Some((
                            (i32::from(a.screen[0]), i32::from(a.screen[1]), a.depth_raw),
                            (i32::from(b.screen[0]), i32::from(b.screen[1]), b.depth_raw),
                            Some(*native),
                        ))
                    }
                }
            }
        };
        let Some((pa, pb, native_projection)) = prepared else {
            continue;
        };
        stats.endpoints_ok += 1;
        stats.projected += 1;
        let material = edge_materials
            .get(index)
            .copied()
            .unwrap_or(FALLBACK_EDGE_MATERIAL);
        match edge.style {
            v2k_formats::models::ModelEdgeStyle::Sprite { size, .. } => {
                let widths = edge_widths[index];
                if near_table_pixel(pa.0).is_none()
                    || near_table_pixel(pa.1).is_none()
                    || near_table_pixel(pb.0).is_none()
                    || near_table_pixel(pb.1).is_none()
                {
                    stats.clip_dropped += 1;
                    stats.quad_overflow += 1;
                    continue;
                }
                if widths.0 == widths.1 {
                    stats.clip_dropped += 1;
                    stats.quad_taper += 1;
                    continue;
                }
                let Some((screen_corners, mid_depth_raw)) = build_edge_quad_impl(
                    (pa.0, pa.1),
                    (pb.0, pb.1),
                    pa.2,
                    pb.2,
                    intr,
                    widths.0,
                    widths.1,
                    i32::from(size as i16),
                    native_projection,
                ) else {
                    stats.clip_dropped += 1;
                    let start = [pa.0 as i16, pa.1 as i16];
                    let end = [pb.0 as i16, pb.1 as i16];
                    if !line_survives_outcode_lut(start, end, intr.width_px, intr.height_px) {
                        stats.quad_lut += 1;
                    } else {
                        stats.quad_taper += 1;
                    }
                    continue;
                };
                stats.submitted += 1;
                selection.quads.push(EdgeQuadSubmission {
                    screen_corners,
                    endpoint_depths_raw: [pa.2, pb.2],
                    mid_depth_raw,
                    material,
                });
            }
            v2k_formats::models::ModelEdgeStyle::Palette { .. } => {
                if near_table_pixel(pa.0).is_none()
                    || near_table_pixel(pa.1).is_none()
                    || near_table_pixel(pb.0).is_none()
                    || near_table_pixel(pb.1).is_none()
                {
                    stats.clip_dropped += 1;
                    stats.line_overflow += 1;
                    continue;
                }
                let Some((start, end)) =
                    build_edge_line((pa.0, pa.1), (pb.0, pb.1), intr.width_px, intr.height_px)
                else {
                    stats.clip_dropped += 1;
                    stats.line_lut += 1;
                    continue;
                };
                stats.lines_submitted += 1;
                selection.lines.push(EdgeLineSubmission {
                    start,
                    end,
                    start_depth_raw: pa.2,
                    end_depth_raw: pb.2,
                    material,
                });
            }
        }
    }
    (selection, stats)
}

/// `FUN_00458c60` A-pass `0x02`: two near-table endpoints, `DAT_004c5268` on
/// the OR of their outcodes. Survivors queue a palette line, not a tapered
/// quad. Equal to the 0x22 LUT, but the two vertex slots are the corners.
pub fn build_edge_line(
    start_px: (i32, i32),
    end_px: (i32, i32),
    width: i32,
    height: i32,
) -> Option<([f32; 2], [f32; 2])> {
    // Same near-table i16 contract as 0x22. Wrapping a 40k-pixel projection
    // into a negative short still looks like a LUT straddle, and pixel-ortho
    // GL_LINES then paint a full-height sky sliver. Those are the thin black
    // rods in the live hive shot; the 0x22 wrap-drop did not cover this path.
    let start = [near_table_pixel(start_px.0)?, near_table_pixel(start_px.1)?];
    let end = [near_table_pixel(end_px.0)?, near_table_pixel(end_px.1)?];
    if !line_survives_outcode_lut(start, end, width, height) {
        return None;
    }
    Some((
        [f32::from(start[0]), f32::from(start[1])],
        [f32::from(end[0]), f32::from(end[1])],
    ))
}

fn near_table_pixel(px: i32) -> Option<i16> {
    i16::try_from(px).ok()
}

fn line_survives_outcode_lut(start: [i16; 2], end: [i16; 2], width: i32, height: i32) -> bool {
    let combined = corner_outcode(start[0], start[1], width, height)
        | corner_outcode(end[0], end[1], width, height);
    DAT_004C5268[usize::from(combined)] != 0
}

fn neg(v: [f32; 3]) -> [f32; 3] {
    [-v[0], -v[1], -v[2]]
}

static EDGE_DIAG_REMAINING: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// Default number of printed invocations when `V2000_EDGE_DIAG` is set
/// without an explicit positive count.
const EDGE_DIAG_DEFAULT_TICKS: u32 = 8;

/// Tick budget carried by a raw `V2000_EDGE_DIAG` value. A decimal value is
/// honored exactly (including `0`, which silences the print while keeping the
/// selector active); any other set value falls back to the default.
fn diag_ticks_from_env(raw: Option<&std::ffi::OsStr>) -> u32 {
    match raw.and_then(std::ffi::OsStr::to_str) {
        Some(text) => text.trim().parse().unwrap_or(EDGE_DIAG_DEFAULT_TICKS),
        None => EDGE_DIAG_DEFAULT_TICKS,
    }
}

/// Whether the one-shot `V2000_EDGE_DIAG` stderr accounting is active. Arming
/// also seeds the print budget from the variable's value, once per process.
pub fn edge_diag_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| {
        let raw = std::env::var_os("V2000_EDGE_DIAG");
        let enabled = raw.is_some();
        if enabled {
            EDGE_DIAG_REMAINING.store(
                diag_ticks_from_env(raw.as_deref()),
                std::sync::atomic::Ordering::Relaxed,
            );
        }
        enabled
    })
}

/// Consume one diagnostic slot, returning the remaining count (0 when done).
pub fn edge_diag_tick() -> u32 {
    let previous = EDGE_DIAG_REMAINING.fetch_update(
        std::sync::atomic::Ordering::Relaxed,
        std::sync::atomic::Ordering::Relaxed,
        |n| (n > 0).then(|| n - 1),
    );
    previous.unwrap_or(0)
}

/// Last printed stage-accounting summary, used to avoid burning the one-shot
/// budget on every frame an edge-bearing model stays on screen.
static EDGE_DIAG_LAST: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

/// Whether a stage-accounting summary deserves a fresh printed line. Identical
/// consecutive summaries (the common per-frame repeat while a model sits in
/// view) return `false` without spending budget.
pub fn edge_diag_record(summary: &str) -> bool {
    let Ok(mut last) = EDGE_DIAG_LAST.lock() else {
        return false;
    };
    if last.as_deref() == Some(summary) {
        return false;
    }
    *last = Some(summary.to_owned());
    true
}

static EDGE_FILL_DIAG_REMAINING: std::sync::atomic::AtomicU32 =
    std::sync::atomic::AtomicU32::new(0);

/// Live fill-space dump. Not a retail tracer: construction is already
/// proven. This prints constructor pixels, 8.8 depths, pixel-ortho Z, gate
/// stats, and pixel spans so a GL hairball can be attributed to fill
/// conversion versus already-wide screen endpoints or selector drop. Set
/// `V2000_EDGE_FILL_DIAG` (optional count, default 16). A decimal `0` arms
/// the selector but prints nothing, matching `V2000_EDGE_DIAG`. Zero-edge
/// draws do not spend budget.
pub fn edge_fill_diag_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| {
        let raw = std::env::var_os("V2000_EDGE_FILL_DIAG");
        let enabled = raw.is_some();
        if enabled {
            let ticks = match raw.as_deref().and_then(std::ffi::OsStr::to_str) {
                Some(text) => text.trim().parse().unwrap_or(EDGE_FILL_DIAG_DEFAULT_TICKS),
                None => EDGE_FILL_DIAG_DEFAULT_TICKS,
            };
            EDGE_FILL_DIAG_REMAINING.store(ticks, std::sync::atomic::Ordering::Relaxed);
        }
        enabled
    })
}

/// Whether a live fill dump still has print budget. Drop-reason lines and
/// per-frame headers must use this, not [`edge_fill_diag_enabled`]: the env
/// var stays set for the process, and an unbudgeted print floods stderr.
pub fn edge_fill_diag_has_budget() -> bool {
    edge_fill_diag_enabled()
        && EDGE_FILL_DIAG_REMAINING.load(std::sync::atomic::Ordering::Relaxed) > 0
}

const EDGE_FILL_DIAG_DEFAULT_TICKS: u32 = 16;

static EDGE_FILL_DIAG_SKIPPED_EMPTY: std::sync::atomic::AtomicU32 =
    std::sync::atomic::AtomicU32::new(0);

/// Print one model's constructed edge fill operands. Spends the
/// `V2000_EDGE_FILL_DIAG` budget. Empty meshes and tiny unused streams do
/// not spend; insect-sized streams (20+ authored edges) or any successful
/// submit do.
pub fn edge_fill_diag_dump(
    selection: &EdgeSelection,
    intr: Option<&ScreenIntrinsics>,
    edges: usize,
    widths_len: usize,
    stats: &EdgeGateStats,
    header: Option<&str>,
) {
    if edges == 0 {
        EDGE_FILL_DIAG_SKIPPED_EMPTY.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        return;
    }
    let submitted = selection.quads.len() + selection.lines.len();
    if submitted == 0 && edges < 20 {
        EDGE_FILL_DIAG_SKIPPED_EMPTY.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        return;
    }
    let previous = EDGE_FILL_DIAG_REMAINING.fetch_update(
        std::sync::atomic::Ordering::Relaxed,
        std::sync::atomic::Ordering::Relaxed,
        |n| (n > 0).then(|| n - 1),
    );
    let Ok(remaining) = previous else {
        return;
    };
    if remaining == 0 {
        return;
    }
    let remaining = remaining - 1;
    let skipped = EDGE_FILL_DIAG_SKIPPED_EMPTY.load(std::sync::atomic::Ordering::Relaxed);
    if let Some(header) = header {
        eprintln!("{header}");
    }
    let Some(intr) = intr else {
        eprintln!("EDGE_FILL_DIAG({remaining}): no intrinsics edges={edges}");
        return;
    };
    eprintln!(
        "EDGE_FILL_DIAG({remaining}): edges={edges} widths_len={widths_len} quads={} lines={} skipped_empty={skipped} {stats:?} intr={intr:?}",
        selection.quads.len(),
        selection.lines.len()
    );
    let mut worst_span = 0.0_f32;
    for (index, quad) in selection.quads.iter().enumerate() {
        let xs: Vec<f32> = quad.screen_corners.iter().map(|c| c[0]).collect();
        let ys: Vec<f32> = quad.screen_corners.iter().map(|c| c[1]).collect();
        let span_x = xs.iter().cloned().fold(f32::NEG_INFINITY, f32::max)
            - xs.iter().cloned().fold(f32::INFINITY, f32::min);
        let span_y = ys.iter().cloned().fold(f32::NEG_INFINITY, f32::max)
            - ys.iter().cloned().fold(f32::INFINITY, f32::min);
        worst_span = worst_span.max(span_x.max(span_y));
        if index < 3 || span_x > 64.0 || span_y > 64.0 {
            let ortho_z = intr.ortho_fill_z(quad.mid_depth_raw);
            eprintln!(
                "  quad[{index}] screen={:?} mid_raw={} span_x={span_x:.1} span_y={span_y:.1} ortho_z={ortho_z:.5}",
                quad.screen_corners, quad.mid_depth_raw
            );
        }
    }
    if selection.quads.len() > 3 {
        eprintln!(
            "  worst_quad_span={worst_span:.1} ({} quads)",
            selection.quads.len()
        );
    }
    for (index, line) in selection.lines.iter().take(4).enumerate() {
        let span_x = (line.end[0] - line.start[0]).abs();
        let span_y = (line.end[1] - line.start[1]).abs();
        let start_z = intr.ortho_fill_z(line.start_depth_raw);
        let end_z = intr.ortho_fill_z(line.end_depth_raw);
        eprintln!(
            "  line[{index}] start={:?} end={:?} d_raw=({},{}) span=({span_x:.1},{span_y:.1}) ortho_z=({start_z:.5},{end_z:.5})",
            line.start, line.end, line.start_depth_raw, line.end_depth_raw
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn intr() -> ScreenIntrinsics {
        ScreenIntrinsics {
            width_px: 1024,
            height_px: 768,
            clip_near: 0.1,
            clip_far: 500.0,
            projection_effect: ProjectionEffect::None,
            scale_x: 886.0,
            scale_y: 1182.0,
            center_x: 512.0,
            center_y: 384.0,
        }
    }

    fn look_along_z() -> ([f32; 3], [[f32; 3]; 3]) {
        (
            [0.0; 3],
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, -1.0]],
        )
    }

    #[test]
    fn integer_sqrt_matches_retail_boundaries() {
        assert_eq!(integer_sqrt(0), 0);
        assert_eq!(integer_sqrt(-5), 0);
        assert_eq!(integer_sqrt(1), 1);
        assert_eq!(integer_sqrt(2), 1);
        assert_eq!(integer_sqrt(3), 1);
        assert_eq!(integer_sqrt(4), 2);
        assert_eq!(integer_sqrt(8), 2);
        assert_eq!(integer_sqrt(9), 3);
        assert_eq!(integer_sqrt(1_000_000), 1000);
        assert_eq!(integer_sqrt(0x3FFF_FFFF), 32_767);
    }

    #[test]
    fn projection_terms_preserve_the_off_centre_camera_signs() {
        let intr = ScreenIntrinsics::from_projection_terms(
            [320, 240],
            [1.5, 2.0, 0.725, 0.65],
            [0.1, 500.0],
        )
        .expect("positive logical viewport");
        assert_eq!((intr.width_px, intr.height_px), (320, 240));
        assert!((intr.center_x - 44.0).abs() < 1.0e-5);
        assert!((intr.center_y - 198.0).abs() < 1.0e-5);
        assert!((intr.scale_x - 240.0).abs() < 1.0e-5);
        assert!((intr.scale_y - 240.0).abs() < 1.0e-5);
        assert_eq!((intr.clip_near, intr.clip_far), (0.1, 500.0));
        assert!(ScreenIntrinsics::from_projection_terms(
            [0, 240],
            [1.0, 1.0, 0.0, 0.0],
            [0.1, 500.0],
        )
        .is_none());
    }

    #[test]
    fn diag_tick_budget_honors_decimal_and_falls_back_to_default() {
        use std::ffi::OsStr;
        let default = EDGE_DIAG_DEFAULT_TICKS;
        assert_eq!(diag_ticks_from_env(None), default, "bare set uses default");
        assert_eq!(diag_ticks_from_env(Some(OsStr::new(""))), default);
        assert_eq!(diag_ticks_from_env(Some(OsStr::new(" 12 "))), 12);
        assert_eq!(diag_ticks_from_env(Some(OsStr::new("1"))), 1);
        assert_eq!(
            diag_ticks_from_env(Some(OsStr::new("0"))),
            0,
            "explicit zero is honored as silence"
        );
        assert_eq!(diag_ticks_from_env(Some(OsStr::new("abc"))), default);
        assert_eq!(diag_ticks_from_env(Some(OsStr::new("-3"))), default);
    }

    #[test]
    fn diag_record_skips_unchanged_summaries() {
        assert!(edge_diag_record("edges=40 submitted=0"));
        assert!(
            !edge_diag_record("edges=40 submitted=0"),
            "identical per-frame repeat must not print"
        );
        assert!(edge_diag_record("edges=40 clip_dropped=16"));
        assert!(edge_diag_record("edges=40 submitted=0"));
    }

    #[test]
    fn equal_widths_have_a_degenerate_taper_denominator_and_drop() {
        assert!(
            build_edge_quad((100, 200), (300, 200), 400, 400, &intr(), 8, 8, 40).is_none(),
            "equal widths divide by zero in retail's taper term"
        );
    }

    #[test]
    fn wider_tip_pulls_both_corners_inward() {
        // Horizontal left-to-right edge whose tip is wider than its base:
        // the taper term goes negative, pulling the start corners toward +X
        // and the end corners toward -X, while the perpendicular stays
        // vertical. dir_y is 0, so perp_x degenerates and retail's
        // `==0 -> 1` guard separates each same-end pair by exactly 2px.
        let (corners, _mid) = build_edge_quad((100, 200), (300, 200), 400, 400, &intr(), 4, 12, 48)
            .expect("visible quad");
        assert_eq!(corners[0][0] - corners[1][0], 2.0, "retail hairline guard");
        assert_eq!(corners[2][0] - corners[3][0], 2.0);
        // Screen Y grows downward: corner A (+perp_y) sits below B (-perp_y).
        assert!(corners[1][1] < corners[0][1]);
        assert!(corners[3][1] < corners[2][1]);
        assert!(corners[0][0] > 100.0, "start pulled toward +X");
        assert!(corners[2][0] < 300.0, "end pulled toward -X");
    }

    #[test]
    fn thinner_tip_extends_both_ends_outward_past_the_endpoints() {
        let (corners, _) = build_edge_quad((100, 200), (300, 200), 400, 400, &intr(), 12, 4, 48)
            .expect("visible quad");
        assert!(corners[0][0] < 100.0, "start extended toward -X");
        assert!(corners[2][0] > 300.0, "end extended toward +X");
        // The perpendicular half-height is projected from the size operand
        // at the raw midpoint depth: 48 * scale_y / 400 = 141, and with the
        // Q15 direction (200<<15)/201 = 32604 the offset is
        // (141*32604)>>15 = 140, so the start corners are 280 apart.
        let spread = (corners[0][1] - corners[1][1]).abs();
        assert_eq!(spread, 280.0);
    }

    #[test]
    fn near_raw_depth_records_degenerate_to_the_retail_hairline() {
        // Midpoint depth 40 raw is at/below retail's 0x3F guard, so the
        // width projection never runs; the ==0 -> 1 guards leave a 2px
        // hairline instead of a perspective quad.
        let (corners, _) = build_edge_quad((100, 200), (300, 200), 40, 40, &intr(), 4, 12, 48)
            .expect("hairline quad");
        let spread = (corners[0][1] - corners[1][1]).abs();
        assert_eq!(spread, 2.0);
    }

    #[test]
    fn perspective_half_width_reproduces_retail_halving_guard() {
        let intr = intr();
        // 100 * 886 / 50 = 1772, 100 * 1182 / 50 = 2364: below the guard.
        assert_eq!(perspective_half_width(100, 50, &intr), (1772, 2364));
        // At depth 5 the raw results (17720, 23640) reach past 0x2000 and
        // both components halve twice together: (4430, 5910).
        assert_eq!(perspective_half_width(100, 5, &intr), (4430, 5910));
    }

    #[test]
    fn lut_inside_byte_and_nonzero_count_match_the_exe_dump() {
        assert_eq!(DAT_004C5268[0x12], 1, "fully inside is X:2 | Y:0x10");
        assert_eq!(DAT_004C5268.iter().filter(|&&byte| byte != 0).count(), 50);
    }

    #[test]
    fn exploded_tip_shift_does_not_wrap_i16_corners_into_a_screen_sliver() {
        // Near-equal authored widths on a short on-screen edge make `t` huge.
        // Truncating those 100k-pixel corners to i16 used to queue a
        // left-edge sliver (`DAT_004c5268` still sees a straddle).
        assert!(
            build_edge_quad(
                (1490, 540),
                (1494, 542),
                7000,
                7000,
                &intr(),
                65535,
                65534,
                48
            )
            .is_none(),
            "overflowing constructor pixels must drop, not wrap"
        );
    }

    #[test]
    fn unanimous_offscreen_records_are_rejected_by_the_lut() {
        // Both endpoints sit far above the top edge; the axial pull only
        // moves corners along X, so every corner stays offscreen-top.
        assert!(build_edge_quad((100, -400), (300, -400), 400, 400, &intr(), 4, 12, 48).is_none());
        // A fully offscreen-left edge whose pull cannot reach the frame.
        assert!(
            build_edge_quad((-2000, 200), (-1500, 200), 400, 400, &intr(), 4, 12, 48).is_none()
        );
    }

    #[test]
    fn palette_line_uses_two_endpoint_outcodes() {
        let (start, end) = build_edge_line((100, 200), (300, 200), 1024, 768).expect("on-screen");
        assert_eq!(start, [100.0, 200.0]);
        assert_eq!(end, [300.0, 200.0]);
        assert!(
            build_edge_line((100, -400), (300, -400), 1024, 768).is_none(),
            "unanimous offscreen-top drops"
        );
        let (start, end) =
            build_edge_line((-50, 200), (300, 200), 1024, 768).expect("straddling line");
        assert!(start[0] < 0.0);
        assert!(end[0] > 0.0);
        assert!(
            build_edge_line((100, 200), (100, 40_000), 1024, 768).is_none(),
            "projected 0x02 endpoints that overflow i16 must drop, not wrap into a sky sliver"
        );
    }

    #[test]
    fn selector_submits_palette_hairlines_without_sprite_widths() {
        let intr = intr();
        let (cam, basis) = look_along_z();
        let start = [0.0_f32, 0.0, 10.0];
        let end = [2.0_f32, 0.0, 10.0];
        let (selection, stats) = select_edge_quads_with_stats(
            &[v2k_formats::models::ModelEdge {
                vertices: [0, 1],
                style: v2k_formats::models::ModelEdgeStyle::Palette { mat: 3 },
            }],
            &[(0, 0)],
            &[FALLBACK_EDGE_MATERIAL],
            &[Some(start), Some(end)],
            &intr,
            cam,
            basis,
        );
        assert!(selection.quads.is_empty());
        assert_eq!(selection.lines.len(), 1);
        assert_eq!(stats.lines_submitted, 1);
        assert_eq!(stats.submitted, 0);
        let pa = intr
            .project(start, cam, basis, SceneProjectionAuthority::default())
            .unwrap();
        let pb = intr
            .project(end, cam, basis, SceneProjectionAuthority::default())
            .unwrap();
        assert_eq!(selection.lines[0].start, [pa.0 as f32, pa.1 as f32]);
        assert_eq!(selection.lines[0].end, [pb.0 as f32, pb.1 as f32]);
        assert_eq!(selection.lines[0].start_depth_raw, pa.2);
        assert_eq!(selection.lines[0].end_depth_raw, pb.2);
    }

    #[test]
    fn straddling_quads_queue_through_the_lut() {
        // Start hangs off the left; end stays on-screen. An AABB "any corner
        // outside drops" test would reject this; DAT_004c5268 queues it
        // (OR = left|inside|inside-y = 0x13).
        let (corners, _) = build_edge_quad((-50, 200), (300, 200), 400, 400, &intr(), 12, 4, 48)
            .expect("partially off-screen legs must submit");
        assert!(
            corners.iter().any(|c| c[0] < 0.0),
            "start hangs off the left"
        );
        assert!(
            corners.iter().any(|c| c[0] >= 0.0 && c[0] < 1024.0),
            "end remains on-screen"
        );
    }

    #[test]
    fn selector_keeps_constructor_screen_corners_without_unprojecting() {
        let intr = intr();
        let (cam, basis) = look_along_z();
        let start = [0.0_f32, 0.0, 10.0];
        let end = [2.0_f32, 0.0, 10.0];
        let pa = intr
            .project(start, cam, basis, SceneProjectionAuthority::default())
            .expect("start in front");
        let pb = intr
            .project(end, cam, basis, SceneProjectionAuthority::default())
            .expect("end in front");
        let (expected, mid) =
            build_edge_quad((pa.0, pa.1), (pb.0, pb.1), pa.2, pb.2, &intr, 12, 4, 48)
                .expect("visible quad");
        let submissions = select_edge_quads(
            &[v2k_formats::models::ModelEdge {
                vertices: [0, 1],
                style: v2k_formats::models::ModelEdgeStyle::Sprite {
                    sprite_id: 0,
                    size: 48,
                },
            }],
            &[(12, 4)],
            &[FALLBACK_EDGE_MATERIAL],
            &[Some(start), Some(end)],
            &intr,
            cam,
            basis,
        );
        assert_eq!(submissions.len(), 1);
        assert_eq!(submissions[0].screen_corners, expected);
        assert_eq!(submissions[0].endpoint_depths_raw, [pa.2, pb.2]);
        assert_eq!(submissions[0].mid_depth_raw, mid);
    }

    #[test]
    fn view_from_screen_inverts_project_in_view_space() {
        let intr = intr();
        let (cam, basis) = look_along_z();
        let world = [1.5_f32, -0.25, 8.0];
        let (sx, sy, raw) = intr
            .project(world, cam, basis, SceneProjectionAuthority::default())
            .expect("in front");
        let view = intr
            .view_from_screen(sx as f32, sy as f32, raw)
            .expect("finite scale");
        let delta = [world[0] - cam[0], world[1] - cam[1], world[2] - cam[2]];
        let expected = [
            dot3(basis[0], delta),
            dot3(basis[1], delta),
            dot3(basis[2], delta),
        ];
        assert!((view[0] - expected[0]).abs() < 0.05);
        assert!((view[1] - expected[1]).abs() < 0.05);
        assert!((view[2] - expected[2]).abs() < 0.05);
        assert!(
            view[2] < 0.0,
            "OpenGL view Z is negative in front of the eye"
        );
    }

    fn mul_column_major(m: [f32; 16], v: [f32; 3]) -> [f32; 4] {
        let x = v[0];
        let y = v[1];
        let z = v[2];
        [
            m[0] * x + m[4] * y + m[8] * z + m[12],
            m[1] * x + m[5] * y + m[9] * z + m[13],
            m[2] * x + m[6] * y + m[10] * z + m[14],
            m[3] * x + m[7] * y + m[11] * z + m[15],
        ]
    }

    #[test]
    fn view_from_screen_round_trips_through_the_gameplay_projection_matrix() {
        let mut cam = crate::camera::Camera::new(4.0 / 3.0);
        cam.fov = 60.0_f32.to_radians();
        cam.left_handed = true;
        cam.projection_offset = [0.25, 0.30];
        let proj = cam.projection_matrix();
        let width = 640.0_f32;
        let height = 480.0_f32;
        let intr = ScreenIntrinsics::from_projection_terms(
            [width as u32, height as u32],
            [
                proj[0],
                proj[5],
                cam.projection_offset[0],
                cam.projection_offset[1],
            ],
            [cam.near, cam.far],
        )
        .expect("positive logical viewport");
        let sx = 400.0;
        let sy = 180.0;
        let raw = 40 * 256;
        let view = intr.view_from_screen(sx, sy, raw).expect("finite scale");
        let clip = mul_column_major(proj, view);
        assert!(clip[3].abs() > 1.0e-5, "clip w {clip:?}");
        let ndc_x = clip[0] / clip[3];
        let ndc_y = clip[1] / clip[3];
        let back_x = (ndc_x + 1.0) * 0.5 * width;
        let back_y = (1.0 - ndc_y) * 0.5 * height;
        assert!(
            (back_x - sx).abs() < 1.5,
            "x round-trip {back_x} vs {sx}; view={view:?} clip={clip:?}"
        );
        assert!(
            (back_y - sy).abs() < 1.5,
            "y round-trip {back_y} vs {sy}; view={view:?} clip={clip:?}"
        );
        let ndc_z = clip[2] / clip[3];
        assert!(
            (intr.ndc_z_from_raw_depth(raw) - ndc_z).abs() < 1.0e-4,
            "window Z must match the 3D perspective pass"
        );
        assert!(
            (intr.ortho_fill_z(raw) + ndc_z).abs() < 1.0e-4,
            "pixel-ortho eye Z is -ndc.z under glOrtho(..., -1, 1)"
        );
    }

    #[test]
    fn projection_round_trip_survives_through_unproject() {
        let intr = intr();
        let world = [130.0_f32, 42.0, -17.0];
        let cam = [100.0_f32, 20.0, -60.0];
        let basis = [
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, -1.0], // -forward: camera looks toward +Z
        ];
        let (sx, sy, d) = intr
            .project(world, cam, basis, SceneProjectionAuthority::default())
            .expect("in front");
        let back = intr.unproject(sx as f32, sy as f32, d as f32, cam, basis);
        assert!((back[0] - world[0]).abs() < 0.6);
        assert!((back[1] - world[1]).abs() < 0.6);
        assert!((back[2] - world[2]).abs() < 0.6);
    }

    #[test]
    fn projection_rejects_points_behind_the_eye() {
        let intr = intr();
        let (cam, basis) = look_along_z();
        assert!(intr
            .project(
                [0.0, 0.0, -5.0],
                cam,
                basis,
                SceneProjectionAuthority::default()
            )
            .is_none());
    }

    #[test]
    fn projection_rejects_raw_depth_below_the_near_clip() {
        let intr = intr();
        let (cam, basis) = look_along_z();
        assert!(
            intr.project(
                [0.0, 0.0, 63.0 / 256.0],
                cam,
                basis,
                SceneProjectionAuthority::default()
            )
            .is_none(),
            "raw 63 < 0x40"
        );
        let accepted = intr
            .project(
                [0.0, 0.0, 64.0 / 256.0],
                cam,
                basis,
                SceneProjectionAuthority::default(),
            )
            .expect("raw 0x40 is the first accepted depth");
        assert_eq!(accepted.2, 0x40);
    }

    #[test]
    fn endpoint_projection_matches_original_46cd90_cap_boundary_cases() {
        let mut intr = intr();
        intr.scale_x = 512.0;
        intr.scale_y = 512.0;
        intr.width_px = 640;
        intr.height_px = 480;
        intr.center_x = 320.0;
        intr.center_y = 240.0;
        let (cam, basis) = look_along_z();
        // Verified original PE 46CD90 outputs; complete VIEW inputs are
        // controlled arithmetic cases, not captured newant actor poses.
        for (raw, pixels) in [
            ([983, 0, 64], [8184, 240]),
            ([984, 0, 64], [4096, 120]),
            ([32760, 512, 64], [4100, -61]),
            ([-32760, -512, 64], [-8180, 135]),
        ] {
            let world = raw.map(|value| value as f32 / 256.0);
            assert_eq!(
                intr.project(world, cam, basis, SceneProjectionAuthority::default()),
                Some((pixels[0], pixels[1], raw[2])),
            );
        }
        assert!(intr
            .project(
                [0.0, 0.0, 63.0 / 256.0],
                cam,
                basis,
                SceneProjectionAuthority::default()
            )
            .is_none());
    }

    #[test]
    fn endpoint_screen_stage_keeps_free_camera_lens_distinct() {
        let mut native = intr();
        native.scale_x = 512.0;
        native.scale_y = 512.0;
        native.width_px = 640;
        native.height_px = 480;
        native.center_x = 320.0;
        native.center_y = 240.0;
        let mut free = native;
        free.scale_x = 256.0;
        free.scale_y = 256.0;
        let (cam, basis) = look_along_z();
        let world = [984.0 / 256.0, 0.0, 64.0 / 256.0];
        assert_eq!(
            native.project(world, cam, basis, SceneProjectionAuthority::default()),
            Some((4096, 120, 64))
        );
        assert_eq!(
            free.project(world, cam, basis, SceneProjectionAuthority::default()),
            Some((4256, 240, 64))
        );
    }

    #[test]
    fn endpoint_cap_reaches_original_a22_constructor_and_palette_lane() {
        let mut intr = intr();
        intr.scale_x = 512.0;
        intr.scale_y = 512.0;
        intr.width_px = 640;
        intr.height_px = 480;
        intr.center_x = 320.0;
        intr.center_y = 240.0;
        let (cam, basis) = look_along_z();
        let vertices = [
            Some([32760.0 / 256.0, 512.0 / 256.0, 64.0 / 256.0]),
            Some([0.0, 0.0, 512.0 / 256.0]),
        ];
        // The previous float adapter produced [262400, -3856] and rejected
        // both lanes before construction because the screen WORD overflowed.
        assert!(build_edge_quad((262400, -3856), (320, 240), 64, 512, &intr, 32, 8, 4,).is_none());
        assert!(build_edge_line((262400, -3856), (320, 240), 640, 480).is_none());
        let (selection, stats) = select_edge_quads_with_stats(
            &[
                v2k_formats::models::ModelEdge {
                    vertices: [0, 1],
                    style: v2k_formats::models::ModelEdgeStyle::Sprite {
                        sprite_id: 986,
                        size: 4,
                    },
                },
                v2k_formats::models::ModelEdge {
                    vertices: [0, 1],
                    style: v2k_formats::models::ModelEdgeStyle::Palette { mat: 3 },
                },
            ],
            &[(32, 8), (0, 0)],
            &[FALLBACK_EDGE_MATERIAL; 2],
            &vertices,
            &intr,
            cam,
            basis,
        );
        assert_eq!(
            (stats.projected, stats.submitted, stats.lines_submitted),
            (2, 1, 1),
        );
        let quad = &selection.quads[0];
        assert_eq!(quad.endpoint_depths_raw, [64, 512]);
        assert_eq!(quad.mid_depth_raw, 288);
        // Actual 46CD90 -> A22 PE result. Native payload stores A,C,D,B;
        // the adapter retains A,B,C,D for its existing triangulation.
        let corners = quad.screen_corners;
        assert_eq!(
            [corners[0], corners[2], corners[3], corners[1]],
            [
                [4731.0, -118.0],
                [-309.0, 283.0],
                [-311.0, 297.0],
                [4729.0, -104.0],
            ],
        );
        assert_eq!(selection.lines[0].start, [4100.0, -61.0]);
        assert_eq!(selection.lines[0].end, [320.0, 240.0]);
    }

    #[test]
    fn underwater_endpoint_phase_follows_one_latched_cap() {
        let mut intr = intr();
        intr.scale_x = 512.0;
        intr.scale_y = 512.0;
        intr.width_px = 640;
        intr.height_px = 480;
        intr.center_x = 320.0;
        intr.center_y = 240.0;
        let (cam, basis) = look_along_z();
        // The uncapped pixels are [-16383, 1]. Native's latched control gives
        // [-8192, 0], then tick 0 contributes [1, 0]. Replaying the cap on the
        // completed endpoint would incorrectly move X near -4096.
        let world = [(-16383.0 - 320.0) / 2048.0, (240.0 - 1.0) / 2048.0, 0.25];
        intr.projection_effect = ProjectionEffect::RetailUnderwater { tick: 0 };
        assert_eq!(
            intr.project(world, cam, basis, SceneProjectionAuthority::default()),
            Some((-8191, 0, 64))
        );
    }
}

#[cfg(test)]
mod command_snapshot_tests {
    use super::*;
    use crate::projection::{ProjectionAuthorityMissing, ProjectionCompatibilityReason};
    use v2k_formats::models::{
        ModelEdge, ModelEdgeStyle, ModelNativeEdgeBoundary, ModelNativeEdgeEndpoint,
    };

    #[test]
    fn native_receipts_ignore_world_reprojection_and_keep_both_palette_and_sprite_edges() {
        let intr = ScreenIntrinsics::from_projection_terms(
            [640, 480],
            [1.6, 512.0 / 240.0, 0.0, 0.0],
            [0.1, 500.0],
        )
        .unwrap();
        let native =
            NativeScreenProjection::new([512; 2], [320, 240], [640, 480], ProjectionEffect::None)
                .unwrap();
        let edges = [
            ModelEdge {
                vertices: [0, 1],
                style: ModelEdgeStyle::Palette { mat: 32 },
            },
            ModelEdge {
                vertices: [0, 1],
                style: ModelEdgeStyle::Sprite {
                    sprite_id: 680,
                    size: 7,
                },
            },
        ];
        let endpoint = |slot, view_raw| ModelNativeEdgeEndpoint {
            slot,
            view_raw,
            clip: ModelSlotClip::Clear,
        };
        let snapshots = [
            ModelEdgeEndpointSnapshot::Native {
                endpoints: [endpoint(0, [-120, 20, 512]), endpoint(2, [140, 80, 768])],
            },
            ModelEdgeEndpointSnapshot::Native {
                endpoints: [endpoint(0, [-120, 20, 512]), endpoint(2, [140, 80, 768])],
            },
        ];
        let request = |authority| ModelEdgeRequest {
            edges: &edges,
            edge_widths: &[(0, 0), (32, 8)],
            edge_materials: &[],
            intr: &intr,
            projection: ModelEdgeProjectionInput::CommandSnapshots {
                snapshots: &snapshots,
                authority,
                compatibility_vertices: &[None, None],
                cam_pos: [0.; 3],
                basis_rows: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            },
        };
        let (selection, stats) =
            select_model_edges(request(SceneProjectionAuthority::Native(native)));
        assert_eq!(selection.lines[0].end, [413.0, 187.0]);
        assert_eq!(selection.lines[0].start_depth_raw, 512);
        assert_eq!(selection.quads[0].endpoint_depths_raw, [512, 768]);
        assert_eq!(stats.native_projection_missing, 0);
        assert_eq!(stats.lines_submitted, 1);
        assert_eq!(stats.submitted, 1);
        let (missing, stats) = select_model_edges(request(SceneProjectionAuthority::Missing(
            ProjectionAuthorityMissing::NativeLens,
        )));
        assert!(missing.lines.is_empty() && missing.quads.is_empty());
        assert_eq!(stats.native_projection_missing, 2);
        let (_, stats) = select_model_edges(request(SceneProjectionAuthority::Compatibility(
            ProjectionCompatibilityReason::Free,
        )));
        assert_eq!(
            stats.native_projection_missing, 0,
            "explicit Free is compatibility, not a missing native authority"
        );
    }

    #[test]
    fn endpoint_missing_native_cannot_fallback_but_explicit_adapter_can() {
        let intr = ScreenIntrinsics::from_projection_terms(
            [640, 480],
            [1.6, 512.0 / 240.0, 0.0, 0.0],
            [0.1, 500.0],
        )
        .unwrap();
        let edges = [ModelEdge {
            vertices: [0, 1],
            style: ModelEdgeStyle::Palette { mat: 32 },
        }];
        let snapshots = [ModelEdgeEndpointSnapshot::MissingNative {
            boundary: ModelNativeEdgeBoundary::ViewUnavailable { slot: 0 },
        }];
        let points = [
            Some([-120.0 / 256., 20.0 / 256., -2.]),
            Some([140.0 / 256., 80.0 / 256., -3.]),
        ];
        let request = |authority| ModelEdgeRequest {
            edges: &edges,
            edge_widths: &[(0, 0)],
            edge_materials: &[],
            intr: &intr,
            projection: ModelEdgeProjectionInput::CommandSnapshots {
                snapshots: &snapshots,
                authority,
                compatibility_vertices: &points,
                cam_pos: [0.; 3],
                basis_rows: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            },
        };
        let native =
            NativeScreenProjection::new([512; 2], [320, 240], [640, 480], ProjectionEffect::None)
                .unwrap();
        let (selection, stats) =
            select_model_edges(request(SceneProjectionAuthority::Native(native)));
        assert!(selection.lines.is_empty());
        assert_eq!(stats.native_endpoint_missing, 1);
        let (selection, stats) =
            select_model_edges(request(SceneProjectionAuthority::Compatibility(
                ProjectionCompatibilityReason::LogicalViewportAdapter,
            )));
        assert_eq!(selection.lines.len(), 1);
        assert_eq!(stats.native_endpoint_missing, 0);
        assert_eq!(
            selection.lines[0].end,
            [413.0, 186.0],
            "compatibility deliberately retains its float quotient order"
        );
    }
}

#[cfg(test)]
mod logical_viewport_tests {
    use super::*;
    use v2k_formats::models::{
        ModelEdge, ModelEdgeConstructorClip, ModelEdgeStyle, ModelNativeEdgeEndpoint,
    };

    const LOGICAL: SceneProjectionAuthority = SceneProjectionAuthority::Compatibility(
        ProjectionCompatibilityReason::LogicalViewportAdapter,
    );

    #[test]
    fn dry_logical_projection_matches_gl_while_other_screen_stages_keep_the_cap() {
        let mut intr = ScreenIntrinsics::from_projection_terms(
            [3840, 2160],
            [1.0, 1.0, 0.0, 0.0],
            [0.1, 500.0],
        )
        .unwrap();
        let cam = [0.0; 3];
        let basis = [[1., 0., 0.], [0., 1., 0.], [0., 0., -1.]];
        // Controlled pixels crossing the source's asymmetric halving boundary.
        let world = [(8384. - intr.center_x) / intr.scale_x, -1., 1.];
        let uncapped = (8384, 2160, 256);
        assert_eq!(intr.project(world, cam, basis, LOGICAL), Some(uncapped));
        let native = NativeScreenProjection::new(
            [1920, 1080],
            [1920, 1080],
            [3840, 2160],
            ProjectionEffect::None,
        )
        .unwrap();
        for authority in [
            SceneProjectionAuthority::default(),
            SceneProjectionAuthority::Compatibility(ProjectionCompatibilityReason::Free),
            SceneProjectionAuthority::Native(native),
        ] {
            assert_eq!(
                intr.project(world, cam, basis, authority),
                Some((4192, 1080, 256))
            );
        }
        intr.projection_effect = ProjectionEffect::RetailUnderwater { tick: 37 };
        let underwater = retail_projected_point(uncapped.0, uncapped.1, intr.projection_effect);
        assert_eq!(
            intr.project(world, cam, basis, LOGICAL),
            Some((underwater.x, underwater.y, 256)),
            "underwater edges retain the cap and phase applied by the GL shader",
        );
        intr.projection_effect = ProjectionEffect::None;
        assert!(intr
            .project([0., 0., 63. / 256.], cam, basis, LOGICAL)
            .is_none());
        assert_eq!(
            intr.project([0., 0., 64. / 256.], cam, basis, LOGICAL)
                .unwrap()
                .2,
            64
        );
    }

    #[test]
    fn offscreen_insect_ribbon_cannot_fold_back_into_a_4k_logical_viewport() {
        let intr = ScreenIntrinsics::from_projection_terms(
            [3840, 2160],
            [1.2011719, 2.1354167, 0., 0.],
            [0.1, 500.],
        )
        .unwrap();
        // Reproduced Intro2 opening frame 990: sprite 680, edge 28. Both true
        // endpoints lie beyond the right/bottom edges. Capping only the first
        // pulled its tapered tip to [3535,1494], creating a stray sky rectangle.
        let points = [
            Some([204.48828, -2.265625, 14.3828125]),
            Some([204.39453, -1.9570313, 14.628906]),
        ];
        let cam = [187.9414, 4.0898438, 4.7734375];
        let basis = [
            [0.98603565, 0., 0.16653417],
            [0.008196946, 0.99878794, -0.04853347],
            [0.16633233, -0.049220808, -0.9848406],
        ];
        let edges = [ModelEdge {
            vertices: [0, 1],
            style: ModelEdgeStyle::Sprite {
                sprite_id: 680,
                size: 7,
            },
        }];
        let snapshots = [ModelEdgeEndpointSnapshot::Native {
            endpoints: [[4585, -1711, 1636], [4573, -1635, 1706]].map(|view_raw| {
                ModelNativeEdgeEndpoint {
                    slot: 0,
                    view_raw,
                    clip: ModelSlotClip::Clear,
                }
            }),
        }];
        for snapshots in [None, Some(snapshots.as_slice())] {
            let request = |authority| ModelEdgeRequest {
                edges: &edges,
                edge_widths: &[(32, 8)],
                edge_materials: &[],
                intr: &intr,
                projection: match snapshots {
                    None => ModelEdgeProjectionInput::Compatibility {
                        authority,
                        vertices: &points,
                        cam_pos: cam,
                        basis_rows: basis,
                    },
                    Some(snapshots) => ModelEdgeProjectionInput::CommandSnapshots {
                        snapshots,
                        authority,
                        compatibility_vertices: &points,
                        cam_pos: cam,
                        basis_rows: basis,
                    },
                },
            };
            let (old, _) = select_model_edges(request(SceneProjectionAuthority::default()));
            assert_eq!(
                old.quads.len(),
                1,
                "legacy cap reproduces the visible rectangle"
            );
            assert_eq!(old.quads[0].screen_corners[0], [3535., 1494.]);
            let (adapted, stats) = select_model_edges(request(LOGICAL));
            assert!(adapted.quads.is_empty());
            assert_eq!(
                stats.quad_lut, 1,
                "ordinary offscreen rejection, without a fitted clamp"
            );
        }
    }

    #[test]
    fn source_clipped_receipts_never_use_visible_logical_fallback_points() {
        let mut intr = ScreenIntrinsics::from_projection_terms(
            [3840, 2160],
            [1.2011719, 2.1354167, 0., 0.],
            [0.1, 500.],
        )
        .unwrap();
        let points = [Some([-0.25, 0., 2.]), Some([0.25, 0.125, 2.])];
        let edges = [ModelEdge {
            vertices: [0, 1],
            style: ModelEdgeStyle::Sprite {
                sprite_id: 680,
                size: 7,
            },
        }];
        for effect in [
            ProjectionEffect::None,
            ProjectionEffect::RetailUnderwater { tick: 37 },
        ] {
            intr.projection_effect = effect;
            let select = |snapshot| {
                select_model_edges(ModelEdgeRequest {
                    edges: &edges,
                    edge_widths: &[(32, 8)],
                    edge_materials: &[],
                    intr: &intr,
                    projection: ModelEdgeProjectionInput::CommandSnapshots {
                        snapshots: &[snapshot],
                        authority: LOGICAL,
                        compatibility_vertices: &points,
                        cam_pos: [0.; 3],
                        basis_rows: [[1., 0., 0.], [0., 1., 0.], [0., 0., -1.]],
                    },
                })
            };
            assert_eq!(
                select(ModelEdgeEndpointSnapshot::Compatibility)
                    .0
                    .quads
                    .len(),
                1,
                "the fallback geometry must be visible for {effect:?}"
            );
            for reason in [
                ModelEdgeConstructorClip::SurfaceBand,
                ModelEdgeConstructorClip::NearView { depth_raw: 63 },
            ] {
                let (selection, stats) =
                    select(ModelEdgeEndpointSnapshot::SourceClipped { slot: 0, reason });
                assert!(selection.quads.is_empty() && selection.lines.is_empty());
                assert_eq!(stats.clip_dropped, 1);
                assert_eq!(stats.endpoints_ok, 0);
                assert_eq!(stats.projected, 0);
            }
        }
    }
}
