//! A world's weather (`FUN_0042DB70`), from its Section-13 record.
//!
//! `+0x70` darkens the terrain light window with a moving shadow pattern,
//! `+0x78` adds a moving light pattern, `+0x6C` keeps falling class-35
//! particles around the camera and repeats the shadow pattern, and `+0x74`
//! flashes lightning. The light writers are `v2k_render::TerrainLightWindow`'s,
//! the particles and the process-global words `WorldFx`'s.

use v2k_formats::levels::LevelDescriptor;
use v2k_render::TerrainLightWindow;

use crate::world_fx::WorldFx;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LevelWeather {
    pub falling: bool,
    pub shadow: bool,
    pub lightning: bool,
    pub light: bool,
}

impl LevelWeather {
    pub fn from_level(level: &LevelDescriptor) -> Self {
        let set = |offset| level.raw_u32(offset).is_some_and(|word| word != 0);
        Self {
            falling: set(0x6C),
            shadow: set(0x70),
            lightning: set(0x74),
            light: set(0x78),
        }
    }

    /// `FUN_0042D9B0`'s tail: only a world with a moving pattern advances the
    /// weather clock (`FUN_0040F9F0`).
    pub fn advance_clock(self, world_fx: &mut WorldFx, elapsed_micros: u32) {
        if self.shadow || self.light {
            world_fx.advance_weather_clock(elapsed_micros);
        }
    }

    /// One presented frame, after the particles' light and before the
    /// terrain draw (`FUN_004536A0`). `live` is false for a frozen redraw,
    /// which repeats the patterns at the unchanged clock but neither births
    /// particles nor rolls lightning.
    pub fn present(
        self,
        lights: &mut TerrainLightWindow,
        world_fx: &mut WorldFx,
        live: bool,
        camera_raw: [i16; 3],
    ) {
        let clock = world_fx.weather_clock_ms();
        if self.shadow {
            lights.add_weather_shadow_pattern(clock);
        }
        if self.light {
            lights.add_weather_light_pattern(clock);
        }
        if self.falling {
            if live {
                world_fx.emit_weather_particles(camera_raw);
            }
            lights.add_weather_shadow_pattern(clock);
        }
        if self.lightning && live && world_fx.roll_weather_lightning() {
            lights.add_flash();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use v2k_formats::{ovl::OvlFile, sections};

    fn weather(level: u32) -> LevelWeather {
        let path = v2k_test_support::retail_dir()
            .join("Overlay")
            .join(format!("1X{level}XX.OVL"));
        let ovl = OvlFile::parse(&std::fs::read(path).unwrap()).unwrap();
        LevelWeather::from_level(&sections::parse_level(&ovl).unwrap())
    }

    #[v2k_test_support::retail_test]
    fn retail_worlds_author_their_weather() {
        let only = |pick: fn(&mut LevelWeather)| {
            let mut weather = LevelWeather::default();
            pick(&mut weather);
            weather
        };
        assert_eq!(weather(32), only(|w| w.falling = true), "WindyIce");
        assert_eq!(weather(38), only(|w| w.falling = true), "Reactor");
        assert_eq!(weather(43), only(|w| w.lightning = true), "Volcano");
        for level in [46, 47] {
            let alien = only(|w| {
                w.shadow = true;
                w.lightning = true;
            });
            assert_eq!(weather(level), alien, "Alien2/3");
        }
        let arena6 = only(|w| {
            w.shadow = true;
            w.light = true;
        });
        assert_eq!(weather(44), arena6, "Arena6");
        for level in [22, 23, 30, 33, 34] {
            assert_eq!(weather(level), only(|w| w.light = true), "world {level}");
        }
        assert_eq!(weather(13), LevelWeather::default());
    }

    #[test]
    fn only_moving_patterns_advance_the_clock() {
        let mut fx = WorldFx::new();
        let still = LevelWeather {
            falling: true,
            lightning: true,
            ..LevelWeather::default()
        };
        still.advance_clock(&mut fx, 20_000);
        assert_eq!(fx.weather_clock_ms(), 0);
        let moving = LevelWeather {
            light: true,
            ..LevelWeather::default()
        };
        moving.advance_clock(&mut fx, 20_000);
        assert_eq!(fx.weather_clock_ms(), 20);
    }

    #[test]
    fn falling_weather_repeats_the_shadow_pattern_and_a_frozen_frame_births_nothing() {
        let mut fx = WorldFx::new();
        fx.advance_weather_clock(1_234_000);
        let mut lights = TerrainLightWindow::default();
        lights.scroll_to_center(80, 60);
        let mut expected = lights.clone();
        expected.add_weather_shadow_pattern(1234);
        expected.add_weather_light_pattern(1234);
        expected.add_weather_shadow_pattern(1234);
        let every = LevelWeather {
            falling: true,
            shadow: true,
            light: true,
            lightning: true,
        };
        every.present(&mut lights, &mut fx, false, [0; 3]);
        assert_eq!(lights.window(), expected.window());
        assert_eq!(fx.particle_count(), 0);
    }
}
