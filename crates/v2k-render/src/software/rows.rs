//! The retail span-row tables and their dispatch.
//!
//! `FUN_00480D10` stores the address of one of two static 36-row tables at
//! Graph2D `+0x1018`: indexed texels at `0x004D55A0`, raw RGB texels at
//! `0x004D5900`. A slot handler picks a row (`table + row * 24`) and the scan
//! converter calls its six routines: the span filler, then the edge
//! initialiser, x-clip interpolator, clip clamp, multi-line step and
//! edge-to-vertex copy of one attribute class.
//!
//! Rows come in three blend groups of twelve (opaque, half-additive,
//! additive). The tables are kept as the retail routine addresses because the
//! dispatch and its quirks are data: additive rows 26/27 and 30/31 share a
//! filler, raw additive rows 28/29/32/33 reuse the half-additive raw fillers,
//! and the span fillers' unchecked reciprocal lookups read these same words
//! for spans wider than 641 pixels (see [`super::fixed::span_reciprocal`]).

use super::attr::AttributeClass;
use super::span::SpanFiller;

pub(crate) const ROW_COUNT: usize = 36;

/// Which static table Graph2D `+0x1018` points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TexelTable {
    /// `PTR_004D5C60`: indexed texels through a palette.
    Indexed,
    /// `PTR_004D5C64`: raw display-format texel words. Untextured handlers
    /// also install this table.
    Raw,
}

/// One row of one table, as stored at Graph2D `+0x20`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpanRow {
    pub table: TexelTable,
    pub index: usize,
}

impl SpanRow {
    pub fn new(table: TexelTable, index: usize) -> Self {
        assert!(
            index < ROW_COUNT,
            "span row {index} outside the retail table"
        );
        Self { table, index }
    }

    fn routines(self) -> &'static [u32; 6] {
        match self.table {
            TexelTable::Indexed => &INDEXED_ROWS[self.index],
            TexelTable::Raw => &RAW_ROWS[self.index],
        }
    }

    pub(crate) fn filler(self) -> SpanFiller {
        SpanFiller::from_retail(self.routines()[0])
    }

    pub(crate) fn class(self) -> AttributeClass {
        AttributeClass::from_retail(self.routines()[1..].try_into().unwrap())
    }
}

/// Indexed-texel table at `0x004D55A0`.
pub(crate) const INDEXED_ROWS: [[u32; 6]; ROW_COUNT] = [
    [
        0x00473830, 0x00473890, 0x00473690, 0x00473A80, 0x00473810, 0x00473BE0,
    ],
    [
        0x00473C00, 0x00473890, 0x00473900, 0x00473A80, 0x00473810, 0x00473BE0,
    ],
    [
        0x00474220, 0x00473CB0, 0x00473D90, 0x00473FA0, 0x004741A0, 0x004741F0,
    ],
    [
        0x00474980, 0x00474390, 0x004744A0, 0x004746B0, 0x004748E0, 0x00474940,
    ],
    [
        0x004782B0, 0x00474B20, 0x00474BE0, 0x00474DC0, 0x00474F70, 0x00474DA0,
    ],
    [
        0x004783E0, 0x00474B20, 0x00474BE0, 0x00474DC0, 0x00474F70, 0x00474DA0,
    ],
    [
        0x00478510, 0x004750B0, 0x004751E0, 0x00475410, 0x00475660, 0x004756D0,
    ],
    [
        0x004786A0, 0x00475940, 0x00475A90, 0x00475CE0, 0x00475F60, 0x00475FE0,
    ],
    [
        0x004788D0, 0x00474B20, 0x00474BE0, 0x00474DC0, 0x00474F70, 0x00474DA0,
    ],
    [
        0x00478A20, 0x00474B20, 0x00474BE0, 0x00474DC0, 0x00474F70, 0x00474DA0,
    ],
    [
        0x00478B60, 0x004750B0, 0x004751E0, 0x00475410, 0x00475660, 0x004756D0,
    ],
    [
        0x00478D00, 0x00475940, 0x00475A90, 0x00475CE0, 0x00475F60, 0x00475FE0,
    ],
    [
        0x00476900, 0x00473890, 0x00473690, 0x00473A80, 0x00473810, 0x00473BE0,
    ],
    [
        0x00476990, 0x00473890, 0x00473900, 0x00473A80, 0x00473810, 0x00473BE0,
    ],
    [
        0x00476A80, 0x00473CB0, 0x00473D90, 0x00473FA0, 0x004741A0, 0x004741F0,
    ],
    [
        0x00476BF0, 0x00474390, 0x004744A0, 0x004746B0, 0x004748E0, 0x00474940,
    ],
    [
        0x00478F60, 0x00474B20, 0x00474BE0, 0x00474DC0, 0x00474F70, 0x00474DA0,
    ],
    [
        0x004790C0, 0x00474B20, 0x00474BE0, 0x00474DC0, 0x00474F70, 0x00474DA0,
    ],
    [
        0x00479220, 0x004750B0, 0x004751E0, 0x00475410, 0x00475660, 0x004756D0,
    ],
    [
        0x00479490, 0x00475940, 0x00475A90, 0x00475CE0, 0x00475F60, 0x00475FE0,
    ],
    [
        0x004796D0, 0x00474B20, 0x00474BE0, 0x00474DC0, 0x00474F70, 0x00474DA0,
    ],
    [
        0x00479840, 0x00474B20, 0x00474BE0, 0x00474DC0, 0x00474F70, 0x00474DA0,
    ],
    [
        0x004799B0, 0x004750B0, 0x004751E0, 0x00475410, 0x00475660, 0x004756D0,
    ],
    [
        0x00479B90, 0x00475940, 0x00475A90, 0x00475CE0, 0x00475F60, 0x00475FE0,
    ],
    [
        0x00477AD0, 0x00473890, 0x00473690, 0x00473A80, 0x00473810, 0x00473BE0,
    ],
    [
        0x00477B80, 0x00473890, 0x00473900, 0x00473A80, 0x00473810, 0x00473BE0,
    ],
    [
        0x00477C60, 0x00473CB0, 0x00473D90, 0x00473FA0, 0x004741A0, 0x004741F0,
    ],
    [
        0x00477C60, 0x00473CB0, 0x00473D90, 0x00473FA0, 0x004741A0, 0x004741F0,
    ],
    [
        0x00479E00, 0x00474B20, 0x00474BE0, 0x00474DC0, 0x00474F70, 0x00474DA0,
    ],
    [
        0x00479F60, 0x00474B20, 0x00474BE0, 0x00474DC0, 0x00474F70, 0x00474DA0,
    ],
    [
        0x0047A0B0, 0x004750B0, 0x004751E0, 0x00475410, 0x00475660, 0x004756D0,
    ],
    [
        0x0047A0B0, 0x004750B0, 0x004751E0, 0x00475410, 0x00475660, 0x004756D0,
    ],
    [
        0x0047A270, 0x00474B20, 0x00474BE0, 0x00474DC0, 0x00474F70, 0x00474DA0,
    ],
    [
        0x0047A3E0, 0x00474B20, 0x00474BE0, 0x00474DC0, 0x00474F70, 0x00474DA0,
    ],
    [
        0x0047A540, 0x004750B0, 0x004751E0, 0x00475410, 0x00475660, 0x004756D0,
    ],
    [
        0x0047A540, 0x004750B0, 0x004751E0, 0x00475410, 0x00475660, 0x004756D0,
    ],
];

/// Raw-texel table at `0x004D5900`.
pub(crate) const RAW_ROWS: [[u32; 6]; ROW_COUNT] = [
    [
        0x00473830, 0x00473890, 0x00473690, 0x00473A80, 0x00473810, 0x00473BE0,
    ],
    [
        0x00473C00, 0x00473890, 0x00473900, 0x00473A80, 0x00473810, 0x00473BE0,
    ],
    [
        0x00474220, 0x00473CB0, 0x00473D90, 0x00473FA0, 0x004741A0, 0x004741F0,
    ],
    [
        0x00474980, 0x00474390, 0x004744A0, 0x004746B0, 0x004748E0, 0x00474940,
    ],
    [
        0x00474FB0, 0x00474B20, 0x00474BE0, 0x00474DC0, 0x00474F70, 0x00474DA0,
    ],
    [
        0x00474FB0, 0x00474B20, 0x00474BE0, 0x00474DC0, 0x00474F70, 0x00474DA0,
    ],
    [
        0x00475710, 0x004750B0, 0x004751E0, 0x00475410, 0x00475660, 0x004756D0,
    ],
    [
        0x00476030, 0x00475940, 0x00475A90, 0x00475CE0, 0x00475F60, 0x00475FE0,
    ],
    [
        0x004762E0, 0x00474B20, 0x00474BE0, 0x00474DC0, 0x00474F70, 0x00474DA0,
    ],
    [
        0x004762E0, 0x00474B20, 0x00474BE0, 0x00474DC0, 0x00474F70, 0x00474DA0,
    ],
    [
        0x004763F0, 0x004750B0, 0x004751E0, 0x00475410, 0x00475660, 0x004756D0,
    ],
    [
        0x00476640, 0x00475940, 0x00475A90, 0x00475CE0, 0x00475F60, 0x00475FE0,
    ],
    [
        0x00476900, 0x00473890, 0x00473690, 0x00473A80, 0x00473810, 0x00473BE0,
    ],
    [
        0x00476990, 0x00473890, 0x00473900, 0x00473A80, 0x00473810, 0x00473BE0,
    ],
    [
        0x00476A80, 0x00473CB0, 0x00473D90, 0x00473FA0, 0x004741A0, 0x004741F0,
    ],
    [
        0x00476BF0, 0x00474390, 0x004744A0, 0x004746B0, 0x004748E0, 0x00474940,
    ],
    [
        0x00476D90, 0x00474B20, 0x00474BE0, 0x00474DC0, 0x00474F70, 0x00474DA0,
    ],
    [
        0x00476D90, 0x00474B20, 0x00474BE0, 0x00474DC0, 0x00474F70, 0x00474DA0,
    ],
    [
        0x00476ED0, 0x004750B0, 0x004751E0, 0x00475410, 0x00475660, 0x004756D0,
    ],
    [
        0x00477140, 0x00475940, 0x00475A90, 0x00475CE0, 0x00475F60, 0x00475FE0,
    ],
    [
        0x00477420, 0x00474B20, 0x00474BE0, 0x00474DC0, 0x00474F70, 0x00474DA0,
    ],
    [
        0x00477420, 0x00474B20, 0x00474BE0, 0x00474DC0, 0x00474F70, 0x00474DA0,
    ],
    [
        0x00477560, 0x004750B0, 0x004751E0, 0x00475410, 0x00475660, 0x004756D0,
    ],
    [
        0x004777D0, 0x00475940, 0x00475A90, 0x00475CE0, 0x00475F60, 0x00475FE0,
    ],
    [
        0x00477AD0, 0x00473890, 0x00473690, 0x00473A80, 0x00473810, 0x00473BE0,
    ],
    [
        0x00477B80, 0x00473890, 0x00473900, 0x00473A80, 0x00473810, 0x00473BE0,
    ],
    [
        0x00477C60, 0x00473CB0, 0x00473D90, 0x00473FA0, 0x004741A0, 0x004741F0,
    ],
    [
        0x00477C60, 0x00473CB0, 0x00473D90, 0x00473FA0, 0x004741A0, 0x004741F0,
    ],
    [
        0x00476D90, 0x00474B20, 0x00474BE0, 0x00474DC0, 0x00474F70, 0x00474DA0,
    ],
    [
        0x00476D90, 0x00474B20, 0x00474BE0, 0x00474DC0, 0x00474F70, 0x00474DA0,
    ],
    [
        0x00477DD0, 0x004750B0, 0x004751E0, 0x00475410, 0x00475660, 0x004756D0,
    ],
    [
        0x00477DD0, 0x004750B0, 0x004751E0, 0x00475410, 0x00475660, 0x004756D0,
    ],
    [
        0x00477420, 0x00474B20, 0x00474BE0, 0x00474DC0, 0x00474F70, 0x00474DA0,
    ],
    [
        0x00477420, 0x00474B20, 0x00474BE0, 0x00474DC0, 0x00474F70, 0x00474DA0,
    ],
    [
        0x00478040, 0x004750B0, 0x004751E0, 0x00475410, 0x00475660, 0x004756D0,
    ],
    [
        0x00478040, 0x004750B0, 0x004751E0, 0x00475410, 0x00475660, 0x004756D0,
    ],
];

/// The `.data` words after the reciprocal table's terminating zero: both
/// tables, indexed first, in memory order.
pub(crate) const SPAN_ROW_WORDS: [u32; 2 * ROW_COUNT * 6] = flatten_tables();

const fn flatten_tables() -> [u32; 2 * ROW_COUNT * 6] {
    let mut out = [0u32; 2 * ROW_COUNT * 6];
    let mut row = 0;
    while row < ROW_COUNT {
        let mut column = 0;
        while column < 6 {
            out[row * 6 + column] = INDEXED_ROWS[row][column];
            out[ROW_COUNT * 6 + row * 6 + column] = RAW_ROWS[row][column];
            column += 1;
        }
        row += 1;
    }
    out
}
