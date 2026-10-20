//! The game channel's records: the table at `0x4CAE08`, one 16-byte entry per
//! kind (apply, pack, kind, size word), and the 12-byte stamp that heads
//! each game message ([MULTIPLAYER.md](../../../../docs/re/MULTIPLAYER.md),
//! "Messages"). Each record keeps retail's byte layout.

use super::framing::Record;

/// `DAT_004CAE00`: the session tick's period for its periodic sends.
pub const PERIOD_US: u32 = 100_000;
/// Kinds the game channel's table holds (descriptor `0x4CAFA8` `+C`).
pub const KINDS: u8 = 26;

/// `FUN_00438BB0`/`FUN_00438BE0`: the sender's session id (its DirectPlay id
/// when its session started), a counter, its world, slot and, from the
/// host, its slot again.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stamp {
    /// `004DC9D0`.
    pub session: u32,
    /// `004DC9D4`, advanced for every message.
    pub counter: u32,
    /// `004DC9D8`: the world the sender plays.
    pub world: u16,
    /// `004DC9DA`.
    pub slot: u8,
    /// `004DC9DB`: the host's slot, 0 from a client.
    pub host_slot: u8,
}

impl Stamp {
    pub fn pack(&self) -> [u8; 12] {
        let mut out = [0u8; 12];
        out[..4].copy_from_slice(&self.session.to_le_bytes());
        out[4..8].copy_from_slice(&self.counter.to_le_bytes());
        out[8..10].copy_from_slice(&self.world.to_le_bytes());
        out[10] = self.slot;
        out[11] = self.host_slot;
        out
    }

    pub fn parse(bytes: &[u8]) -> Option<Self> {
        let b = bytes.get(..12)?;
        Some(Self {
            session: u32::from_le_bytes([b[0], b[1], b[2], b[3]]),
            counter: u32::from_le_bytes([b[4], b[5], b[6], b[7]]),
            world: u16::from_le_bytes([b[8], b[9]]),
            slot: b[10],
            host_slot: b[11],
        })
    }
}

/// Kind 7, `FUN_00417240`/`FUN_00417680`: a body's sync record. `flags`
/// names the fields present; the low bits mirror the body's state word.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BodySync {
    pub handle: u32,
    /// `+7C`: how often the owner has sent this body.
    pub resend: u32,
    pub flags: u16,
    /// `+84`: which extra fields follow.
    pub extras: u16,
    /// `+96`, flag `0x20`.
    pub position: [i16; 3],
    /// Flag 1: the body's type; a player's craft (46) travels as 51.
    pub entity_type: u32,
    /// `+9C`, flag `0x10`.
    pub velocity: [i16; 3],
    /// `+A2`, flag 4.
    pub euler: [i16; 3],
    /// Flag 8: the forward (`+24`) and lateral (`+0C`) rows, each Q31 word
    /// shifted right 19.
    pub basis: [i16; 6],
    /// `+64`, flag `0x100`.
    pub capability: u32,
    /// `+60`, extras bit 2.
    pub owner: u32,
    /// `+A8..+AE`, extras bit 1.
    pub words: [u16; 4],
    /// The animation sequence word (`FUN_00418F30`), extras bit 4.
    pub sequence: u16,
    /// `+50`, flag `0x200`.
    pub shield: u32,
    /// `+88`, flag `0x400`.
    pub payload: u32,
    /// The Sub-K model variables, flag `0x80`.
    pub model_variables: Vec<u16>,
    /// `+80`, flag `0x1000`: the parent's handle.
    pub parent: u32,
}

/// One game-channel record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GameRecord {
    /// 1: a particle request (`FUN_00440A60`'s 20 bytes).
    Particle([u8; 20]),
    /// 2: a burst request (`FUN_00440950`'s first 12 bytes).
    Burst([u8; 12]),
    /// 3: the host's sea level and terrain runs, as packed.
    Terrain(Vec<u8>),
    /// 4: one cell from the host: x, z, flags byte, attribute byte.
    Cell([u8; 4]),
    /// 5: a client's cell request.
    CellRequest([u8; 4]),
    /// 6: a static object's destruction, its cell word.
    StaticDestroyed(u32),
    /// 7.
    Body(BodySync),
    /// 8: a body's removal.
    Removed(u32),
    /// 9: damage to a body the receiver owns: amount, and the damage kind
    /// plus 6 with the sender's slot above it.
    Damage {
        handle: u32,
        amount: u16,
        kind_slot: u16,
    },
    /// 10: velocity (`+9C`) and attitude (`+A2`) deltas.
    Impact { handle: u32, deltas: [i16; 6] },
    /// 11: the owner should send the body's whole record (state bit 1).
    Resend(u32),
    /// 12: the bodies the sender sees in detail.
    LiveList(Vec<u32>),
    /// 13: parent, child and flags of an attach.
    Attach([u32; 3]),
    /// 14: a child released from its parent.
    Release { child: u32, parent: u32 },
    /// 15: `FUN_004180F0`'s record.
    Dropped([u32; 3]),
    /// 16: a client's kill counter (low word) and last killer (high).
    ScoreWord(u32),
    /// 17: a hit or kill credit: the victim's slot, the attacker's.
    KillReport { victim: u8, attacker: u8 },
    /// 18: the frag limit (low word) and winner (high), then each slot's
    /// kills, deaths and score.
    Scores {
        limit_winner: u32,
        table: [[u16; 3]; 8],
    },
    /// 19: the slot whose name is wanted.
    NameRequest(u32),
    /// 20: the sender's initials and name.
    Name { initials: [u8; 6], name: [u8; 16] },
    /// 21: a message line.
    Line(Vec<u8>),
    /// 22..=25: a positional sound: the id and the position's high bytes,
    /// then the optional third (`rate`, bit 0) and fourth (`gain`, bit 1)
    /// arguments of `FUN_0044F450`.
    Sound {
        word: u32,
        rate: Option<u32>,
        gain: Option<u32>,
    },
}

fn u16_at(b: &[u8], at: usize) -> Option<u16> {
    b.get(at..at + 2).map(|w| u16::from_le_bytes([w[0], w[1]]))
}

fn i16_at(b: &[u8], at: usize) -> Option<i16> {
    u16_at(b, at).map(|w| w as i16)
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    b.get(at..at + 4)
        .map(|w| u32::from_le_bytes([w[0], w[1], w[2], w[3]]))
}

struct Out<'a> {
    room: &'a mut [u8],
    at: usize,
}

impl Out<'_> {
    fn bytes(&mut self, bytes: &[u8]) -> Option<()> {
        self.room
            .get_mut(self.at..self.at + bytes.len())?
            .copy_from_slice(bytes);
        self.at += bytes.len();
        Some(())
    }
    fn u16(&mut self, value: u16) -> Option<()> {
        self.bytes(&value.to_le_bytes())
    }
    fn u32(&mut self, value: u32) -> Option<()> {
        self.bytes(&value.to_le_bytes())
    }
    fn i16s(&mut self, values: &[i16]) -> Option<()> {
        values.iter().try_for_each(|v| self.u16(*v as u16))
    }
}

impl BodySync {
    fn pack(&self, out: &mut Out) -> Option<()> {
        out.u32(self.handle)?;
        out.u32(self.resend)?;
        out.u16(self.flags)?;
        let f = self.flags;
        if f & 2 != 0 {
            out.u16(self.extras)?;
        }
        if f & 0x20 != 0 {
            out.i16s(&self.position)?;
        }
        if f & 1 != 0 {
            out.u32(self.entity_type)?;
        }
        if f & 0x10 != 0 {
            out.i16s(&self.velocity)?;
        }
        if f & 4 != 0 {
            out.i16s(&self.euler)?;
        }
        if f & 8 != 0 {
            out.i16s(&self.basis)?;
        }
        if f & 0x100 != 0 {
            out.u32(self.capability)?;
        }
        let x = if f & 2 != 0 { self.extras } else { 0 };
        if x & 2 != 0 {
            out.u32(self.owner)?;
        }
        if x & 1 != 0 {
            self.words.iter().try_for_each(|w| out.u16(*w))?;
        }
        if x & 4 != 0 {
            out.u16(self.sequence)?;
        }
        if f & 0x200 != 0 {
            out.u32(self.shield)?;
        }
        if f & 0x400 != 0 {
            out.u32(self.payload)?;
        }
        if f & 0x80 != 0 {
            out.u16(self.model_variables.len() as u16)?;
            self.model_variables.iter().try_for_each(|v| out.u16(*v))?;
        }
        if f & 0x1000 != 0 {
            out.u32(self.parent)?;
        }
        Some(())
    }

    /// `FUN_00417680`'s reading order; returns the record and its size.
    pub fn parse(b: &[u8]) -> Option<(Self, usize)> {
        let mut sync = Self {
            handle: u32_at(b, 0)?,
            resend: u32_at(b, 4)?,
            flags: u16_at(b, 8)?,
            ..Self::default()
        };
        let f = sync.flags;
        let mut at = 10;
        let i16s = |at: &mut usize, count: usize| -> Option<Vec<i16>> {
            let values = (0..count)
                .map(|i| i16_at(b, *at + i * 2))
                .collect::<Option<Vec<_>>>()?;
            *at += count * 2;
            Some(values)
        };
        if f & 2 != 0 {
            sync.extras = u16_at(b, at)?;
            at += 2;
        }
        if f & 0x20 != 0 {
            sync.position.copy_from_slice(&i16s(&mut at, 3)?);
        }
        if f & 1 != 0 {
            sync.entity_type = u32_at(b, at)?;
            at += 4;
        }
        if f & 0x10 != 0 {
            sync.velocity.copy_from_slice(&i16s(&mut at, 3)?);
        }
        if f & 4 != 0 {
            sync.euler.copy_from_slice(&i16s(&mut at, 3)?);
        }
        if f & 8 != 0 {
            sync.basis.copy_from_slice(&i16s(&mut at, 6)?);
        }
        if f & 0x100 != 0 {
            sync.capability = u32_at(b, at)?;
            at += 4;
        }
        let x = sync.extras;
        if x & 2 != 0 {
            sync.owner = u32_at(b, at)?;
            at += 4;
        }
        if x & 1 != 0 {
            for word in &mut sync.words {
                *word = u16_at(b, at)?;
                at += 2;
            }
        }
        if x & 4 != 0 {
            sync.sequence = u16_at(b, at)?;
            at += 2;
        }
        if f & 0x200 != 0 {
            sync.shield = u32_at(b, at)?;
            at += 4;
        }
        if f & 0x400 != 0 {
            sync.payload = u32_at(b, at)?;
            at += 4;
        }
        if f & 0x80 != 0 {
            let count = usize::from(u16_at(b, at)?);
            at += 2;
            sync.model_variables = (0..count)
                .map(|i| u16_at(b, at + i * 2))
                .collect::<Option<Vec<_>>>()?;
            at += count * 2;
        }
        if f & 0x1000 != 0 {
            sync.parent = u32_at(b, at)?;
            at += 4;
        }
        Some((sync, at))
    }
}

impl GameRecord {
    /// The table's kind for this record.
    pub fn kind_of(&self) -> u8 {
        match self {
            Self::Particle(_) => 1,
            Self::Burst(_) => 2,
            Self::Terrain(_) => 3,
            Self::Cell(_) => 4,
            Self::CellRequest(_) => 5,
            Self::StaticDestroyed(_) => 6,
            Self::Body(_) => 7,
            Self::Removed(_) => 8,
            Self::Damage { .. } => 9,
            Self::Impact { .. } => 10,
            Self::Resend(_) => 11,
            Self::LiveList(_) => 12,
            Self::Attach(_) => 13,
            Self::Release { .. } => 14,
            Self::Dropped(_) => 15,
            Self::ScoreWord(_) => 16,
            Self::KillReport { .. } => 17,
            Self::Scores { .. } => 18,
            Self::NameRequest(_) => 19,
            Self::Name { .. } => 20,
            Self::Line(_) => 21,
            Self::Sound { rate, gain, .. } => {
                22 + u8::from(rate.is_some()) + 2 * u8::from(gain.is_some())
            }
        }
    }

    /// Reads the record of `kind` at the start of `b`, with the size its
    /// retail apply routine returns.
    pub fn parse(kind: u8, b: &[u8]) -> Option<(Self, usize)> {
        let array = |n: usize| b.get(..n);
        Some(match kind {
            1 => (Self::Particle(array(20)?.try_into().ok()?), 20),
            2 => (Self::Burst(array(12)?.try_into().ok()?), 12),
            3 => {
                let size = usize::from(u16_at(b, 0)?);
                (Self::Terrain(b.get(..size)?.to_vec()), size)
            }
            4 => (Self::Cell(array(4)?.try_into().ok()?), 4),
            5 => (Self::CellRequest(array(4)?.try_into().ok()?), 4),
            6 => (Self::StaticDestroyed(u32_at(b, 0)?), 4),
            7 => {
                let (sync, size) = BodySync::parse(b)?;
                (Self::Body(sync), size)
            }
            8 => (Self::Removed(u32_at(b, 0)?), 4),
            9 => (
                Self::Damage {
                    handle: u32_at(b, 0)?,
                    amount: u16_at(b, 4)?,
                    kind_slot: u16_at(b, 6)?,
                },
                8,
            ),
            10 => {
                let mut deltas = [0i16; 6];
                for (i, delta) in deltas.iter_mut().enumerate() {
                    *delta = i16_at(b, 4 + i * 2)?;
                }
                (
                    Self::Impact {
                        handle: u32_at(b, 0)?,
                        deltas,
                    },
                    16,
                )
            }
            11 => (Self::Resend(u32_at(b, 0)?), 4),
            12 => {
                let count = u32_at(b, 0)? as usize;
                let handles = (0..count)
                    .map(|i| u32_at(b, 4 + i * 4))
                    .collect::<Option<Vec<_>>>()?;
                (Self::LiveList(handles), count * 4 + 4)
            }
            13 => (
                Self::Attach([u32_at(b, 0)?, u32_at(b, 4)?, u32_at(b, 8)?]),
                12,
            ),
            14 => (
                Self::Release {
                    child: u32_at(b, 0)?,
                    parent: u32_at(b, 4)?,
                },
                8,
            ),
            15 => (
                Self::Dropped([u32_at(b, 0)?, u32_at(b, 4)?, u32_at(b, 8)?]),
                12,
            ),
            16 => (Self::ScoreWord(u32_at(b, 0)?), 4),
            17 => (
                Self::KillReport {
                    victim: *b.first()?,
                    attacker: *b.get(1)?,
                },
                2,
            ),
            18 => {
                let mut table = [[0u16; 3]; 8];
                for (slot, row) in table.iter_mut().enumerate() {
                    for (field, value) in row.iter_mut().enumerate() {
                        *value = u16_at(b, 4 + slot * 6 + field * 2)?;
                    }
                }
                (
                    Self::Scores {
                        limit_winner: u32_at(b, 0)?,
                        table,
                    },
                    0x34,
                )
            }
            19 => (Self::NameRequest(u32_at(b, 0)?), 4),
            20 => (
                Self::Name {
                    initials: array(6)?.try_into().ok()?,
                    name: b.get(6..22)?.try_into().ok()?,
                },
                0x20,
            ),
            21 => {
                let length = usize::from(*b.first()?);
                (Self::Line(b.get(1..1 + length)?.to_vec()), length + 1)
            }
            22..=25 => {
                let flags = kind - 22;
                let mut at = 4;
                let mut optional = |present: bool| -> Option<Option<u32>> {
                    if !present {
                        return Some(None);
                    }
                    let value = u32_at(b, at)?;
                    at += 4;
                    Some(Some(value))
                };
                let rate = optional(flags & 1 != 0)?;
                let gain = optional(flags & 2 != 0)?;
                (
                    Self::Sound {
                        word: u32_at(b, 0)?,
                        rate,
                        gain,
                    },
                    at,
                )
            }
            _ => return None,
        })
    }
}

impl Record for GameRecord {
    fn kind(&self) -> u8 {
        self.kind_of()
    }

    /// The table's size words.
    fn size_word(&self) -> usize {
        [
            0, 20, 12, 16, 4, 4, 4, 64, 4, 8, 16, 4, 64, 12, 8, 12, 4, 4, 40, 4, 32, 64, 4, 8, 12,
            16,
        ][usize::from(self.kind_of())]
    }

    fn pack(&self, room: &mut [u8]) -> Option<usize> {
        let mut out = Out { room, at: 0 };
        match self {
            Self::Particle(b) => out.bytes(b)?,
            Self::Burst(b) => out.bytes(b)?,
            Self::Terrain(b) => out.bytes(b)?,
            Self::Cell(b) | Self::CellRequest(b) => out.bytes(b)?,
            Self::StaticDestroyed(w)
            | Self::Removed(w)
            | Self::Resend(w)
            | Self::ScoreWord(w)
            | Self::NameRequest(w) => out.u32(*w)?,
            Self::Body(sync) => sync.pack(&mut out)?,
            Self::Damage {
                handle,
                amount,
                kind_slot,
            } => {
                out.u32(*handle)?;
                out.u16(*amount)?;
                out.u16(*kind_slot)?;
            }
            Self::Impact { handle, deltas } => {
                out.u32(*handle)?;
                out.i16s(deltas)?;
            }
            Self::LiveList(handles) => {
                out.u32(handles.len() as u32)?;
                handles.iter().try_for_each(|h| out.u32(*h))?;
            }
            Self::Attach(words) | Self::Dropped(words) => {
                words.iter().try_for_each(|w| out.u32(*w))?
            }
            Self::Release { child, parent } => {
                out.u32(*child)?;
                out.u32(*parent)?;
            }
            Self::KillReport { victim, attacker } => out.bytes(&[*victim, *attacker])?,
            Self::Scores {
                limit_winner,
                table,
            } => {
                out.u32(*limit_winner)?;
                for row in table {
                    row.iter().try_for_each(|v| out.u16(*v))?;
                }
            }
            Self::Name { initials, name } => {
                out.bytes(initials)?;
                out.bytes(name)?;
                out.bytes(&[0; 10])?;
            }
            Self::Line(text) => {
                // `FUN_00439770` sends at most 250 characters.
                let text = &text[..text.len().min(250)];
                out.bytes(&[text.len() as u8])?;
                out.bytes(text)?;
            }
            Self::Sound { word, rate, gain } => {
                out.u32(*word)?;
                if let Some(rate) = rate {
                    out.u32(*rate)?;
                }
                if let Some(gain) = gain {
                    out.u32(*gain)?;
                }
            }
        }
        Some(out.at)
    }
}

#[cfg(test)]
#[path = "protocol_tests.rs"]
mod tests;
