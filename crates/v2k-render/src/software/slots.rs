//! Graph2D fill-slot handlers of the 16-bpp software table.
//!
//! `FUN_00480D10` installs 43 handlers at Graph2D `+0x101C..+0x10C4`; the
//! primitive queue reaches 32 of them through the record thunks at
//! `0x0047A720..0x0047AB00` (`+0x101C`, `+0x1068..+0x107C` and
//! `+0x10B0..+0x10BC` have no thunk or call site). Polygon handlers decode
//! their packet, bind material and span row, fill the vertex pool and run
//! the scan converter; the 2D handlers in [`super::direct`] write the
//! surface themselves.
//!
//! Packet offsets are bytes from the queue record's payload (`record + 8`).
//! Material pointers are carried as [`MaterialId`]s in the same four bytes.

use super::material::{flags, MaterialId, MaterialSource, MaterialView};
use super::rows::{SpanRow, TexelTable};
use super::scan::scan_polygon;
use super::span::{SpanContext, SpanTarget};
use super::state::{ScanFrame, VU, VV, VX, VY};
use super::{PixelFormat, SoftwareRaster, WordImageSource};

/// A reachable Graph2D fill slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FillSlot {
    /// `+0x1020` → `FUN_0047AC30`: unclipped word-image blit.
    WordImage,
    /// `+0x1024` → `FUN_0047B360`: flat line.
    Line,
    /// `+0x1028` → `FUN_0047B6C0`: two-colour faded line.
    FadedLine,
    /// `+0x102C` → `FUN_0047AB20`: set the clip rectangle.
    SetClip,
    /// `+0x1030` → `FUN_0047AD90`: unscaled sprite.
    Sprite,
    /// `+0x1034` → `FUN_0047B9F0`: fill whole rows.
    FillRows,
    /// `+0x1038` → `FUN_0047BAB0`: flat triangle.
    FlatTriangle,
    /// `+0x103C` → `FUN_0047BB70`: triangle faded toward a fog colour.
    FogTriangle,
    /// `+0x1040` → `FUN_0047BD80`: tinted flat triangle.
    TintTriangle,
    /// `+0x1044` → `FUN_0047BE40`: lit triangle faded toward a fog colour.
    LitFogTriangle,
    /// `+0x1048` → `FUN_0047C150`: Gouraud triangle.
    GouraudTriangle,
    /// `+0x104C` → `FUN_0047C2F0`: per-vertex lit triangle with fog.
    VertexLitFogTriangle,
    /// `+0x1050` → `FUN_0047C600`: textured triangle.
    TexturedTriangle,
    /// `+0x1054` → `FUN_0047C7C0`: textured triangle with fog.
    TexturedFogTriangle,
    /// `+0x1058` → `FUN_0047CBE0`: textured triangle, one palette row.
    TexturedRowTriangle,
    /// `+0x105C` → `FUN_0047CD30`: textured triangle, one shade, fog.
    TexturedShadeFogTriangle,
    /// `+0x1060` → `FUN_0047CFA0`: textured triangle, per-vertex shade.
    ShadedTriangle,
    /// `+0x1064` → `FUN_0047D240`: per-vertex shade and fog.
    ShadedFogTriangle,
    /// `+0x1080` → `FUN_0047E150`: flat quad.
    FlatQuad,
    /// `+0x1084` → `FUN_0047E220`: quad faded toward a fog colour.
    FogQuad,
    /// `+0x1088` → `FUN_0047E490`: tinted flat quad.
    TintQuad,
    /// `+0x108C` → `FUN_0047E560`: lit quad faded toward a fog colour.
    LitFogQuad,
    /// `+0x1090` → `FUN_0047E930`: Gouraud quad.
    GouraudQuad,
    /// `+0x1094` → `FUN_0047EB40`: per-vertex lit quad with fog.
    VertexLitFogQuad,
    /// `+0x1098` → `FUN_0047EF10`: textured quad.
    TexturedQuad,
    /// `+0x109C` → `FUN_0047F060`: textured quad with fog.
    TexturedFogQuad,
    /// `+0x10A0` → `FUN_0047F320`: textured quad, one palette row.
    TexturedRowQuad,
    /// `+0x10A4` → `FUN_0047F480`: textured quad, one shade, fog.
    TexturedShadeFogQuad,
    /// `+0x10A8` → `FUN_0047F750`: textured quad, per-vertex shade.
    ShadedQuad,
    /// `+0x10AC` → `FUN_0047F9E0`: per-vertex shade and fog.
    ShadedFogQuad,
    /// `+0x10C0` → `FUN_004804C0`: explicit UVs, per-vertex shade.
    MappedShadedQuad,
    /// `+0x10C4` → `FUN_00480790`: explicit UVs, per-vertex shade, fog.
    MappedShadedFogQuad,
}

impl FillSlot {
    pub const ALL: [FillSlot; 32] = [
        Self::WordImage,
        Self::Line,
        Self::FadedLine,
        Self::SetClip,
        Self::Sprite,
        Self::FillRows,
        Self::FlatTriangle,
        Self::FogTriangle,
        Self::TintTriangle,
        Self::LitFogTriangle,
        Self::GouraudTriangle,
        Self::VertexLitFogTriangle,
        Self::TexturedTriangle,
        Self::TexturedFogTriangle,
        Self::TexturedRowTriangle,
        Self::TexturedShadeFogTriangle,
        Self::ShadedTriangle,
        Self::ShadedFogTriangle,
        Self::FlatQuad,
        Self::FogQuad,
        Self::TintQuad,
        Self::LitFogQuad,
        Self::GouraudQuad,
        Self::VertexLitFogQuad,
        Self::TexturedQuad,
        Self::TexturedFogQuad,
        Self::TexturedRowQuad,
        Self::TexturedShadeFogQuad,
        Self::ShadedQuad,
        Self::ShadedFogQuad,
        Self::MappedShadedQuad,
        Self::MappedShadedFogQuad,
    ];

    /// Graph2D byte offset of the slot.
    pub fn offset(self) -> u16 {
        match self {
            Self::WordImage => 0x1020,
            Self::Line => 0x1024,
            Self::FadedLine => 0x1028,
            Self::SetClip => 0x102C,
            Self::Sprite => 0x1030,
            Self::FillRows => 0x1034,
            Self::FlatTriangle => 0x1038,
            Self::FogTriangle => 0x103C,
            Self::TintTriangle => 0x1040,
            Self::LitFogTriangle => 0x1044,
            Self::GouraudTriangle => 0x1048,
            Self::VertexLitFogTriangle => 0x104C,
            Self::TexturedTriangle => 0x1050,
            Self::TexturedFogTriangle => 0x1054,
            Self::TexturedRowTriangle => 0x1058,
            Self::TexturedShadeFogTriangle => 0x105C,
            Self::ShadedTriangle => 0x1060,
            Self::ShadedFogTriangle => 0x1064,
            Self::FlatQuad => 0x1080,
            Self::FogQuad => 0x1084,
            Self::TintQuad => 0x1088,
            Self::LitFogQuad => 0x108C,
            Self::GouraudQuad => 0x1090,
            Self::VertexLitFogQuad => 0x1094,
            Self::TexturedQuad => 0x1098,
            Self::TexturedFogQuad => 0x109C,
            Self::TexturedRowQuad => 0x10A0,
            Self::TexturedShadeFogQuad => 0x10A4,
            Self::ShadedQuad => 0x10A8,
            Self::ShadedFogQuad => 0x10AC,
            Self::MappedShadedQuad => 0x10C0,
            Self::MappedShadedFogQuad => 0x10C4,
        }
    }

    pub fn from_offset(offset: u16) -> Option<Self> {
        Self::ALL.into_iter().find(|slot| slot.offset() == offset)
    }

    /// Bytes from the handler's entry ESP to `FUN_00472B20`'s ESP after its
    /// prologue (handler frame, call arguments, return address, the
    /// converter's 0xC8 locals and four saved registers), measured from the
    /// unchanged handlers. `None` for the 2D slots.
    pub fn scan_depth(self) -> Option<u32> {
        Some(match self {
            Self::FlatTriangle
            | Self::TintTriangle
            | Self::GouraudTriangle
            | Self::FlatQuad
            | Self::TintQuad
            | Self::GouraudQuad => 0xF0,
            Self::TexturedTriangle
            | Self::TexturedRowTriangle
            | Self::TexturedQuad
            | Self::TexturedRowQuad => 0xF8,
            Self::TexturedFogTriangle
            | Self::TexturedShadeFogTriangle
            | Self::ShadedTriangle
            | Self::ShadedFogTriangle
            | Self::TexturedFogQuad
            | Self::TexturedShadeFogQuad
            | Self::ShadedQuad
            | Self::ShadedFogQuad
            | Self::MappedShadedQuad
            | Self::MappedShadedFogQuad => 0xFC,
            Self::FogTriangle | Self::FogQuad => 0x108,
            Self::LitFogTriangle
            | Self::VertexLitFogTriangle
            | Self::LitFogQuad
            | Self::VertexLitFogQuad => 0x110,
            Self::WordImage
            | Self::Line
            | Self::FadedLine
            | Self::SetClip
            | Self::Sprite
            | Self::FillRows => return None,
        })
    }
}

/// Little-endian packet field readers.
pub(super) struct Packet<'a>(pub &'a [u8]);

impl Packet<'_> {
    pub(super) fn i16(&self, offset: usize) -> i16 {
        i16::from_le_bytes([self.0[offset], self.0[offset + 1]])
    }
    pub(super) fn u16(&self, offset: usize) -> u16 {
        u16::from_le_bytes([self.0[offset], self.0[offset + 1]])
    }
    pub(super) fn u32(&self, offset: usize) -> u32 {
        u32::from_le_bytes(self.0[offset..offset + 4].try_into().unwrap())
    }
    pub(super) fn i32(&self, offset: usize) -> i32 {
        self.u32(offset) as i32
    }
    pub(super) fn u8(&self, offset: usize) -> u8 {
        self.0[offset]
    }
    fn bytes4(&self, offset: usize) -> [u8; 4] {
        self.0[offset..offset + 4].try_into().unwrap()
    }
}

/// `f + (f >> 7)`: a fog byte as 0..=256.
fn fog_weight(byte: u8) -> i32 {
    i32::from(byte) + (i32::from(byte) >> 7)
}

/// 8-bit channels of a display-format colour as the handlers extract them:
/// `(c >> (red+1)) & 0xF8`, `(c >> (green+1)) & 0xF8`, `(c << (blue-1)) & 0xF8`.
pub(super) fn channels(colour: u32, format: PixelFormat) -> [i32; 3] {
    [
        ((colour >> ((u32::from(format.red_shift) + 1) & 31)) & 0xF8) as i32,
        ((colour >> ((u32::from(format.green_shift) + 1) & 31)) & 0xF8) as i32,
        (colour.wrapping_shl(u32::from(format.blue_shift).wrapping_sub(1) & 31) & 0xF8) as i32,
    ]
}

/// `(byte << 16) + 0x8000`.
fn byte_attribute(byte: u8) -> i32 {
    (i32::from(byte) << 16) + 0x8000
}

/// `((255 - f) * s + 0x80) << 8`.
fn shade_level(fog: u8, shade: u8) -> i32 {
    ((0xFF - i32::from(fog)) * i32::from(shade) + 0x80) * 0x100
}

/// Which colour bytes a fog-colour handler adds before fading.
#[derive(Clone, Copy)]
enum Lit {
    None,
    /// One RGB byte triple for every vertex.
    Shared(usize),
    /// One RGB byte triple per vertex, four bytes apart.
    PerVertex(usize),
}

/// Shade source of a fogged textured handler.
#[derive(Clone, Copy)]
enum Shade {
    /// Palette row 28 scaled by `1 - fog`: `(255-f) * 0x1C00 + 0x8000`.
    Unlit,
    Shared(usize),
    PerVertex(usize),
}

impl SoftwareRaster {
    pub(super) fn dispatch(
        &mut self,
        slot: FillSlot,
        packet: &[u8],
        target: &mut SpanTarget<'_>,
        materials: &dyn MaterialSource,
        images: &dyn WordImageSource,
    ) {
        let packet = Packet(packet);
        let count = match slot {
            FillSlot::WordImage => return self.word_image(&packet, target, images),
            FillSlot::Line => return self.line(&packet, target),
            FillSlot::FadedLine => return self.faded_line(&packet, target),
            FillSlot::SetClip => return self.set_clip_packet(&packet, target),
            FillSlot::Sprite => return self.sprite(&packet, target, materials),
            FillSlot::FillRows => return self.fill_rows(&packet, target),
            FillSlot::FlatTriangle => self.flat(&packet, 3, 0xC, 0x10),
            FillSlot::FlatQuad => self.flat(&packet, 4, 0x10, 0x14),
            FillSlot::TintTriangle => self.tint(&packet, 3, 0xC),
            FillSlot::TintQuad => self.tint(&packet, 4, 0x10),
            FillSlot::FogTriangle => self.fog_colour(&packet, 3, 0xC, Lit::None, 0x10, 0x14, 0x18),
            FillSlot::FogQuad => self.fog_colour(&packet, 4, 0x10, Lit::None, 0x14, 0x18, 0x1C),
            FillSlot::LitFogTriangle => {
                self.fog_colour(&packet, 3, 0xC, Lit::Shared(0x10), 0x14, 0x18, 0x1C)
            }
            FillSlot::LitFogQuad => {
                self.fog_colour(&packet, 4, 0x10, Lit::Shared(0x14), 0x18, 0x1C, 0x20)
            }
            FillSlot::VertexLitFogTriangle => {
                self.fog_colour(&packet, 3, 0xC, Lit::PerVertex(0x10), 0x1C, 0x20, 0x24)
            }
            FillSlot::VertexLitFogQuad => {
                self.fog_colour(&packet, 4, 0x10, Lit::PerVertex(0x14), 0x24, 0x28, 0x2C)
            }
            FillSlot::GouraudTriangle => self.gouraud(&packet, 3, 0xC),
            FillSlot::GouraudQuad => self.gouraud(&packet, 4, 0x10),
            FillSlot::TexturedTriangle => self.textured(&packet, 3, materials, 4, None),
            FillSlot::TexturedQuad => self.textured(&packet, 4, materials, 4, None),
            FillSlot::TexturedRowTriangle => self.textured(&packet, 3, materials, 5, Some(0x10)),
            FillSlot::TexturedRowQuad => self.textured(&packet, 4, materials, 5, Some(0x14)),
            FillSlot::TexturedFogTriangle => {
                self.textured_fog(&packet, 3, materials, 0x14, Shade::Unlit, 0x18)
            }
            FillSlot::TexturedFogQuad => {
                self.textured_fog(&packet, 4, materials, 0x18, Shade::Unlit, 0x1C)
            }
            FillSlot::TexturedShadeFogTriangle => {
                self.textured_fog(&packet, 3, materials, 0x18, Shade::Shared(0x13), 0x1C)
            }
            FillSlot::TexturedShadeFogQuad => {
                self.textured_fog(&packet, 4, materials, 0x1C, Shade::Shared(0x17), 0x20)
            }
            FillSlot::ShadedFogTriangle => {
                self.textured_fog(&packet, 3, materials, 0x20, Shade::PerVertex(0x13), 0x24)
            }
            FillSlot::ShadedFogQuad => {
                self.textured_fog(&packet, 4, materials, 0x28, Shade::PerVertex(0x17), 0x2C)
            }
            FillSlot::MappedShadedFogQuad => self.mapped_shaded_fog(&packet, materials),
            FillSlot::ShadedTriangle => self.shaded(&packet, 3, materials, None),
            FillSlot::ShadedQuad => self.shaded(&packet, 4, materials, None),
            FillSlot::MappedShadedQuad => self.shaded(&packet, 4, materials, Some(0x28)),
        };
        // Every polygon handler stores 0xE49C before scan conversion.
        self.state.dither = 0xE49C;
        let depth = slot.scan_depth().expect("polygon slot");
        let frame = ScanFrame {
            esp: self.handler_esp.wrapping_sub(depth),
        };
        let context = SpanContext {
            materials,
            format: self.format,
            fog: &self.fog,
        };
        scan_polygon(&mut self.state, count, target, &context, frame);
    }

    fn set_corners(&mut self, packet: &Packet<'_>, corners: usize) {
        for index in 0..corners {
            let vertex = &mut self.state.pool[index];
            vertex[VX] = i32::from(packet.i16(4 * index)) << 16;
            vertex[VY] = i32::from(packet.i16(4 * index + 2));
        }
    }

    /// Graph2D `+0x1018` follows the material's texel format.
    fn bind_table(&mut self, material: &MaterialView<'_>) {
        self.state.table = if material.flags & flags::RAW == 0 {
            TexelTable::Indexed
        } else {
            TexelTable::Raw
        };
    }

    /// The `cVar` row selection of the unshaded textured handlers: `base`
    /// (4 or 5), +4 when keyed, then +0x18 for additive or +0xC for
    /// half-additive, in the table at Graph2D `+0x1018`.
    fn textured_row(&self, material: &MaterialView<'_>, base: usize) -> SpanRow {
        let mut row = base;
        if material.flags & flags::KEYED != 0 {
            row += 4;
        }
        if material.flags & flags::ADDITIVE != 0 {
            row += 0x18;
        } else if material.flags & flags::HALF_ADDITIVE != 0 {
            row += 0xC;
        }
        SpanRow::new(self.state.table, row)
    }

    /// Fogged handlers pick their row directly from the material's table:
    /// 7, keyed 11, half 19/23, additive 31/35. Graph2D `+0x1018` is not
    /// written.
    fn fog_row(material: &MaterialView<'_>) -> SpanRow {
        let table = if material.flags & flags::RAW == 0 {
            TexelTable::Indexed
        } else {
            TexelTable::Raw
        };
        let keyed = material.flags & flags::KEYED != 0;
        let row = match (
            material.flags & flags::ADDITIVE != 0,
            material.flags & flags::HALF_ADDITIVE != 0,
            keyed,
        ) {
            (true, _, false) => 31,
            (true, _, true) => 35,
            (false, true, false) => 19,
            (false, true, true) => 23,
            (false, false, false) => 7,
            (false, false, true) => 11,
        };
        SpanRow::new(table, row)
    }

    /// `FUN_0047BAB0` / `FUN_0047E150`.
    fn flat(&mut self, packet: &Packet<'_>, corners: usize, colour: usize, flag: usize) -> usize {
        self.state.table = TexelTable::Raw;
        self.state.word_1c = packet.u32(colour);
        let row = if packet.u8(flag) & 0x08 != 0 { 12 } else { 0 };
        self.state.row = SpanRow::new(TexelTable::Raw, row);
        self.set_corners(packet, corners);
        corners
    }

    /// `FUN_0047BD80` / `FUN_0047E490`: base colour, tint word, flag byte.
    fn tint(&mut self, packet: &Packet<'_>, corners: usize, colour: usize) -> usize {
        self.state.table = TexelTable::Raw;
        self.state.word_1c = packet.u32(colour);
        self.state.word_18 = packet.u32(colour + 4);
        let row = if packet.u8(colour + 8) & 0x08 != 0 {
            13
        } else {
            1
        };
        self.state.row = SpanRow::new(TexelTable::Raw, row);
        self.set_corners(packet, corners);
        corners
    }

    /// `FUN_0047BB70` family: each vertex lerps the (optionally lit) colour
    /// toward the fog colour by its fog byte; channels are stored as
    /// `(c * 256 + f * (fog - c)) * 512`. Row 2, or 14 with flag 0x08, of the
    /// raw table; Graph2D `+0x1018` is not written.
    #[allow(clippy::too_many_arguments)]
    fn fog_colour(
        &mut self,
        packet: &Packet<'_>,
        corners: usize,
        colour: usize,
        lit: Lit,
        flag: usize,
        fog: usize,
        fog_bytes: usize,
    ) -> usize {
        let base = channels(packet.u32(colour), self.format);
        let target = channels(packet.u32(fog), self.format);
        let row = if packet.u8(flag) & 0x08 != 0 { 14 } else { 2 };
        self.state.row = SpanRow::new(TexelTable::Raw, row);
        for index in 0..corners {
            let lit_bytes = match lit {
                Lit::None => None,
                Lit::Shared(offset) => Some(offset),
                Lit::PerVertex(offset) => Some(offset + 4 * index),
            };
            let weight = fog_weight(packet.u8(fog_bytes + index));
            let vertex = &mut self.state.pool[index];
            for channel in 0..3 {
                let start = match lit_bytes {
                    Some(offset) => base[channel] + i32::from(packet.u8(offset + channel)),
                    None => base[channel],
                };
                vertex[channel] = target[channel]
                    .wrapping_sub(start)
                    .wrapping_mul(weight)
                    .wrapping_add(start.wrapping_mul(256))
                    .wrapping_mul(0x200);
            }
        }
        self.set_corners(packet, corners);
        corners
    }

    /// `FUN_0047C150` / `FUN_0047E930`: per-vertex RGBA bytes as 16.16.
    fn gouraud(&mut self, packet: &Packet<'_>, corners: usize, colour: usize) -> usize {
        self.state.table = TexelTable::Raw;
        self.state.word_1c = packet.u32(colour);
        let flag = colour + 4 + 4 * corners;
        let row = if packet.u8(flag) & 0x08 != 0 { 14 } else { 2 };
        self.state.row = SpanRow::new(TexelTable::Raw, row);
        for index in 0..corners {
            let bytes = packet.bytes4(colour + 4 + 4 * index);
            let vertex = &mut self.state.pool[index];
            for channel in 0..4 {
                vertex[channel] = byte_attribute(bytes[channel]);
            }
        }
        self.set_corners(packet, corners);
        corners
    }

    /// Implicit UVs: A (0,0), B (umax,0), C (umax,vmax), D (0,vmax).
    fn implicit_uvs(&mut self, material: &MaterialView<'_>, corners: usize) {
        let u_max = (u32::from(material.width) << 16).wrapping_sub(1) as i32;
        let v_max = (u32::from(material.height) << 16).wrapping_sub(1) as i32;
        let uvs = [(0, 0), (u_max, 0), (u_max, v_max), (0, v_max)];
        for (index, (u, v)) in uvs.into_iter().take(corners).enumerate() {
            self.state.pool[index][VU] = u;
            self.state.pool[index][VV] = v;
        }
    }

    /// `FUN_0047C600`/`FUN_0047EF10` (base row 4) and
    /// `FUN_0047CBE0`/`FUN_0047F320` (base row 5 with a row word).
    fn textured(
        &mut self,
        packet: &Packet<'_>,
        corners: usize,
        materials: &dyn MaterialSource,
        base_row: usize,
        row_word: Option<usize>,
    ) -> usize {
        let material_at = if corners == 3 { 0xC } else { 0x10 };
        let id: MaterialId = packet.u32(material_at);
        let material = materials.material(id);
        self.bind_table(&material);
        self.state.material = Some(id);
        if let Some(offset) = row_word {
            self.state.word_18 = packet.u32(offset);
        }
        self.state.row = self.textured_row(&material, base_row);
        self.set_corners(packet, corners);
        self.implicit_uvs(&material, corners);
        corners
    }

    /// The fogged textured handlers: unlit (`FUN_0047C7C0`/`FUN_0047F060`),
    /// one shade byte (`FUN_0047CD30`/`FUN_0047F480`) or per-vertex shade
    /// bytes (`FUN_0047D240`/`FUN_0047F9E0`). The fog level is
    /// `(f << 16) + 0x8000`.
    fn textured_fog(
        &mut self,
        packet: &Packet<'_>,
        corners: usize,
        materials: &dyn MaterialSource,
        fog: usize,
        shade: Shade,
        fog_bytes: usize,
    ) -> usize {
        let material_at = if corners == 3 { 0xC } else { 0x10 };
        let id: MaterialId = packet.u32(material_at);
        let material = materials.material(id);
        self.state.row = Self::fog_row(&material);
        self.state.material = Some(id);
        self.fog.select(self.format, packet.u32(fog));
        for index in 0..corners {
            let f = packet.u8(fog_bytes + index);
            let level = match shade {
                Shade::Unlit => i32::from(f).wrapping_mul(-0x1C00) + 0x1C_6400,
                Shade::Shared(offset) => shade_level(f, packet.u8(offset)),
                Shade::PerVertex(offset) => shade_level(f, packet.u8(offset + 4 * index)),
            };
            let vertex = &mut self.state.pool[index];
            vertex[3] = level;
            vertex[8] = byte_attribute(f);
        }
        self.set_corners(packet, corners);
        self.implicit_uvs(&material, corners);
        corners
    }

    /// `FUN_00480790`: explicit UVs at `+0x28`, shade bytes, fog colour at
    /// `+0x48` and fog bytes at `+0x4C`.
    fn mapped_shaded_fog(&mut self, packet: &Packet<'_>, materials: &dyn MaterialSource) -> usize {
        let id: MaterialId = packet.u32(0x10);
        let material = materials.material(id);
        self.state.row = Self::fog_row(&material);
        self.state.material = Some(id);
        self.fog.select(self.format, packet.u32(0x48));
        for index in 0..4 {
            let f = packet.u8(0x4C + index);
            let vertex = &mut self.state.pool[index];
            vertex[3] = shade_level(f, packet.u8(0x17 + 4 * index));
            vertex[8] = byte_attribute(f);
            vertex[VU] = packet.i32(0x28 + 8 * index);
            vertex[VV] = packet.i32(0x2C + 8 * index);
        }
        self.set_corners(packet, 4);
        4
    }

    /// `FUN_0047CFA0` / `FUN_0047F750` / `FUN_004804C0`: per-vertex shade
    /// words. Equal words use the one-row filler with that word at Graph2D
    /// `+0x18`; otherwise `FUN_0047D1A0` selects shaded row 6 (+4 keyed,
    /// +12 half, +24 additive) and each vertex takes the word's bytes as
    /// 16.16 attributes.
    fn shaded(
        &mut self,
        packet: &Packet<'_>,
        corners: usize,
        materials: &dyn MaterialSource,
        uv_at: Option<usize>,
    ) -> usize {
        let material_at = if corners == 3 { 0xC } else { 0x10 };
        let id: MaterialId = packet.u32(material_at);
        let material = materials.material(id);
        self.bind_table(&material);
        self.state.material = Some(id);
        let words: Vec<u32> = (0..corners)
            .map(|index| packet.u32(material_at + 4 + 4 * index))
            .collect();
        if words.windows(2).all(|pair| pair[0] == pair[1]) {
            self.state.word_18 = words[0];
            self.state.row = self.textured_row(&material, 5);
        } else {
            let mut row = 6;
            if material.flags & flags::KEYED != 0 {
                row += 4;
            }
            if material.flags & flags::ADDITIVE != 0 {
                row += 24;
            } else if material.flags & flags::HALF_ADDITIVE != 0 {
                row += 12;
            }
            self.state.row = SpanRow::new(self.state.table, row);
            for (index, word) in words.iter().enumerate() {
                let bytes = word.to_le_bytes();
                let vertex = &mut self.state.pool[index];
                for channel in 0..4 {
                    vertex[channel] = byte_attribute(bytes[channel]);
                }
            }
        }
        self.set_corners(packet, corners);
        match uv_at {
            Some(offset) => {
                for index in 0..corners {
                    self.state.pool[index][VU] = packet.i32(offset + 8 * index);
                    self.state.pool[index][VV] = packet.i32(offset + 4 + 8 * index);
                }
            }
            None => self.implicit_uvs(&material, corners),
        }
        corners
    }
}
