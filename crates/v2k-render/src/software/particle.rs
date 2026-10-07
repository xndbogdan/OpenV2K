//! Particle producer: `FUN_0043D410`.
//!
//! A particle queues one sprite quad around its projected centre, sized by
//! its sprite record, its draw scale and the depth: half width
//! `((w * scale) >> 8) * focal / (depth << 9)` (height likewise). Unless the
//! class centres it, the quad stands on the particle; a class may mirror
//! it. Particles off screen by more than the screen size, behind the near
//! plane or past the far plane draw nothing, as do quads that cannot reach
//! the screen. The quad is keyed by the depth plus the class's signed bias:
//! a near textured quad (`+0x1098`) before the fog plane, a fogged one
//! (`+0x109C`) with the centre's fade byte past it. A class with a shadow
//! size also queues a one-pixel flat quad on the ground under the particle,
//! between two points projected by `FUN_0046D010`, keyed one unit deeper.
//!
//! The particle's frame selection, size jitter and ground height
//! (`FUN_0043DB60`) are the caller's; the terrain light a frame can carry
//! (`FUN_004385E0`) is not a draw.

use super::queue::{PrimitiveQueue, QueueError};
use super::slots::FillSlot;
use super::terrain::{GroundMaterial, GroundPoint, GroundProjection};

/// What `FUN_0043D410` reads from its render context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParticleScene {
    /// The world projector (`+0xB4`): the scene's dry or wet one.
    pub projection: GroundProjection,
    /// Eye words (`+0x04/+0x08/+0x0C`).
    pub eye: [i16; 3],
    /// `+0x74`: depths from here take the fogged quad.
    pub fog_near: i32,
    /// `+0x78`: depths from here draw nothing.
    pub far: i32,
    /// `+0x7C`.
    pub fog_colour: u32,
}

/// One particle's draw inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Particle {
    /// World words (particle `+0x08/+0x0A/+0x0C`).
    pub position: [i16; 3],
    /// The draw scale after its size jitter times the frame's size word.
    pub scale: i32,
    /// The frame's sprite record.
    pub sprite: GroundMaterial,
    /// Descriptor `+0x07`: 1 mirrors, 4 centres (else the quad stands on the
    /// point), 8 keeps at least one pixel.
    pub flags: u8,
    /// Descriptor `+0x12`.
    pub sort_bias: i16,
    /// The frame's size word, which alone scales the shadow.
    pub frame_size: u16,
    pub shadow: Option<ParticleShadow>,
}

/// Descriptor `+0x09` and what the shadow quad needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParticleShadow {
    /// Descriptor `+0x09`, non-zero.
    pub size: u8,
    /// Ground height under the particle (particle `+0x18`).
    pub ground: i16,
    /// System-2 palette entry 32's colour dword.
    pub colour: u32,
}

fn word_pair(payload: &mut [u8], at: usize, x: i16, y: i16) {
    payload[at..at + 2].copy_from_slice(&x.to_le_bytes());
    payload[at + 2..at + 4].copy_from_slice(&y.to_le_bytes());
}

fn word(payload: &[u8], at: usize) -> i16 {
    i16::from_le_bytes([payload[at], payload[at + 1]])
}

fn set_word(payload: &mut [u8], at: usize, value: i16) {
    payload[at..at + 2].copy_from_slice(&value.to_le_bytes());
}

/// Queue one particle; returns whether its quad was queued.
pub fn queue_particle(
    queue: &mut PrimitiveQueue,
    scene: &ParticleScene,
    particle: &Particle,
) -> Result<bool, QueueError> {
    let projection = &scene.projection;
    let dx = particle.position[0].wrapping_sub(scene.eye[0]);
    let dy = particle.position[1].wrapping_sub(scene.eye[1]);
    let dz = particle.position[2].wrapping_sub(scene.eye[2]);
    let mut centre = GroundPoint::default();
    let depth = projection.project(&mut centre, [i32::from(dx), i32::from(dy), i32::from(dz)]);
    let [sx, sy] = centre.screen.map(i32::from);
    let [width, height] = projection.bounds.map(|value| value as i32);
    if centre.clip & 0x6D != 0
        && (centre.clip & 0x40 != 0
            || scene.far <= depth
            || width.wrapping_mul(2) < ((width >> 1).wrapping_add(sx)).wrapping_abs()
            || height.wrapping_mul(2) < ((height >> 1).wrapping_add(sy)).wrapping_abs())
    {
        return Ok(false);
    }
    if depth >= scene.far {
        return Ok(false);
    }
    let size = |dimension: u16, focal: i32| -> i32 {
        ((u32::from(dimension).wrapping_mul(particle.scale as u32) as i32) >> 8)
            .wrapping_mul(focal)
            .checked_div(depth.wrapping_shl(9))
            .unwrap_or(0)
    };
    let mut half_width = size(particle.sprite.width, projection.focal[0]);
    let mut half_height = size(particle.sprite.height, projection.focal[1]);
    if particle.flags & 8 != 0 {
        half_width = half_width.max(1);
        half_height = half_height.max(1);
    }
    let reaches = |half: i32, at: i32, extent: i32| {
        (half.wrapping_add(at) as u32) < (extent.wrapping_add(half.wrapping_mul(2)) as u32)
    };
    if centre.clip & 0x6D != 0
        && !(reaches(half_width, sx, width) && reaches(half_height, sy, height))
    {
        return Ok(false);
    }
    let near = depth < scene.fog_near;
    let key = depth.wrapping_add(i32::from(particle.sort_bias));
    let (slot, bytes) = if near {
        (FillSlot::TexturedQuad, 0x18)
    } else {
        (FillSlot::TexturedFogQuad, 0x20)
    };
    let payload = queue.push(key, slot, bytes)?;
    let (hx, hy) = (half_width as i16, half_height as i16);
    let x = centre.screen[0];
    let mut y = centre.screen[1];
    if particle.flags & 4 == 0 {
        y = y.wrapping_sub(hy);
    }
    let (left, right) = (x.wrapping_sub(hx), x.wrapping_add(hx));
    let (top, bottom) = (y.wrapping_sub(hy), y.wrapping_add(hy));
    let (first, second) = if particle.flags & 1 == 0 {
        (left, right)
    } else {
        (right, left)
    };
    word_pair(payload, 0, first, top);
    word_pair(payload, 4, second, top);
    word_pair(payload, 8, second, bottom);
    word_pair(payload, 12, first, bottom);
    payload[0x10..0x14].copy_from_slice(&particle.sprite.id.to_le_bytes());
    payload[0x14..0x18].copy_from_slice(&0u32.to_le_bytes());
    if !near {
        payload[0x18..0x1C].copy_from_slice(&scene.fog_colour.to_le_bytes());
        payload[0x1C..0x20].fill(centre.fade);
    }
    if let Some(shadow) = particle.shadow {
        queue_shadow(queue, scene, particle, shadow, depth, dx, dz, centre.fade)?;
    }
    Ok(true)
}

#[allow(clippy::too_many_arguments)]
fn queue_shadow(
    queue: &mut PrimitiveQueue,
    scene: &ParticleScene,
    particle: &Particle,
    shadow: ParticleShadow,
    depth: i32,
    dx: i16,
    dz: i16,
    fade: u8,
) -> Result<(), QueueError> {
    let product = (scene.projection.focal[0] as u32)
        .wrapping_mul(u32::from(shadow.size))
        .wrapping_mul(i32::from(particle.frame_size as i16) as u32) as i32;
    let mut half = product.checked_div(depth).unwrap_or(0) >> 9;
    if half == 0 {
        half = 1;
    }
    let near = depth < scene.fog_near;
    let (slot, bytes) = if near {
        (FillSlot::FlatQuad, 0x18)
    } else {
        (FillSlot::FogQuad, 0x20)
    };
    let payload = queue.push(depth.wrapping_add(1), slot, bytes)?;
    payload[0x10..0x14].copy_from_slice(&shadow.colour.to_le_bytes());
    payload[0x14..0x18].copy_from_slice(&8u32.to_le_bytes());
    if !near {
        payload[0x18..0x1C].copy_from_slice(&scene.fog_colour.to_le_bytes());
        payload[0x1C..0x20].fill(fade);
    }
    // Both ends on the ground; a point behind the near plane keeps the
    // words the arena held.
    let x0 = dx.wrapping_sub(half as i16);
    let y = i32::from(shadow.ground.wrapping_sub(scene.eye[1]));
    let z = i32::from(dz);
    for (end, x) in [x0, x0.wrapping_add((half as i16).wrapping_mul(2))]
        .into_iter()
        .enumerate()
    {
        let at = 4 * end;
        let mut screen = [word(payload, at), word(payload, at + 2)];
        scene
            .projection
            .project_plain(&mut screen, [i32::from(x), y, z]);
        word_pair(payload, at, screen[0], screen[1]);
    }
    let [ax, ay, bx, by] = [
        word(payload, 0),
        word(payload, 2),
        word(payload, 4),
        word(payload, 6),
    ];
    let step_x: i16 = if ay < by { 1 } else { -1 };
    set_word(payload, 8, bx.wrapping_add(step_x));
    set_word(payload, 12, ax.wrapping_add(step_x));
    let step_y: i16 = if ax < bx { 1 } else { -1 };
    set_word(payload, 10, by.wrapping_add(step_y));
    set_word(payload, 14, ay.wrapping_add(step_y));
    Ok(())
}
