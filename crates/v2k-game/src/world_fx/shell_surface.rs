//! Classes 4, 54, 77 and 78 (the shells and the Virus/Antidote Bombs): mode 3
//! with `FUN_0043F260`'s gravity and `FUN_0043E300`'s surface response.

use super::virus_projectile::mode_three_touches_ground;
use super::*;

const SHELL_SURFACE_CALLBACK_VA: u32 = 0x0043_E300;
/// `DAT_004CD788`: the class `FUN_0043E300` bursts, by surface selector.
const SHELL_SURFACE_RESPONSE_CLASS_BY_SELECTOR: [u8; 8] = [7, 8, 9, 10, 7, 11, 13, 12];
/// `FUN_0043E300`'s `FUN_00440DC0` scale.
const SHELL_SURFACE_RESPONSE_SCALE: u32 = 10_000;
/// E300 returns one (the record is freed) while both horizontal velocity
/// words are below this.
const SHELL_SURFACE_REST_SPEED_RAW: i32 = 0x4B1;

pub(super) fn uses_shell_surface_callback(class: u8) -> bool {
    particle_descriptor(class).is_some_and(|descriptor| {
        descriptor.raw_byte(0x0e) == 3
            && descriptor.raw_u32(0x14) == GRAVITY_UPDATE_CALLBACK_VA
            && descriptor.raw_u32(0x18) == SHELL_SURFACE_CALLBACK_VA
    })
}

impl WorldFx {
    /// 40120's mode-3 surface stage for a shell: the water entry, then
    /// (unless E300 returned one) the terrain test, each through E300 with
    /// the material's selector and the surface word. True when E300 returned
    /// one; 40120 then frees the record at that address.
    pub(super) fn dispatch_shell_surface(
        &mut self,
        slot: usize,
        context: TerrainCollisionContext<'_>,
        retail_tick: u32,
    ) -> bool {
        let Some(particle) = self.particles.slots[slot] else {
            return false;
        };
        if particle.step_start_water_state == 2 && particle.water_state < 2 {
            if let Some(surface_y) =
                displaced_water_surface(context, particle.position, retail_tick)
            {
                let material = nearest_material_code(context.terrain, particle.position);
                if self.apply_shell_surface_response(
                    slot,
                    context.water_response_selectors[usize::from(material)],
                    world_position_to_raw([0.0, surface_y, 0.0])[1],
                ) {
                    return true;
                }
            }
        }
        let Some(particle) = self.particles.slots[slot] else {
            return false;
        };
        if !mode_three_touches_ground(context.terrain, &particle) {
            return false;
        }
        let material = nearest_material_code(context.terrain, particle.position);
        // FUN_0043DB60: the bilinear height while +1D bit 0x10 (on screen) is
        // set, else the cell's own.
        let position = world_position_to_raw(particle.position);
        let ground = if particle.on_screen {
            bilinear_terrain_height_raw(context.terrain, position[0], position[2])
        } else {
            coarse_terrain_height_raw(context.terrain, position[0], position[2])
        };
        self.apply_shell_surface_response(
            slot,
            context.ground_response_selectors[usize::from(material)],
            ground,
        )
    }

    /// `FUN_0043E300`: the `FUN_00440DC0` burst (the selector's class, scale
    /// 10000, the null owner), sound 90, then either the rest (returns one)
    /// or the bounce: Y on the surface and the vertical word halved and
    /// reversed.
    fn apply_shell_surface_response(
        &mut self,
        slot: usize,
        selector: u8,
        surface_raw: i16,
    ) -> bool {
        let Some(particle) = self.particles.slots[slot] else {
            return true;
        };
        let class = SHELL_SURFACE_RESPONSE_CLASS_BY_SELECTOR
            .get(usize::from(selector))
            .copied()
            .expect("authored surface selector has an E300 response class");
        self.emit_fun_00440dc0_raw(
            world_position_to_raw(particle.position),
            class,
            SHELL_SURFACE_RESPONSE_SCALE,
            0,
            particle.suppresses_impact_damage,
        );
        self.emit_fun_0044f450_hit_sound(90, particle.position);
        // The burst may have recycled this address: E300 writes whatever
        // record now holds it.
        let Some(live) = self.particles.slots[slot].as_mut() else {
            return true;
        };
        let velocity = live
            .velocity
            .map(|v| world_velocity_component_to_raw(v) as i16);
        if i32::from(velocity[0]).abs() < SHELL_SURFACE_REST_SPEED_RAW
            && i32::from(velocity[2]).abs() < SHELL_SURFACE_REST_SPEED_RAW
        {
            return true;
        }
        live.position[1] = f32::from(surface_raw) / 256.0;
        live.velocity[1] = raw_velocity_to_world([0, -(i32::from(velocity[1]) / 2), 0])[1];
        false
    }
}
