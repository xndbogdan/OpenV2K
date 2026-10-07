//! Graph2D fill-slot handlers of the 16-bpp software table.
//!
//! `FUN_00480D10` installs 43 handlers at Graph2D `+0x101C..+0x10C4`; the
//! primitive queue reaches 32 of them through the record thunks at
//! `0x0047A720..0x0047AB00`. Each handler decodes its packet, binds the
//! material and span row, fills the vertex pool and runs the scan converter.

use super::material::{flags, MaterialId, MaterialSource};
use super::rows::{SpanRow, TexelTable};
use super::scan::scan_polygon;
use super::span::{SpanContext, SpanTarget};
use super::state::{ScanFrame, VU, VV, VX, VY};
use super::SoftwareRaster;

/// A reachable Graph2D fill slot, named by its byte offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FillSlot {
    /// `+0x1038` → `FUN_0047BAB0`: flat triangle.
    FlatTriangle,
    /// `+0x1080` → `FUN_0047E150`: flat quad.
    FlatQuad,
    /// `+0x1098` → `FUN_0047EF10`: textured quad with implicit UVs.
    TexturedQuad,
}

impl FillSlot {
    pub fn offset(self) -> u16 {
        match self {
            Self::FlatTriangle => 0x1038,
            Self::FlatQuad => 0x1080,
            Self::TexturedQuad => 0x1098,
        }
    }

    pub fn from_offset(offset: u16) -> Option<Self> {
        Some(match offset {
            0x1038 => Self::FlatTriangle,
            0x1080 => Self::FlatQuad,
            0x1098 => Self::TexturedQuad,
            _ => return None,
        })
    }

    /// Bytes between the handler's entry ESP and `FUN_00472B20`'s ESP after
    /// its prologue, for this handler's direct call.
    fn scan_depth(self) -> u32 {
        // Handler pushes + the 16 argument bytes + return address + the
        // converter's 0xC8 locals and four saved registers.
        let handler_frame = match self {
            Self::FlatTriangle | Self::FlatQuad => 4,
            Self::TexturedQuad => 0xC,
        };
        handler_frame + 0x10 + 4 + 0xC8 + 0x10
    }
}

/// Little-endian packet field readers.
struct Packet<'a>(&'a [u8]);

impl Packet<'_> {
    fn i16(&self, offset: usize) -> i16 {
        i16::from_le_bytes([self.0[offset], self.0[offset + 1]])
    }
    fn u32(&self, offset: usize) -> u32 {
        u32::from_le_bytes(self.0[offset..offset + 4].try_into().unwrap())
    }
    fn u8(&self, offset: usize) -> u8 {
        self.0[offset]
    }
    /// Corner `index` as `(x << 16, y)`.
    fn corner(&self, index: usize) -> (i32, i32) {
        (
            i32::from(self.i16(4 * index)) << 16,
            i32::from(self.i16(4 * index + 2)),
        )
    }
}

impl SoftwareRaster {
    pub(super) fn dispatch(
        &mut self,
        slot: FillSlot,
        packet: &[u8],
        target: &mut SpanTarget<'_>,
        materials: &dyn MaterialSource,
    ) {
        let packet = Packet(packet);
        let frame = ScanFrame {
            esp: self.handler_esp.wrapping_sub(slot.scan_depth()),
        };
        let count = match slot {
            FillSlot::FlatTriangle => self.flat_polygon(&packet, 3, 0xC, 0x10),
            FillSlot::FlatQuad => self.flat_polygon(&packet, 4, 0x10, 0x14),
            FillSlot::TexturedQuad => self.textured_quad(&packet, materials),
        };
        // Every polygon handler stores 0xE49C before scan conversion.
        self.state.dither = 0xE49C;
        let context = SpanContext {
            materials,
            format: self.format,
            fog: &self.fog,
        };
        scan_polygon(&mut self.state, count, target, &context, frame);
    }

    /// `FUN_0047BAB0` / `FUN_0047E150`: colour dword then flag byte after
    /// the corners; flag 0x08 selects the half-additive flat row.
    fn flat_polygon(
        &mut self,
        packet: &Packet<'_>,
        corners: usize,
        colour: usize,
        flag: usize,
    ) -> usize {
        self.state.table = TexelTable::Raw;
        self.state.word_1c = packet.u32(colour);
        let row = if packet.u8(flag) & 0x08 != 0 { 12 } else { 0 };
        self.state.row = SpanRow::new(TexelTable::Raw, row);
        for index in 0..corners {
            let (x, y) = packet.corner(index);
            self.state.pool[index][VX] = x;
            self.state.pool[index][VY] = y;
        }
        corners
    }

    /// `FUN_0047EF10`: corners A, B, C, D take UVs (0,0), (umax,0),
    /// (umax,vmax), (0,vmax), where umax/vmax are the record's pixel size in
    /// 16.16 minus one.
    fn textured_quad(&mut self, packet: &Packet<'_>, materials: &dyn MaterialSource) -> usize {
        let id: MaterialId = packet.u32(0x10);
        let material = materials.material(id);
        let table = if material.flags & flags::RAW == 0 {
            TexelTable::Indexed
        } else {
            TexelTable::Raw
        };
        self.state.table = table;
        self.state.material = Some(id);
        let keyed = material.flags & flags::KEYED != 0;
        let mut row = if keyed { 8 } else { 4 };
        if material.flags & flags::ADDITIVE != 0 {
            row += 0x18;
        } else if material.flags & flags::HALF_ADDITIVE != 0 {
            row += 0xC;
        }
        self.state.row = SpanRow::new(table, row);
        let u_max = (u32::from(material.width) << 16).wrapping_sub(1) as i32;
        let v_max = (u32::from(material.height) << 16).wrapping_sub(1) as i32;
        let uvs = [(0, 0), (u_max, 0), (u_max, v_max), (0, v_max)];
        for (index, (u, v)) in uvs.into_iter().enumerate() {
            let (x, y) = packet.corner(index);
            let vertex = &mut self.state.pool[index];
            vertex[VX] = x;
            vertex[VY] = y;
            vertex[VU] = u;
            vertex[VV] = v;
        }
        4
    }
}
