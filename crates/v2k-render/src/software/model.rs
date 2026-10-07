//! Model face constructors: the face opcodes of the near (`0x004D3CE0`)
//! and fog (`0x004D40E0`) constructor tables.
//!
//! A face command names a material, a normal and three or four vertex
//! slots (Gouraud faces add one normal per corner). Its constructor skips
//! the face when the normal is culled, when the corners' combined outcode
//! cannot reach the screen, or (fog table) when every corner is fully
//! faded; otherwise it queues one fill-slot packet keyed by the first
//! corner's depth. Opcode bits select the packet:
//!
//! | bits | family | triangle / quad slot (near; fog adds 4) |
//! |---|---|---|
//! | — | flat colour | `+0x1038` / `+0x1080` |
//! | 0x40 | colour lit by the face normal | `+0x1040` / `+0x1088` |
//! | 0x20 | Gouraud colour | `+0x1048` / `+0x1090` |
//! | 0x80 | sprite | `+0x1050` / `+0x1098` |
//! | 0xC0 | sprite, one shade row | `+0x1058` / `+0x10A0` |
//! | 0xA0 | sprite, Gouraud shade | `+0x1060` / `+0x10A8` |
//!
//! The `x7`/`x8` opcodes repeat the face with every slot and normal index
//! XOR 1 (the mirrored half of a symmetric model); the copy is queued
//! through `FUN_0045B280`.

use super::material::MaterialId;
use super::queue::{PrimitiveQueue, QueueError};
use super::slots::FillSlot;
use super::terrain::OUTCODE_VISIBLE;

/// One warm vertex cache entry (0x14 bytes in retail).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ModelCorner {
    /// VIEW coordinates; the depth (`view[2]`) keys sorted packets.
    pub view: [i32; 3],
    pub screen: [i16; 2],
    /// Outcode, with bit 0x80 once the projector has run.
    pub clip: u8,
    /// Fade byte written by the fog projector.
    pub fade: u8,
}

impl ModelCorner {
    fn screen_dword(self) -> u32 {
        u32::from(self.screen[0] as u16) | (u32::from(self.screen[1] as u16) << 16)
    }
}

/// One evaluated normal cache entry (8 bytes in retail).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ModelNormal {
    /// The Section-6 shade dword lighting selected for this normal.
    pub shade: u32,
    /// Facing away from the camera.
    pub culled: bool,
}

/// What a constructor reads besides the command words.
pub struct FaceContext<'a> {
    pub corners: &'a [ModelCorner],
    pub normals: &'a [ModelNormal],
    /// Context `+0x6C`: a Section-7 palette entry's colour dword.
    pub palette: &'a dyn Fn(i16) -> u32,
    /// Context `+0x68`: a global sprite record.
    pub sprite: &'a dyn Fn(i16) -> MaterialId,
    /// Context `+0x74`.
    pub fog_colour: u32,
}

/// Which constructor table the node selected (`FUN_00464E60`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FacePass {
    Near,
    Fog,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Family {
    Flat,
    Lit,
    Gouraud,
    Sprite,
    SpriteLit,
    SpriteGouraud,
}

/// A decoded face opcode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FaceOp {
    family: Family,
    corners: usize,
    mirror: bool,
}

impl FaceOp {
    fn decode(opcode: u8) -> Option<Self> {
        let (corners, mirror) = match opcode & 0x1F {
            0x03 => (3, false),
            0x04 => (4, false),
            0x07 => (3, true),
            0x08 => (4, true),
            _ => return None,
        };
        let family = match opcode & 0xE0 {
            0x00 => Family::Flat,
            0x40 => Family::Lit,
            0x20 => Family::Gouraud,
            0x80 => Family::Sprite,
            0xC0 => Family::SpriteLit,
            0xA0 => Family::SpriteGouraud,
            _ => return None,
        };
        Some(Self {
            family,
            corners,
            mirror,
        })
    }

    /// Command words after the opcode.
    fn words(self) -> usize {
        let shades = match self.family {
            Family::Gouraud | Family::SpriteGouraud => self.corners,
            _ => 0,
        };
        2 + self.corners + shades
    }

    fn slot(self, pass: FacePass) -> FillSlot {
        use FillSlot::*;
        let triangle = self.corners == 3;
        let fog = pass == FacePass::Fog;
        match (self.family, triangle, fog) {
            (Family::Flat, true, false) => FlatTriangle,
            (Family::Flat, true, true) => FogTriangle,
            (Family::Flat, false, false) => FlatQuad,
            (Family::Flat, false, true) => FogQuad,
            (Family::Lit, true, false) => TintTriangle,
            (Family::Lit, true, true) => LitFogTriangle,
            (Family::Lit, false, false) => TintQuad,
            (Family::Lit, false, true) => LitFogQuad,
            (Family::Gouraud, true, false) => GouraudTriangle,
            (Family::Gouraud, true, true) => VertexLitFogTriangle,
            (Family::Gouraud, false, false) => GouraudQuad,
            (Family::Gouraud, false, true) => VertexLitFogQuad,
            (Family::Sprite, true, false) => TexturedTriangle,
            (Family::Sprite, true, true) => TexturedFogTriangle,
            (Family::Sprite, false, false) => TexturedQuad,
            (Family::Sprite, false, true) => TexturedFogQuad,
            (Family::SpriteLit, true, false) => TexturedRowTriangle,
            (Family::SpriteLit, true, true) => TexturedShadeFogTriangle,
            (Family::SpriteLit, false, false) => TexturedRowQuad,
            (Family::SpriteLit, false, true) => TexturedShadeFogQuad,
            (Family::SpriteGouraud, true, false) => ShadedTriangle,
            (Family::SpriteGouraud, true, true) => ShadedFogTriangle,
            (Family::SpriteGouraud, false, false) => ShadedQuad,
            (Family::SpriteGouraud, false, true) => ShadedFogQuad,
        }
    }
}

/// `FUN_0046DC00`, the type-1 vertex projector: average the two projected
/// sources' screen points (truncating), keep the nearer source's depth and
/// fade and take a fresh outcode from the averaged point. Either source's
/// near flag rejects the vertex instead: its outcode becomes 0x40 and its
/// depth 0. Fields retail does not write are kept from `current`.
pub fn screen_midpoint(
    a: ModelCorner,
    b: ModelCorner,
    current: ModelCorner,
    bounds: [u32; 2],
) -> ModelCorner {
    if (a.clip | b.clip) & 0x40 != 0 {
        return ModelCorner {
            view: [current.view[0], current.view[1], 0],
            clip: 0x40,
            ..current
        };
    }
    let x = ((i32::from(a.screen[0]) + i32::from(b.screen[0])) / 2) as i16;
    let y = ((i32::from(a.screen[1]) + i32::from(b.screen[1])) / 2) as i16;
    let nearer = if a.view[2] < b.view[2] { a } else { b };
    ModelCorner {
        view: [current.view[0], current.view[1], nearer.view[2]],
        screen: [x, y],
        clip: super::terrain::outcode_of(i32::from(x), i32::from(y), bounds),
        fade: nearer.fade,
    }
}

/// Run the constructor for `opcode` on the command `words` following it.
/// Returns the words consumed, or `None` when `opcode` is not a face.
pub fn construct_face(
    queue: &mut PrimitiveQueue,
    context: &FaceContext<'_>,
    pass: FacePass,
    opcode: u8,
    words: &[i16],
) -> Result<Option<usize>, QueueError> {
    let Some(op) = FaceOp::decode(opcode) else {
        return Ok(None);
    };
    let words = &words[..op.words()];
    emit(queue, context, pass, op, words, 0, false)?;
    if op.mirror {
        emit(queue, context, pass, op, words, 1, true)?;
    }
    Ok(Some(op.words()))
}

/// One face; `flip` XORs every slot and normal index (the mirrored copy).
fn emit(
    queue: &mut PrimitiveQueue,
    context: &FaceContext<'_>,
    pass: FacePass,
    op: FaceOp,
    words: &[i16],
    flip: i32,
    mode_checked: bool,
) -> Result<(), QueueError> {
    let index = |word: i16| (i32::from(word) ^ flip) as usize;
    let normal = context.normals[index(words[1])];
    if normal.culled {
        return Ok(());
    }
    let corners: Vec<ModelCorner> = words[2..2 + op.corners]
        .iter()
        .map(|&word| context.corners[index(word)])
        .collect();
    let outcode = corners.iter().fold(0u8, |code, corner| code | corner.clip);
    if OUTCODE_VISIBLE[usize::from(outcode)] == 0 {
        return Ok(());
    }
    let fog = pass == FacePass::Fog;
    if fog && corners.iter().all(|corner| corner.fade == 0xFF) {
        return Ok(());
    }
    let n = op.corners;
    let gouraud = matches!(op.family, Family::Gouraud | Family::SpriteGouraud);
    let near_bytes = match op.family {
        Family::Flat | Family::Sprite => 4 * n + 8,
        Family::Lit | Family::SpriteLit => 4 * n + 12,
        Family::Gouraud | Family::SpriteGouraud => 8 * n + 8,
    };
    let bytes = if fog { near_bytes + 8 } else { near_bytes };
    let slot = op.slot(pass);
    let key = corners[0].view[2];
    let payload = if mode_checked || queue.sorted_scope() {
        queue.push(key, slot, bytes)?
    } else {
        queue.push_fifo(slot, bytes)?
    };
    for (at, corner) in corners.iter().enumerate() {
        payload[4 * at..4 * at + 4].copy_from_slice(&corner.screen_dword().to_le_bytes());
    }
    let first = match op.family {
        Family::Flat | Family::Lit | Family::Gouraud => (context.palette)(words[0]),
        Family::Sprite | Family::SpriteLit | Family::SpriteGouraud => (context.sprite)(words[0]),
    };
    payload[4 * n..4 * n + 4].copy_from_slice(&first.to_le_bytes());
    let mut at = 4 * n + 4;
    match op.family {
        Family::Lit | Family::SpriteLit => {
            payload[at..at + 4].copy_from_slice(&normal.shade.to_le_bytes());
            at += 4;
        }
        _ if gouraud => {
            for &reference in &words[2 + n..2 + 2 * n] {
                let shade = context.normals[index(reference)].shade;
                payload[at..at + 4].copy_from_slice(&shade.to_le_bytes());
                at += 4;
            }
        }
        _ => {}
    }
    payload[at..at + 4].copy_from_slice(&0u32.to_le_bytes());
    at += 4;
    if fog {
        payload[at..at + 4].copy_from_slice(&context.fog_colour.to_le_bytes());
        at += 4;
        for (offset, corner) in corners.iter().enumerate() {
            payload[at + offset] = corner.fade;
        }
    }
    Ok(())
}

/// A billboard command (`0x68` flat colour, `0x78` sprite): its anchor
/// vertex, colour or sprite operand, and the size and angle operands as
/// `FUN_00470840` decodes them. `size` is in VIEW units; retail's model
/// units are VIEW units, and it doubles the operand's low word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BillboardCommand {
    pub vertex: usize,
    pub id: i16,
    pub size: i32,
    pub angle: u16,
    pub textured: bool,
}

/// The duplicated quarter-sine word as a Q31 value (`FUN_00467840`).
fn sine_q31(angle: u32) -> i32 {
    v2k_formats::fixed_math::retail_sine_q15(angle & 0xFFFF).wrapping_mul(0x1_0001)
}

/// One Q31 product's low word.
fn product_word(a: i32, b: i32) -> i16 {
    super::fixed::mul_q31(a, b) as i16
}

/// The billboard constructors (`FUN_00467840`/`00467C60` near,
/// `004680F0`/`00468520` fog): a quad rotated by the angle about the
/// anchor's screen point, half the projected size on each side. Sprite
/// billboards scale the rotation by the sprite's aspect (`1 / max(w, h)`
/// times `1/sqrt(2)`). Anchors behind the near plane, and in the fog table
/// fully faded anchors, are skipped; the quad is keyed by the anchor's
/// depth and, in the fog table, every corner takes the anchor's fade byte.
pub fn construct_billboard(
    queue: &mut PrimitiveQueue,
    context: &FaceContext<'_>,
    lens: &super::terrain::GroundProjection,
    sprite_size: &dyn Fn(i16) -> (u16, u16),
    pass: FacePass,
    command: BillboardCommand,
) -> Result<(), QueueError> {
    let anchor = context.corners[command.vertex];
    if anchor.clip & 0x40 != 0 || (pass == FacePass::Fog && anchor.fade == 0xFF) {
        return Ok(());
    }
    let depth = anchor.view[2];
    let (hx, hy) = if depth > 0x3F {
        lens.half_width(command.size.wrapping_mul(2), depth)
    } else {
        (0, 0)
    };
    let (hx, hy) = (i32::from(hx), i32::from(hy));
    let [cx, cy] = anchor.screen;
    let sine = sine_q31(u32::from(command.angle));
    let cosine = sine_q31(u32::from(command.angle) + 0x4000);
    let corners: [(i16, i16); 4];
    let first;
    if command.textured {
        let (width, height) = sprite_size(command.id);
        let (width, height) = (i32::from(width), i32::from(height));
        let larger = width.max(height);
        let sine = super::fixed::mul_q31(sine / larger, 0x5A82_7999);
        let cosine = super::fixed::mul_q31(cosine / larger, 0x5A82_7999);
        let u8_ = product_word(sine, hy.wrapping_mul(width));
        let u7 = product_word(cosine, hx.wrapping_mul(width));
        let u6 = product_word(sine, hx.wrapping_mul(height));
        let u5 = product_word(cosine, hy.wrapping_mul(height));
        corners = [
            (
                cx.wrapping_sub(u7).wrapping_add(u6),
                cy.wrapping_sub(u8_).wrapping_sub(u5),
            ),
            (
                cx.wrapping_add(u6).wrapping_add(u7),
                cy.wrapping_add(u8_).wrapping_sub(u5),
            ),
            (
                cx.wrapping_add(u7).wrapping_sub(u6),
                cy.wrapping_add(u5).wrapping_add(u8_),
            ),
            (
                cx.wrapping_sub(u7).wrapping_sub(u6),
                cy.wrapping_add(u5).wrapping_sub(u8_),
            ),
        ];
        first = (context.sprite)(command.id);
    } else {
        let u7 = product_word(sine, hy);
        let u6 = product_word(cosine, hx);
        let u9 = product_word(sine, hx);
        let u8_ = product_word(cosine, hy);
        corners = [
            (
                cx.wrapping_add(u9).wrapping_sub(u6),
                cy.wrapping_sub(u7).wrapping_sub(u8_),
            ),
            (
                cx.wrapping_add(u9).wrapping_add(u6),
                cy.wrapping_add(u7).wrapping_sub(u8_),
            ),
            (
                cx.wrapping_add(u6).wrapping_sub(u9),
                cy.wrapping_add(u7).wrapping_add(u8_),
            ),
            (
                cx.wrapping_sub(u6).wrapping_sub(u9),
                cy.wrapping_add(u8_).wrapping_sub(u7),
            ),
        ];
        first = (context.palette)(command.id);
    }
    let outcode = corners.iter().fold(0u8, |code, &(x, y)| {
        code | super::terrain::outcode_of(i32::from(x), i32::from(y), lens.bounds)
    });
    if OUTCODE_VISIBLE[usize::from(outcode)] == 0 {
        return Ok(());
    }
    let fog = pass == FacePass::Fog;
    let slot = match (command.textured, fog) {
        (false, false) => FillSlot::FlatQuad,
        (false, true) => FillSlot::FogQuad,
        (true, false) => FillSlot::TexturedQuad,
        (true, true) => FillSlot::TexturedFogQuad,
    };
    let bytes = if fog { 0x20 } else { 0x18 };
    let payload = queue.push(depth, slot, bytes)?;
    for (index, (x, y)) in corners.into_iter().enumerate() {
        payload[4 * index..4 * index + 2].copy_from_slice(&x.to_le_bytes());
        payload[4 * index + 2..4 * index + 4].copy_from_slice(&y.to_le_bytes());
    }
    payload[0x10..0x14].copy_from_slice(&first.to_le_bytes());
    payload[0x14..0x18].copy_from_slice(&0u32.to_le_bytes());
    if fog {
        payload[0x18..0x1C].copy_from_slice(&context.fog_colour.to_le_bytes());
        payload[0x1C..0x20].fill(anchor.fade);
    }
    Ok(())
}
