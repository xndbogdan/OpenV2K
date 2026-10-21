//! Type34 meteors born from a Type35 launcher's method-11 shot.
//!
//! 44E770's actor branch raises the record's Y by 300 and floors its vertical
//! velocity at 0x960 before 438080 builds the actor; the new actor's +60 is
//! the launcher. Class19's B910 then publishes Tertiary Trailing Fire
//! (`069E0`, particle class 0x28), clears Secondary and constructs the
//! 5000-ms Primary Boulder Rolling (`04580`). Style 4C7858's +00 is 40CEF0
//! but its +04 is null, so the stationary tag kills nothing: only the strict
//! lifetime or contact damage reaches 410C10, whose class1 alternate runs
//! BAC0's blast. Every other style hook is null.

use super::{grenade::RollingPrimaryProfile, EntityWeaponBlock};
use crate::entity_collision_state::EntityTypeRuntimeMetadata;

pub const METEOR_ENTITY_TYPE: u32 = crate::intro2_meteors::INTRO2_METEOR_TYPE;
/// Class19 Boulder Trailing Fire's only style.
pub const METEOR_STYLE_ADDRESS: u32 = 0x004c_7858;
/// Method 11's `DAT_004D02C0` speed.
pub const METEOR_LAUNCH_SPEED_RAW: i32 = 2400;
pub const METEOR_PRIMARY: RollingPrimaryProfile = RollingPrimaryProfile {
    lifetime_ms: crate::intro2_meteors::BOULDER_ROLLING_LIFETIME_MS,
    tag_transition: false,
};

/// 44E770's Type34 record adjustment: Y+300, then `vy` at least 0x960.
pub const fn meteor_launch_raw(
    position_raw: [i16; 3],
    velocity_raw: [i16; 3],
) -> ([i16; 3], [i16; 3]) {
    let [x, y, z] = position_raw;
    let [vx, vy, vz] = velocity_raw;
    (
        [x, y.wrapping_add(300), z],
        [vx, if vy < 0x960 { 0x960 } else { vy }, vz],
    )
}

pub(crate) fn authenticate_meteor_metadata(
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), EntityWeaponBlock> {
    crate::intro2_meteors::authenticate_metadata(metadata).map_err(|_| EntityWeaponBlock::Metadata)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_record_rises_300_and_climbs_at_least_0x960() {
        assert_eq!(
            meteor_launch_raw([1, 2112, 3], [-5, -900, 7]),
            ([1, 2412, 3], [-5, 0x960, 7])
        );
        assert_eq!(
            meteor_launch_raw([0, 32_700, 0], [0, 3000, 0]),
            ([0, -32_536, 0], [0, 3000, 0])
        );
    }
}
