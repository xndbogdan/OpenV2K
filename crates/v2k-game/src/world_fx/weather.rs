//! The level weather's falling class-35 particles: the births
//! `FUN_0042DB70` makes around the camera, and their `FUN_00442E10` update.

use super::*;

/// Class 35, the snow or rain of `+0x6C` worlds (WindyIce, Reactor).
pub const WEATHER_PARTICLE_CLASS: u8 = 35;
const WEATHER_FALL_UPDATE_CALLBACK_VA: u32 = 0x0044_2E10;

pub(super) fn uses_weather_fall_update_callback(class: u8) -> bool {
    particle_descriptor(class)
        .is_some_and(|descriptor| descriptor.raw_u32(0x14) == WEATHER_FALL_UPDATE_CALLBACK_VA)
}

/// The process-global weather words `FUN_0042D9B0` and `FUN_0042DB70` keep:
/// the patterns' clock `0x004DAFC4` (milliseconds; no world load resets it),
/// the lightning cooldown `0x004DB24C`, and `0x004DE844`, the falling
/// particles that survived this frame's traversal.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct WeatherState {
    clock_ms: u32,
    lightning_cooldown: i32,
    falling_live: i32,
}

impl WorldFx {
    /// `FUN_0040F9F0`, which `FUN_0042D9B0` calls each frame of a world
    /// authoring either moving pattern.
    pub fn advance_weather_clock(&mut self, elapsed_micros: u32) {
        self.weather.clock_ms = self.weather.clock_ms.wrapping_add(elapsed_micros / 1000);
    }

    /// The patterns' clock, `0x004DAFC4`.
    pub fn weather_clock_ms(&self) -> u32 {
        self.weather.clock_ms
    }

    pub(super) fn reset_weather_falling_count(&mut self) {
        self.weather.falling_live = 0;
    }

    /// `FUN_00442E10`, after the traversal's ordinary integration. A fully
    /// submerged flake, one below the ground first sampled under it, or one
    /// the last draw did not put on screen is marked for removal (it still
    /// draws this frame); the rest fall by the frame's gravity word and count.
    pub(super) fn update_weather_fall(
        &mut self,
        particle: &mut WorldParticle,
        terrain: Option<&TerrainGrid>,
        elapsed_micros: u32,
    ) {
        if particle.water_state == 0 {
            particle.pending_destruction = true;
            return;
        }
        let position = world_position_to_raw(particle.position);
        // 43DB60 fills the +18 cache once (flag 0x20); the first sample comes
        // here, while the allocation's flag 0x10 still selects bilinear.
        let ground = *particle.ground_sample_raw.get_or_insert_with(|| {
            terrain.map_or(0, |terrain| {
                bilinear_terrain_height_raw(terrain, position[0], position[2])
            })
        });
        if ground > position[1] || !particle.on_screen {
            particle.pending_destruction = true;
            return;
        }
        let gravity = ((i64::from(elapsed_micros) * 0x30_0000) >> 31) as i16;
        let vertical =
            (world_velocity_component_to_raw(particle.velocity[1]) as i16).wrapping_sub(gravity);
        particle.velocity[1] = raw_velocity_to_world([0, i32::from(vertical), 0])[1];
        self.weather.falling_live += 1;
    }

    /// `FUN_0042DB70`'s `+0x6C` births: while fewer flakes than the frame-rate
    /// scale `>> 10` survive, a quarter of the shortfall is attempted around
    /// the camera's signed 8.8 position, six shared random words each, until
    /// one is refused.
    pub fn emit_weather_particles(&mut self, camera_raw: [i16; 3]) {
        let shortfall =
            (self.frame_pacing.count_scale_q16 >> 10) as i32 - self.weather.falling_live;
        if shortfall <= 4 {
            return;
        }
        for _ in 0..shortfall >> 2 {
            let mut random = || i32::from(self.next_shared_retail_random_u16());
            let position = [
                ((random() >> 3) & 0x1fff) - 0x1000 + i32::from(camera_raw[0]),
                ((random() >> 4) & 0xfff) - 1000 + i32::from(camera_raw[1]),
                ((random() >> 4) & 0xfff) + 2000 + i32::from(camera_raw[2]),
            ]
            .map(|word| word as i16);
            let velocity = [
                ((random() >> 9) & 0x7f) - 0x100,
                ((random() >> 10) & 0x3f) - 0x100,
                ((random() >> 9) & 0x7f) - 0x100,
            ];
            let burst =
                single_descriptor_particle_burst([0.0; 3], WEATHER_PARTICLE_CLASS, velocity, None);
            let spawn = burst.particles[0];
            if self
                .materialize_particle_spawn_with_birth_context(
                    raw_position_to_world(position),
                    spawn,
                    None,
                )
                .is_none()
            {
                return;
            }
        }
    }

    /// `FUN_0042DB70`'s `+0x74` lightning: one shared random word; a flash
    /// when its low word divides by `12 - cooldown`, which then restarts at
    /// 9, else the cooldown counts down to zero.
    pub fn roll_weather_lightning(&mut self) -> bool {
        let sample = i32::from(self.next_shared_retail_random_u16());
        if sample % (12 - self.weather.lightning_cooldown) == 0 {
            self.weather.lightning_cooldown = 9;
            return true;
        }
        self.weather.lightning_cooldown = (self.weather.lightning_cooldown - 1).max(0);
        false
    }

    /// The draw sweep's visibility flag: `FUN_0043DCD0` clears particle
    /// `+0x1D` bit `0x10` before each draw callback and `FUN_0043D410` sets
    /// it again for a sprite on screen.
    pub fn mark_particles_on_screen(&mut self, slots: &[usize]) {
        for &slot in slots {
            if let Some(particle) = self.particles.slots[slot].as_mut() {
                particle.on_screen = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use v2k_formats::terrain::{TerrainCell, GRID_SIZE};

    fn flat_terrain(height: i8) -> TerrainGrid {
        TerrainGrid {
            header: [i32::MIN, 0, 0, 0, 0],
            cells: vec![
                TerrainCell {
                    height: height as u8,
                    attribute: 0,
                    terrain_type: 0,
                };
                GRID_SIZE * GRID_SIZE
            ],
        }
    }

    fn flake(y_raw: i16, vertical_raw: i32) -> WorldParticle {
        let spawn = single_descriptor_particle_burst(
            [0.0; 3],
            WEATHER_PARTICLE_CLASS,
            [0, vertical_raw, 0],
            None,
        )
        .particles[0];
        let mut particle = world_particle(raw_position_to_world([0x1000, y_raw, 0x1000]), spawn);
        particle.water_state = 2;
        particle
    }

    fn seed_where(test: impl Fn(u16) -> bool) -> u32 {
        (0..u32::MAX)
            .find(|&seed| {
                let mut state = seed;
                test(retail_random_u16(&mut state))
            })
            .unwrap()
    }

    #[test]
    fn class_35_is_the_falling_weather_particle() {
        assert!(uses_weather_fall_update_callback(WEATHER_PARTICLE_CLASS));
        assert!(!uses_weather_fall_update_callback(34));
        let descriptor = particle_descriptor(WEATHER_PARTICLE_CLASS).unwrap();
        assert_eq!(descriptor.lifetime_ticks(), 0xAA);
        assert_eq!(
            descriptor.flags() & 0x02,
            0,
            "off-screen flakes are deleted"
        );
    }

    #[test]
    fn a_flake_on_screen_above_its_first_ground_falls_and_counts() {
        let mut fx = WorldFx::new();
        let mut particle = flake(0, -0x100);
        fx.update_weather_fall(&mut particle, Some(&flat_terrain(-10)), 20_000);
        assert!(!particle.pending_destruction);
        assert_eq!(particle.ground_sample_raw, Some(-10 * 0x20));
        // DAT_004DCEE0 at 20 ms: (20000 * 0x300000) >> 31 = 29.
        assert_eq!(
            world_velocity_component_to_raw(particle.velocity[1]),
            -0x100 - 29
        );
        assert_eq!(fx.weather.falling_live, 1);

        // The cached ground stays, even over higher terrain.
        fx.update_weather_fall(&mut particle, Some(&flat_terrain(100)), 20_000);
        assert!(!particle.pending_destruction);
        assert_eq!(fx.weather.falling_live, 2);
    }

    #[test]
    fn flakes_below_ground_off_screen_or_submerged_are_marked_dead() {
        let mut fx = WorldFx::new();
        let terrain = flat_terrain(10);
        let mut below = flake(0, -0x100);
        fx.update_weather_fall(&mut below, Some(&terrain), 20_000);
        assert!(below.pending_destruction);

        let mut hidden = flake(0x1000, -0x100);
        hidden.on_screen = false;
        fx.update_weather_fall(&mut hidden, Some(&terrain), 20_000);
        assert!(hidden.pending_destruction);

        let mut submerged = flake(0x1000, -0x100);
        submerged.water_state = 0;
        fx.update_weather_fall(&mut submerged, Some(&terrain), 20_000);
        assert!(submerged.pending_destruction);
        assert_eq!(
            submerged.ground_sample_raw, None,
            "no ground sample under water"
        );
        assert_eq!(fx.weather.falling_live, 0);
    }

    #[test]
    fn births_fill_a_quarter_of_the_shortfall_from_six_random_words_each() {
        let mut fx = WorldFx::new();
        fx.frame_pacing.count_scale_q16 = 0x1_0000;
        fx.weather.falling_live = 20;
        fx.rng_state = 0x1234_5678;
        let mut expected_state = fx.rng_state;
        let mut words: Vec<i32> = (0..66)
            .map(|_| i32::from(retail_random_u16(&mut expected_state)))
            .collect();
        words.truncate(6);
        let first = [
            ((words[0] >> 3) & 0x1fff) - 0x1000 + 0x2000,
            ((words[1] >> 4) & 0xfff) - 1000 - 0x100,
            ((words[2] >> 4) & 0xfff) + 2000 + 0x3000,
        ]
        .map(|word| word as i16);
        let first_velocity = [
            ((words[3] >> 9) & 0x7f) - 0x100,
            ((words[4] >> 10) & 0x3f) - 0x100,
            ((words[5] >> 9) & 0x7f) - 0x100,
        ];
        fx.emit_weather_particles([0x2000, -0x100, 0x3000]);
        // (64 - 20) >> 2 = 11 births of six words each.
        assert_eq!(fx.particle_count(), 11);
        assert_eq!(fx.rng_state, expected_state);
        let born = fx
            .particles
            .slots
            .iter()
            .flatten()
            .find(|particle| world_position_to_raw(particle.position) == first)
            .expect("the first birth at its random position");
        assert_eq!(born.source_class, WEATHER_PARTICLE_CLASS);
        assert!(born.on_screen);
        assert_eq!(
            born.velocity.map(world_velocity_component_to_raw),
            first_velocity
        );
    }

    #[test]
    fn no_births_until_the_shortfall_exceeds_four() {
        let mut fx = WorldFx::new();
        fx.frame_pacing.count_scale_q16 = 0x1_0000;
        fx.weather.falling_live = 60;
        let state = fx.rng_state;
        fx.emit_weather_particles([0; 3]);
        assert_eq!(fx.particle_count(), 0);
        assert_eq!(fx.rng_state, state);
    }

    #[test]
    fn lightning_odds_rise_as_the_cooldown_runs_out() {
        let mut fx = WorldFx::new();
        fx.rng_state = seed_where(|word| word % 12 != 0);
        assert!(!fx.roll_weather_lightning());
        assert_eq!(fx.weather.lightning_cooldown, 0);
        fx.rng_state = seed_where(|word| word % 12 == 0);
        assert!(fx.roll_weather_lightning());
        assert_eq!(fx.weather.lightning_cooldown, 9);
        // At cooldown 9 the divisor is 3.
        fx.rng_state = seed_where(|word| word % 3 != 0);
        assert!(!fx.roll_weather_lightning());
        assert_eq!(fx.weather.lightning_cooldown, 8);
    }

    #[test]
    fn the_weather_clock_counts_whole_milliseconds_across_world_loads() {
        let mut fx = WorldFx::new();
        fx.advance_weather_clock(20_000);
        fx.advance_weather_clock(1_999);
        assert_eq!(fx.weather_clock_ms(), 21);
        fx.clear();
        assert_eq!(fx.weather_clock_ms(), 21);
    }
}
