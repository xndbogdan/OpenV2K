//! `FUN_00433720`'s infection write and its listener list `004DC5F8`.
//!
//! Every infection or cure goes through [`write_infection`] at the moment of
//! the write, so the radar's shared word and the static object's carriers
//! fall between the writer's own draws and births exactly where retail's do.

use crate::{
    infection_evolution::InfectionCellWrite,
    resource_cache::ResourceCache,
    static_kind_catalog::static_kind_descriptor,
    world_fx::{ParticleTerrainMutation, ParticleTerrainPublication, WorldFx},
};

/// `FUN_00433720`: set or clear the infection bit of the write's cell. A
/// change runs the list's listeners in retail's order: the radar's
/// `FUN_0044A850`, the static object's `FUN_00427C30`, then the network's
/// `FUN_00438F60`, which the cache journals. Returns whether the bit changed;
/// an error names the listener that could not run, after the write.
pub fn write_infection(
    resources: &mut ResourceCache,
    world_fx: &mut WorldFx,
    write: InfectionCellWrite,
) -> Result<bool, &'static str> {
    let changed = resources
        .apply_level_infection_writes(&[write])
        .ok_or("infection terrain")?
        != 0;
    if changed {
        radar_redraw(resources, world_fx, write)?;
        static_object_effect(resources, world_fx, write)?;
    }
    Ok(changed)
}

/// A committing particle host's writes for one slot, in order, while the
/// carrier still holds its address: a class-5/6 contact's infection with its
/// listeners, F800's burn bit as before.
pub fn publish_particle_terrain_writes(
    resources: &mut ResourceCache,
    world_fx: &mut WorldFx,
    mutations: &[ParticleTerrainMutation],
) -> ParticleTerrainPublication {
    if resources.level_terrain().is_none() {
        return ParticleTerrainPublication::Deferred;
    }
    for &mutation in mutations {
        let ParticleTerrainMutation::Infection {
            infected,
            position_raw,
            ..
        } = mutation
        else {
            resources.apply_particle_terrain_mutations(&[mutation]);
            continue;
        };
        match write_infection(
            resources,
            world_fx,
            InfectionCellWrite::at(position_raw, infected),
        ) {
            Ok(true) => resources.mark_level_terrain_presentation_dirty(),
            Ok(false) => {}
            Err(listener) => eprintln!("Particle infection write blocked: {listener}"),
        }
    }
    ParticleTerrainPublication::Committed
}

/// `FUN_0044A850`: `FUN_0044A5C0` redraws the cell at the write's raw words,
/// one shared word when the cell has coverage. A world without a resident
/// radar has no coverage (as [`crate::static_terrain_burn::radar_cell_listener`]).
fn radar_redraw(
    resources: &mut ResourceCache,
    world_fx: &mut WorldFx,
    write: InfectionCellWrite,
) -> Result<(), &'static str> {
    if resources.level_terrain_radar().is_none() {
        return Ok(());
    }
    resources
        .refresh_level_terrain_radar(&[write.position_raw], &mut || {
            world_fx.next_shared_retail_random_u16()
        })
        .map(|_| ())
        .map_err(|_| "infection radar redraw")
}

/// `FUN_00427C30`: a cell holding a static object whose kind has an entry in
/// `004C9E48` emits it through `FUN_00440DC0` with the null owner: a cure the
/// `+0C` class at the cell centre on the ground, an infection the `+08` class
/// at the cell's X centre, 0x80 short of its low Z edge, raised by half the
/// now-selected model's extent. Scale is `+00`; a network client's carriers
/// are harmless.
fn static_object_effect(
    resources: &ResourceCache,
    world_fx: &mut WorldFx,
    write: InfectionCellWrite,
) -> Result<(), &'static str> {
    let Some(request) = static_object_effect_request(resources, write)? else {
        return Ok(());
    };
    world_fx.emit_static_object_infection_effect_raw(
        request.position_raw,
        request.particle_class,
        request.scale_raw,
    );
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StaticObjectEffect {
    position_raw: [i16; 3],
    particle_class: u8,
    scale_raw: u32,
}

fn static_object_effect_request(
    resources: &ResourceCache,
    write: InfectionCellWrite,
) -> Result<Option<StaticObjectEffect>, &'static str> {
    let cell = *resources
        .level_terrain()
        .and_then(|terrain| terrain.cell(usize::from(write.cell[0]), usize::from(write.cell[1])))
        .ok_or("infection effect terrain")?;
    // A zero attribute or no Section-9 table selects no record.
    let Some(objects) = resources.terrain_objects() else {
        return Ok(None);
    };
    if cell.attribute == 0 {
        return Ok(None);
    }
    let object = objects
        .records
        .get(usize::from(cell.attribute))
        .ok_or("infection effect static descriptor")?;
    let words = static_kind_descriptor(object.kind_index)
        .ok_or("infection effect static kind")?
        .raw_words();
    let particle_class = if write.infected { words[6] } else { words[7] };
    if particle_class == 0 {
        return Ok(None);
    }
    let height_raw = i16::from(cell.height as i8) * 0x20;
    let x_raw = ((write.position_raw[0] as u16 & 0xFF00) + 0x80) as i16;
    let z_edge_raw = (write.position_raw[1] as u16 & 0xFF00) as i16;
    let position_raw = if write.infected {
        let extent_raw = resources
            .global_model(usize::from(object.model_id_for(cell.terrain_type)))
            .ok_or("infection effect model")?
            .radius;
        [
            x_raw,
            height_raw.wrapping_add((extent_raw >> 1) as i16),
            z_edge_raw.wrapping_sub(0x80),
        ]
    } else {
        [x_raw, height_raw, z_edge_raw.wrapping_add(0x80)]
    };
    Ok(Some(StaticObjectEffect {
        position_raw,
        particle_class: particle_class as u8,
        scale_raw: words[4],
    }))
}

#[cfg(test)]
#[path = "infection_listeners_tests.rs"]
mod tests;
