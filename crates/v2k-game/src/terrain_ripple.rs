//! Deferred terrain ripples: a `FUN_00437390` template with bit 0x01 queues a
//! record that `FUN_004374B0` advances every frame (`FUN_004371B0`), after
//! the static programs and the radar's sea redraw. A ring of raised ground
//! runs outward over the template's period; each frame writes only the
//! difference from the last, so the ground returns to its old heights, and a
//! cell whose height fell takes `FUN_00427B20`'s static hit. When the period
//! ends the record runs its flash (bit 0x02), radial (0x04) and crater (0x08)
//! and is freed. The queue is dropped with the world (`FUN_00437470`).

use crate::{
    entity::EntityManager,
    full_frame_sprite_sequence::FullFrameSpriteSequence,
    resource_cache::ResourceCache,
    static_damage::{StaticCraterOutcome, StaticDamageScheduler},
    terrain_crater::{
        apply_world_terrain_crater, crater_material_for_world_style, TerrainCraterRequest,
    },
    world_fx::WorldFx,
};
use v2k_formats::terrain::{TerrainGrid, GRID_SIZE};

/// The 14 template words `FUN_00437390` copies into a record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerrainRippleTemplate {
    /// 0x01 deferred, 0x02 flash, 0x04 radial, 0x08 crater.
    pub flags: u32,
    pub radius_raw: i32,
    /// Height-byte units at the ring's peak.
    pub height: i32,
    pub period_us: i32,
    /// Words 4..11: `FUN_004566E0`'s radial template.
    pub radial_words: [u32; 8],
    pub crater_radius_raw: i32,
    /// Height-byte units.
    pub crater_depth: i32,
}

/// `004C89E8`, world 43's Type85 death: a ring 10 cells across, 10 height
/// units high, over one second, then a flash and a 0x1900 crater 10 deep.
pub const TYPE85_DEATH_RIPPLE: TerrainRippleTemplate = TerrainRippleTemplate {
    flags: 0x0B,
    radius_raw: 0x0A00,
    height: 10,
    period_us: 1_000_000,
    radial_words: [0; 8],
    crater_radius_raw: 0x1900,
    crater_depth: 10,
};

/// One queued record: the template, its origin and `+48/+4C/+50`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerrainRipple {
    pub position_raw: [i16; 3],
    pub template: TerrainRippleTemplate,
    elapsed_us: i32,
    amplitude: i32,
    ring_squared: i32,
}

impl TerrainRipple {
    pub fn new(position_raw: [i16; 3], template: TerrainRippleTemplate) -> Self {
        Self {
            position_raw,
            template,
            elapsed_us: 0,
            amplitude: 0,
            ring_squared: 0,
        }
    }

    /// `FUN_004371B0`: advance by `elapsed_us` and write this frame's height
    /// change. Returns the cells whose height fell, in visit order (X outer,
    /// Z inner); the caller gives each `FUN_00427B20`'s hit. The period's
    /// end wraps the clock to zero, which [`Self::finished`] reports.
    pub fn advance(&mut self, terrain: &mut TerrainGrid, elapsed_us: i32) -> Vec<[u8; 2]> {
        let t = &self.template;
        let mut now = elapsed_us.wrapping_add(self.elapsed_us);
        if t.period_us <= now {
            now = 0;
        }
        let radius = t.radius_raw;
        let radius_squared = radius.wrapping_mul(radius);
        let ring = ((now >> 10).wrapping_mul(radius)) / (t.period_us >> 10);
        let ring_squared = ring.wrapping_mul(ring);
        let quarter = t.period_us >> 2;
        let amplitude = if now <= quarter {
            t.height.wrapping_mul(now) / quarter
        } else if now <= quarter * 3 {
            t.height
        } else {
            (now - quarter * 3).wrapping_mul(t.height) / quarter
        };
        let lift = |distance: i32, ring_squared: i32, amplitude: i32| {
            let falloff = (distance.wrapping_sub(ring_squared).wrapping_abs() - 0x40000).max(0);
            (amplitude - (falloff >> 16)).max(0)
        };
        let [x, _, z] = self.position_raw;
        let mut fallen = Vec::new();
        let mut dx = -i32::from(radius as i16);
        while dx <= radius {
            let cell_x = (x.wrapping_add(dx as i16) as u16 >> 8) as usize;
            let mut dz = -i32::from(radius as i16);
            while dz <= radius {
                let distance = dz * dz + dx * dx;
                if distance < radius_squared {
                    let cell_z = (z.wrapping_add(dz as i16) as u16 >> 8) as usize;
                    let delta = lift(distance, ring_squared, amplitude)
                        - lift(distance, self.ring_squared, self.amplitude);
                    let cell = &mut terrain.cells[cell_x * GRID_SIZE + cell_z];
                    cell.height = cell.height.wrapping_add(delta as u8);
                    if delta < 0 {
                        fallen.push([cell_x as u8, cell_z as u8]);
                    }
                }
                dz += 0x100;
            }
            dx += 0x100;
        }
        self.elapsed_us = now;
        self.amplitude = amplitude;
        self.ring_squared = ring_squared;
        fallen
    }

    /// `FUN_004374B0` frees a record whose clock wrapped to zero.
    pub fn finished(&self) -> bool {
        self.elapsed_us == 0
    }
}

/// A ripple that stopped: the phase and what was written before it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerrainRippleStop {
    pub phase: &'static str,
    pub terrain_changed: bool,
}

/// `FUN_004566E0` at a finished record (bit 0x04): the static cells, then
/// the live list, in the world the caller owns.
pub type TerrainRippleRadial<'a> = dyn FnMut(
        &mut ResourceCache,
        &mut EntityManager,
        &mut WorldFx,
        &mut StaticDamageScheduler,
        [i16; 3],
        crate::radial_damage::RadialDamageTemplate,
    ) -> Result<(), &'static str>
    + 'a;

/// `FUN_004374B0`'s ripple pass over the queue held in `world_fx`, in queue
/// order. A record that stops is dropped with its logged phase; the rest
/// still run.
pub fn advance_terrain_ripples(
    elapsed_us: u32,
    resources: &mut ResourceCache,
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    static_damage: &mut StaticDamageScheduler,
    flash: &mut FullFrameSpriteSequence,
    radial: &mut TerrainRippleRadial<'_>,
) -> Vec<TerrainRippleStop> {
    let mut stops = Vec::new();
    for mut ripple in world_fx.take_terrain_ripples() {
        match advance_one(
            &mut ripple,
            elapsed_us as i32,
            resources,
            manager,
            world_fx,
            static_damage,
            flash,
            radial,
        ) {
            Ok(true) => {}
            Ok(false) => world_fx.queue_terrain_ripple(ripple),
            Err(stop) => stops.push(stop),
        }
    }
    stops
}

/// One record's frame; `Ok(true)` once it has finished and run its effects.
fn advance_one(
    ripple: &mut TerrainRipple,
    elapsed_us: i32,
    resources: &mut ResourceCache,
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    static_damage: &mut StaticDamageScheduler,
    flash: &mut FullFrameSpriteSequence,
    radial: &mut TerrainRippleRadial<'_>,
) -> Result<bool, TerrainRippleStop> {
    let stop = |phase, terrain_changed| TerrainRippleStop {
        phase,
        terrain_changed,
    };
    let terrain = resources
        .level_terrain_mut()
        .ok_or_else(|| stop("ripple terrain", false))?;
    let fallen = ripple.advance(terrain, elapsed_us);
    resources.mark_level_terrain_presentation_dirty();
    for cell in fallen {
        crater_hit(cell, resources, manager, world_fx, static_damage)
            .map_err(|phase| stop(phase, true))?;
    }
    if !ripple.finished() {
        return Ok(false);
    }
    if ripple.template.flags & 0x02 != 0 {
        flash.request();
    }
    if ripple.template.flags & 0x04 != 0 {
        // Record +0x20, words 4..11: 4566E0's template, whose trailing
        // source type/handle 19750 wrote before 37390 copied it.
        let w = ripple.template.radial_words;
        let template = crate::radial_damage::RadialDamageTemplate {
            inner_radius_raw: w[0] as u16 as i16,
            outer_radius_raw: (w[0] >> 16) as u16 as i16,
            impulse_raw: w[1] as i32,
            packet: crate::damage::DamagePacket {
                channels: [w[2] as i32, w[3] as i32],
                amounts_raw: [w[4] as i32, w[5] as i32],
            },
            trailing_raw: [w[6] as i32, w[7] as i32],
        };
        radial(
            resources,
            manager,
            world_fx,
            static_damage,
            ripple.position_raw,
            template,
        )
        .map_err(|phase| stop(phase, true))?;
    }
    if ripple.template.flags & 0x08 != 0 {
        let world_style = resources
            .level_desc()
            .ok_or_else(|| stop("ripple crater world", true))?
            .world_style;
        apply_world_terrain_crater(
            manager,
            resources,
            world_fx,
            static_damage,
            TerrainCraterRequest {
                position_raw: ripple.position_raw,
                radius_raw: ripple.template.crater_radius_raw,
                depth_height_units: ripple.template.crater_depth,
                replacement_material: crater_material_for_world_style(world_style),
            },
        )
        .map_err(|failure| stop(failure.phase, true))?;
    }
    Ok(true)
}

/// `FUN_00427B20` at a cell: a static object with no timed program burns
/// (`FUN_004337E0`), any other starts its program once.
fn crater_hit(
    cell: [u8; 2],
    resources: &mut ResourceCache,
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    static_damage: &mut StaticDamageScheduler,
) -> Result<(), &'static str> {
    let attribute = resources
        .level_terrain()
        .and_then(|terrain| terrain.cell(usize::from(cell[0]), usize::from(cell[1])))
        .ok_or("ripple hit terrain")?
        .attribute;
    if attribute == 0 {
        return Ok(());
    }
    let kind = resources
        .terrain_objects()
        .and_then(|objects| objects.records.get(usize::from(attribute)))
        .ok_or("ripple hit static descriptor")?
        .kind_index;
    match static_damage.submit_crater_destruction(cell, kind) {
        StaticCraterOutcome::BurnCell => {
            crate::static_terrain_burn::apply_static_terrain_burn(
                cell,
                resources,
                manager,
                world_fx,
                static_damage,
            )
            .map_err(|failure| failure.phase)?;
        }
        StaticCraterOutcome::Started | StaticCraterOutcome::Duplicate => {}
        StaticCraterOutcome::UnsupportedKind { .. } => return Err("ripple hit static kind"),
    }
    Ok(())
}

#[cfg(test)]
#[path = "terrain_ripple_tests.rs"]
mod tests;
