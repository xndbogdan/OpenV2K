//! Common 44EC60 wind/drag, with explicit 44EB40 level initialization.
//!
//! Mode1 retains the authored vector. Mode2 receives the shared 44EBD0
//! vector, advanced once per world frame by the process-global phase owner.
//! The sea-side gate uses Section13+80 (42ED00), not water visibility.

use std::num::NonZeroU16;

use super::type9_attitude::Type9BodyBasis;
use crate::entity::CommonEnvironmentPhysics;
use crate::hover::q31_mul;
use v2k_formats::{levels::LevelDescriptor, terrain::TerrainGrid};

/// Retail .data at VA4D04C8 (file offsetCECC8), not a zero-initialized BSS
/// word. Level initialization44EB40 deliberately leaves this phase alone.
pub(crate) const INITIAL_WIND_PHASE_RAW: u32 = 0x1234_5678;

#[derive(Debug, Clone, Copy)]
pub(crate) struct GustPhase(u32);

impl Default for GustPhase {
    fn default() -> Self {
        Self(INITIAL_WIND_PHASE_RAW)
    }
}

impl GustPhase {
    pub(crate) fn advance(&mut self, elapsed_micros: u32) -> u32 {
        self.0 = self.0.wrapping_add(elapsed_micros);
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Wind {
    None,
    Current {
        vector_raw: [i16; 3],
        max_height_raw: i32,
        above_sea: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CommonWindDrag {
    wind: Wind,
    strength: u32,
}

impl CommonWindDrag {
    pub(crate) fn from_level(
        level: &LevelDescriptor,
        runtime: CommonEnvironmentPhysics,
    ) -> Result<Self, &'static str> {
        for offset in [0x80, 0x84, 0x90, 0x94, 0x98, 0x9c, 0xa0, 0xa4] {
            level.raw_u32(offset).ok_or("world wind descriptor")?;
        }
        let mut authored = CommonEnvironmentPhysics::from_level(level);
        // Only EBD0 mutates the current vector. Every descriptor-owned field
        // must still match the current manager's 44EB40 initialization.
        if runtime.runtime_wind_mode == 2 {
            authored.current_wind_raw = runtime.current_wind_raw;
        }
        if runtime != authored {
            return Err("world wind custody");
        }
        Ok(Self::from_environment(runtime))
    }

    pub(crate) fn from_environment(runtime: CommonEnvironmentPhysics) -> Self {
        let wind = if runtime.runtime_wind_mode == 0 {
            Wind::None
        } else {
            Wind::Current {
                vector_raw: runtime.current_wind_raw,
                max_height_raw: runtime.maximum_wind_height_raw,
                above_sea: runtime.wind_above_sea,
            }
        };
        Self {
            wind,
            strength: runtime.drag_strength,
        }
    }
}

/// 44EBD0 scales each signed authored word with the retail quarter-sine
/// lookup. The caller owns the one wrapping phase advance, before this read.
pub(crate) fn gust_vector_raw(configured: [i16; 3], phase: u32) -> [i16; 3] {
    let sine = v2k_formats::fixed_math::retail_sine_q15(phase >> 7);
    configured.map(|word| (sine.wrapping_mul(i32::from(word)) >> 15) as i16)
}

pub(crate) struct CommonWindDragFrame<'a> {
    pub terrain: &'a TerrainGrid,
    pub position_raw: [i16; 3],
    pub basis: Type9BodyBasis,
    pub callback_mass_raw: NonZeroU16,
    pub elapsed_micros: u32,
}

/// Apply after gravity and F70's body rebuild. EC60 changes angle words but
/// does not rebuild the basis again; later 18640 sees that earlier basis.
/// The enclosing E100 owner authenticates effective drag bit8 before entry.
pub(crate) fn apply_common_wind_drag_raw(
    velocity: &mut [i16; 3],
    angles: &mut [i16; 3],
    wind: CommonWindDrag,
    frame: CommonWindDragFrame<'_>,
) {
    let factor = frame.elapsed_micros.wrapping_mul(wind.strength)
        / (u32::from(frame.callback_mass_raw.get()) << 3);
    let mut delta = None;
    if let Wind::Current {
        vector_raw,
        max_height_raw,
        above_sea,
    } = wind.wind
    {
        if (frame.terrain.sea_level_raw() < frame.position_raw[1]) == above_sea {
            let [x, y, z] = frame.position_raw;
            let half = frame.terrain.bilinear_height_raw(
                x.wrapping_sub(vector_raw[0] >> 1),
                z.wrapping_sub(vector_raw[2] >> 1),
            );
            let full = frame
                .terrain
                .bilinear_height_raw(x.wrapping_sub(vector_raw[0]), z.wrapping_sub(vector_raw[2]));
            let height = (i32::from(y) - i32::from(half.max(full))).min(max_height_raw);
            // A sheltered actor returns before both angular and linear drag.
            if height < 0 {
                return;
            }
            let scale = height.wrapping_mul(32);
            let relative: [i16; 3] = std::array::from_fn(|axis| {
                ((i32::from(vector_raw[axis]).wrapping_mul(scale) >> 15) as i16)
                    .wrapping_sub(velocity[axis])
            });
            let dot = |basis: [i32; 3]| {
                basis
                    .into_iter()
                    .zip(relative)
                    .fold(0i32, |sum, (component, word)| {
                        sum.wrapping_add(q31_mul(component, i32::from(word)))
                    })
            };
            angles[2] = angles[2].wrapping_add(q31_mul(
                frame.elapsed_micros as i32,
                dot(frame.basis.lateral).wrapping_mul(-0x4000),
            ) as i16);
            angles[1] = angles[1].wrapping_add(q31_mul(
                frame.elapsed_micros as i32,
                dot(frame.basis.forward).wrapping_mul(0x1000),
            ) as i16);
            delta = Some(relative);
        }
    }
    for axis in 0..3 {
        // The fallback subtracts a signed shifted velocity product. Negating
        // velocity before multiplication would differ for -32768 and rounding.
        if let Some(delta) = delta {
            velocity[axis] = velocity[axis]
                .wrapping_add(((factor as i32).wrapping_mul(i32::from(delta[axis])) >> 15) as i16);
        } else {
            velocity[axis] = velocity[axis].wrapping_sub(
                ((factor as i32).wrapping_mul(i32::from(velocity[axis])) >> 15) as i16,
            );
        }
    }
}

#[cfg(test)]
mod tests;
