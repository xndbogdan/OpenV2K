//! Per-row vertex attribute classes: span-row routines one to five.
//!
//! Each class owns a fixed mapping between raster-vertex dwords and the
//! scan converter's 18-dword edge record. The retail routines differ only in
//! that mapping, so one table-driven implementation reproduces all of them:
//!
//! | class | initialise | x clip | clamp | step | copy |
//! |---|---|---|---|---|---|
//! | flat | `00473890` | `00473690`/`00473900` | `00473A80` | `00473810` | `00473BE0` |
//! | Gouraud | `00473CB0` | `00473D90` | `00473FA0` | `004741A0` | `004741F0` |
//! | Gouraud + fade | `00474390` | `004744A0` | `004746B0` | `004748E0` | `00474940` |
//! | textured | `00474B20` | `00474BE0` | `00474DC0` | `00474F70` | `00474DA0` |
//! | textured + shade | `004750B0` | `004751E0` | `00475410` | `00475660` | `004756D0` |
//! | textured + shade + fade | `00475940` | `00475A90` | `00475CE0` | `00475F60` | `00475FE0` |

use super::fixed::{clip_ratio, edge_reciprocal, mul_q31, slope_q31};
use super::state::{Edge, RasterVertex, EDGE_VALID, EDGE_Y, VX, VY};

/// One interpolated vertex dword and where an edge keeps it.
#[derive(Debug, Clone, Copy)]
struct Field {
    vertex: usize,
    value: usize,
    slope: usize,
    /// Slope setup moves a negative step one unit toward zero.
    rounded: bool,
}

const fn field(vertex: usize, value: usize, slope: usize, rounded: bool) -> Field {
    Field {
        vertex,
        value,
        slope,
        rounded,
    }
}

const FLAT: &[Field] = &[field(VX, 0, 1, true)];
const GOURAUD: &[Field] = &[
    field(3, 4, 9, false),
    field(0, 1, 6, false),
    field(1, 2, 7, false),
    field(2, 3, 8, false),
    field(VX, 0, 5, true),
];
const GOURAUD_FADE: &[Field] = &[
    field(3, 4, 10, false),
    field(0, 1, 7, false),
    field(1, 2, 8, false),
    field(2, 3, 9, false),
    field(8, 5, 11, true),
    field(VX, 0, 6, true),
];
const TEXTURED: &[Field] = &[
    field(6, 1, 4, true),
    field(7, 2, 5, true),
    field(VX, 0, 3, true),
];
const TEXTURED_SHADE: &[Field] = &[
    field(3, 6, 13, false),
    field(0, 3, 10, false),
    field(1, 4, 11, false),
    field(2, 5, 12, false),
    field(6, 1, 8, true),
    field(7, 2, 9, true),
    field(VX, 0, 7, true),
];
const TEXTURED_SHADE_FADE: &[Field] = &[
    field(3, 6, 14, false),
    field(0, 3, 11, false),
    field(1, 4, 12, false),
    field(2, 5, 13, false),
    field(6, 1, 9, true),
    field(7, 2, 10, true),
    field(8, 7, 15, true),
    field(VX, 0, 8, true),
];

/// The attribute set a span row interpolates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AttributeClass {
    Flat,
    Gouraud,
    GouraudFade,
    Textured,
    TexturedShade,
    TexturedShadeFade,
}

impl AttributeClass {
    /// Identify the class from a row's routines one to five.
    pub(crate) fn from_retail(routines: &[u32; 5]) -> Self {
        let class = match routines[0] {
            0x0047_3890 => Self::Flat,
            0x0047_3CB0 => Self::Gouraud,
            0x0047_4390 => Self::GouraudFade,
            0x0047_4B20 => Self::Textured,
            0x0047_50B0 => Self::TexturedShade,
            0x0047_5940 => Self::TexturedShadeFade,
            other => panic!("unknown span-row edge initialiser {other:08X}"),
        };
        let expected = class.retail_routines();
        // Rows 1/13/25 use 00473900, a byte-equivalent copy of 00473690.
        let clip_matches =
            routines[1] == expected[1] || (class == Self::Flat && routines[1] == 0x0047_3900);
        debug_assert!(clip_matches && routines[2..] == expected[2..]);
        class
    }

    fn retail_routines(self) -> [u32; 5] {
        match self {
            // The second flat row uses the byte-equivalent 00473900 clip.
            Self::Flat => [0x473890, 0x473690, 0x473A80, 0x473810, 0x473BE0],
            Self::Gouraud => [0x473CB0, 0x473D90, 0x473FA0, 0x4741A0, 0x4741F0],
            Self::GouraudFade => [0x474390, 0x4744A0, 0x4746B0, 0x4748E0, 0x474940],
            Self::Textured => [0x474B20, 0x474BE0, 0x474DC0, 0x474F70, 0x474DA0],
            Self::TexturedShade => [0x4750B0, 0x4751E0, 0x475410, 0x475660, 0x4756D0],
            Self::TexturedShadeFade => [0x475940, 0x475A90, 0x475CE0, 0x475F60, 0x475FE0],
        }
    }

    fn fields(self) -> &'static [Field] {
        match self {
            Self::Flat => FLAT,
            Self::Gouraud => GOURAUD,
            Self::GouraudFade => GOURAUD_FADE,
            Self::Textured => TEXTURED,
            Self::TexturedShade => TEXTURED_SHADE,
            Self::TexturedShadeFade => TEXTURED_SHADE_FADE,
        }
    }

    /// Routine 1: start an edge at `from` heading to `to`, one scanline per
    /// step. A negative height leaves the edge untouched.
    pub(crate) fn init_edge(self, edge: &mut Edge, from: &RasterVertex, to: &RasterVertex) {
        let height = to[VY].wrapping_sub(from[VY]);
        if height < 0 {
            return;
        }
        let reciprocal = edge_reciprocal(height);
        for f in self.fields() {
            edge[f.value] = from[f.vertex];
            let delta = to[f.vertex].wrapping_sub(from[f.vertex]);
            edge[f.slope] = if f.rounded {
                slope_q31(delta, reciprocal)
            } else {
                mul_q31(delta, reciprocal)
            };
        }
        edge[EDGE_VALID] = 1;
    }

    /// Routine 2: the vertex where `from`→`to` crosses x = `clip` (16.16).
    pub(crate) fn clip_vertex(
        self,
        out: &mut RasterVertex,
        from: &RasterVertex,
        to: &RasterVertex,
        clip: i32,
    ) {
        let t = clip_ratio(from[VX], to[VX], clip);
        for f in self.fields() {
            if f.vertex != VX {
                out[f.vertex] = mul_q31(to[f.vertex].wrapping_sub(from[f.vertex]), t)
                    .wrapping_add(from[f.vertex]);
            }
        }
        out[VX] = clip;
        out[VY] = mul_q31(to[VY].wrapping_sub(from[VY]), t).wrapping_add(from[VY]);
    }

    /// Routine 3: slide `vertex` along its scanline toward the edge's current
    /// point until it reaches x = `clip`.
    pub(crate) fn clamp_vertex(self, vertex: &mut RasterVertex, edge: &Edge, clip: i32) {
        if edge[0] == clip {
            for f in self.fields() {
                if f.vertex != VX {
                    vertex[f.vertex] = edge[f.value];
                }
            }
        } else {
            let t = clip_ratio(vertex[VX], edge[0], clip);
            for f in self.fields() {
                if f.vertex != VX {
                    vertex[f.vertex] = vertex[f.vertex]
                        .wrapping_add(mul_q31(edge[f.value].wrapping_sub(vertex[f.vertex]), t));
                }
            }
        }
        vertex[VX] = clip;
    }

    /// Routine 4: advance an edge by `lines` scanlines.
    pub(crate) fn step_edge(self, edge: &mut Edge, lines: i32) {
        for f in self.fields() {
            edge[f.value] = edge[f.value].wrapping_add(edge[f.slope].wrapping_mul(lines));
        }
    }

    /// Routine 5: write the edge's current point into a vertex. The y dword
    /// comes from edge `+0x40`, which no routine writes; every caller
    /// overwrites it immediately.
    pub(crate) fn copy_edge(self, vertex: &mut RasterVertex, edge: &Edge) {
        for f in self.fields() {
            vertex[f.vertex] = edge[f.value];
        }
        vertex[VY] = edge[EDGE_Y];
    }
}
