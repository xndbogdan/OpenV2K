//! Portable Radar's `405610` callback and `4055C0` component destructor.
//!
//! Sub-K variable 1 belongs to the entity, not the task wrapper. These detached
//! programs borrow that exact word; they retain neither an activation boolean
//! nor a previous footprint center. Native construction, scheduler custody,
//! task installation, cargo and death/teardown dispatch remain separate owners.

use crate::{
    gameplay_radar::{RadarCoverageChange, TerrainRadarError},
    resource_cache::ResourceCache,
};

/// Inputs at the actual component callback boundary. The outer scheduler owns
/// detailed/coarse cadence and supplies its carry-adjusted delta here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortableRadarFrame {
    pub state_flags_raw: u32,
    pub elapsed_micros: u32,
    pub position_raw: [i16; 3],
}

/// Reports the callback's effects; every successful retail branch returns zero
/// and leaves the current behavior installed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortableRadarTickOutcome {
    Carried,
    Deploying,
    Deployed { random_draws: usize },
    AlreadyDeployed,
}

/// `405610`: attachment resets K1; otherwise advance once per callback with
/// no fractional carry. The first saturation writes FFFF before adding coverage.
///
/// A coverage error therefore retains that committed K1 write. A native caller
/// must park the failed callback prefix, rather than retrying it as a fresh
/// visit or interpreting the word alone as proof of completed deployment.
pub fn tick_portable_radar(
    sub_k1: &mut u16,
    frame: PortableRadarFrame,
    resources: &mut ResourceCache,
    next_random: &mut impl FnMut() -> u16,
) -> Result<PortableRadarTickOutcome, TerrainRadarError> {
    if frame.state_flags_raw & 0x1000 != 0 {
        // Successful attach normally already retired the previous Primary.
        // This callback itself never removes coverage.
        *sub_k1 = 0;
        return Ok(PortableRadarTickOutcome::Carried);
    }
    if *sub_k1 == u16::MAX {
        return Ok(PortableRadarTickOutcome::AlreadyDeployed);
    }
    *sub_k1 = (u32::from(*sub_k1) + (frame.elapsed_micros >> 4)).min(u32::from(u16::MAX)) as u16;
    if *sub_k1 != u16::MAX {
        return Ok(PortableRadarTickOutcome::Deploying);
    }
    let [x, _, z] = frame.position_raw;
    let random_draws = resources.mutate_level_terrain_radar_coverage(
        [x, z],
        RadarCoverageChange::Add,
        next_random,
    )?;
    Ok(PortableRadarTickOutcome::Deployed { random_draws })
}

/// `4055C0`: remove a saturated footprint at the entity's *current* position,
/// then reset the shared K1 word. Partial deployment needs no radar resources.
///
/// The native task owner must run this during inner-task retirement, before
/// freeing the task or publishing its replacement. A coverage failure leaves
/// K1 intact and must not be followed by slot disposal. Class49 calls it after
/// radial delivery and before the Type60 constructor; world teardown calls it
/// while the old world resources and process RNG are still available.
pub fn destroy_portable_radar(
    sub_k1: &mut u16,
    position_raw: [i16; 3],
    resources: &mut ResourceCache,
    next_random: &mut impl FnMut() -> u16,
) -> Result<usize, TerrainRadarError> {
    let draws = if *sub_k1 == u16::MAX {
        let [x, _, z] = position_raw;
        resources.mutate_level_terrain_radar_coverage(
            [x, z],
            RadarCoverageChange::Remove,
            next_random,
        )?
    } else {
        0
    };
    *sub_k1 = 0;
    Ok(draws)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(elapsed_micros: u32) -> PortableRadarFrame {
        PortableRadarFrame {
            state_flags_raw: 0x8000,
            elapsed_micros,
            position_raw: [0; 3],
        }
    }

    #[test]
    fn portable_radar_deploys_on_the_53rd_twenty_millisecond_callback() {
        let mut resources = ResourceCache::new(Vec::new());
        let mut count = 0;
        for visit in 1..=52 {
            assert_eq!(
                tick_portable_radar(&mut count, frame(20_000), &mut resources, &mut || {
                    panic!("partial deployment drew RNG")
                }),
                Ok(PortableRadarTickOutcome::Deploying)
            );
            assert_eq!(count, visit * 1250);
        }
        assert_eq!(
            tick_portable_radar(&mut count, frame(20_000), &mut resources, &mut || {
                panic!("missing raster drew RNG")
            }),
            Err(TerrainRadarError::Uninitialized)
        );
        assert_eq!(count, u16::MAX, "405610 commits K1 before coverage");
    }

    #[test]
    fn portable_radar_uses_each_callback_delta_without_fractional_carry() {
        let mut resources = ResourceCache::new(Vec::new());
        let mut count = 0;
        for delta in [15, 15, 15, 15] {
            tick_portable_radar(&mut count, frame(delta), &mut resources, &mut || {
                panic!("partial deployment drew RNG")
            })
            .unwrap();
        }
        assert_eq!(count, 0);
        tick_portable_radar(&mut count, frame(31), &mut resources, &mut || 0).unwrap();
        assert_eq!(count, 1);
        assert_eq!(
            tick_portable_radar(&mut count, frame(u32::MAX), &mut resources, &mut || 0),
            Err(TerrainRadarError::Uninitialized)
        );
        assert_eq!(
            count,
            u16::MAX,
            "large delta saturates without u16 wrapping"
        );
    }

    #[test]
    fn portable_radar_attachment_precedes_deployed_and_partial_destruction_needs_no_map() {
        let mut resources = ResourceCache::new(Vec::new());
        let mut count = u16::MAX;
        let mut no_rng = || panic!("inactive portable radar drew RNG");
        assert_eq!(
            tick_portable_radar(&mut count, frame(u32::MAX), &mut resources, &mut no_rng),
            Ok(PortableRadarTickOutcome::AlreadyDeployed)
        );
        assert_eq!(count, u16::MAX);
        assert_eq!(
            tick_portable_radar(
                &mut count,
                PortableRadarFrame {
                    state_flags_raw: 0x1000,
                    ..frame(u32::MAX)
                },
                &mut resources,
                &mut no_rng,
            ),
            Ok(PortableRadarTickOutcome::Carried)
        );
        assert_eq!(count, 0);
        count = u16::MAX - 1;
        assert_eq!(
            destroy_portable_radar(&mut count, [0; 3], &mut resources, &mut no_rng),
            Ok(0)
        );
        assert_eq!(count, 0);
    }
}
