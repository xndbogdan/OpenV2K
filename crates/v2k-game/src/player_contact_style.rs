//! Player style contact callbacks preceding the common physical response.
//!
//! Dying frame 4CDAA8 uses 447AF0 at +10/+14 and 447C90 at +1C.
//! Player Control frames 0..4 share only the latter static hook. Geometry and
//! the common response remain owned by the terrain/water/static walkers.

use crate::entity::{PlayerDyingContactBurstRequest, PlayerDyingContactRuntime};
use crate::entity_behavior::{
    audited_behavior_program, ActiveBehaviorStyle, BehaviorContextRuntime,
    BehaviorDescriptorIdentity,
};
use crate::entity_collision_state::RetailRuntimeValue;
use crate::terrain_contact::PlayerTerrainUnsupportedHaptic;
use crate::world_fx::WorldFx;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerContactStyleRequest {
    pub behavior_context: RetailRuntimeValue<Option<BehaviorContextRuntime>>,
    pub entity_handle: u32,
    pub controlled_entity_handle: Option<u32>,
    pub haptic_scale_raw: i32,
}

pub struct PlayerDyingSurfaceContactFrame<'a> {
    pub request: PlayerContactStyleRequest,
    pub runtime: &'a mut Option<PlayerDyingContactRuntime>,
    pub world_fx: &'a mut WorldFx,
    /// `475F0` reads half the fresh active model header's unsigned +08 radius.
    pub source_extent_raw: u16,
    pub sea_level_raw: Option<i16>,
    pub retail_tick: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerContactStyleBlock {
    UnauthenticatedStyle,
    MissingDyingControllerRuntime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerDyingSurfaceContactOutcome {
    pub controller_matched: bool,
    pub normal_velocity_raw: i32,
    pub unsupported_haptic: Option<PlayerTerrainUnsupportedHaptic>,
    pub wreck_burst_emitted: bool,
    pub next_burst_retail_tick: Option<i32>,
}

pub fn is_player_style(context: BehaviorContextRuntime, class_id: u32) -> bool {
    audited_behavior_program(class_id).is_some_and(|program| {
        matches!(context.descriptor(), BehaviorDescriptorIdentity::Named(current) if current == program)
            && match (class_id,context.active_style()) {
                (24,ActiveBehaviorStyle::Audited(style))=>style.class_id==24
                    && (0x004c_d940..=0x004c_daf0).contains(&style.frame_address)
                    && (style.frame_address-0x004c_d940)%0x48==0,
                (25,style)=>style==ActiveBehaviorStyle::Audited(program.initial_style),
                _=>false,
            }
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerSurfaceStyle {
    PlayerControl,
    DyingBounce,
    Null,
}

/// Actual contact-slot matrix: frames0..4 use448280/NULL/447C90, frame5
/// uses447AF0/447AF0/447C90, and frame6 has all three NULL. Class25 owns
/// exactly frame5; an unaudited active record never inherits its class slots.
pub fn player_surface_style(context: BehaviorContextRuntime) -> Option<PlayerSurfaceStyle> {
    if is_player_style(context, 25) {
        return Some(PlayerSurfaceStyle::DyingBounce);
    }
    if !is_player_style(context, 24) {
        return None;
    }
    match context.active_style().style_address() {
        0x004c_daa8 => Some(PlayerSurfaceStyle::DyingBounce),
        0x004c_daf0 => Some(PlayerSurfaceStyle::Null),
        _ => Some(PlayerSurfaceStyle::PlayerControl),
    }
}

pub fn player_contact_haptic(
    normal_q12: [i16; 3],
    velocity_raw: [i16; 3],
    scale_raw: i32,
) -> (i32, Option<PlayerTerrainUnsupportedHaptic>) {
    let dot = i32::from(normal_q12[1])
        .wrapping_mul(i32::from(velocity_raw[1]))
        .wrapping_add(i32::from(normal_q12[2]).wrapping_mul(i32::from(velocity_raw[2])))
        .wrapping_add(i32::from(normal_q12[0]).wrapping_mul(i32::from(velocity_raw[0])))
        >> 12;
    let inward = dot.wrapping_neg();
    (
        dot,
        (inward > 512).then(|| PlayerTerrainUnsupportedHaptic {
            inward_speed_raw: inward,
            haptic_scale_raw: scale_raw,
            strength_raw: (scale_raw.wrapping_mul(inward.wrapping_sub(512)) / 15).min(255),
        }),
    )
}

/// Exact successful 447AF0 continuation: feedback, strict signed latch,
/// synchronous 475F0/reseed, then the live entity's VY=1000. A failed controller
/// lookup skips feedback/burst but still reaches that final velocity write.
pub fn apply_player_dying_surface_contact(
    frame: PlayerDyingSurfaceContactFrame<'_>,
    position_raw: [i16; 3],
    normal_q12: [i16; 3],
    velocity_raw: &mut [i16; 3],
) -> Result<PlayerDyingSurfaceContactOutcome, PlayerContactStyleBlock> {
    if !matches!(frame.request.behavior_context, RetailRuntimeValue::Known(Some(context)) if player_surface_style(context)==Some(PlayerSurfaceStyle::DyingBounce))
    {
        return Err(PlayerContactStyleBlock::UnauthenticatedStyle);
    }
    let matched = frame.request.controlled_entity_handle == Some(frame.request.entity_handle);
    if matched
        && !frame
            .runtime
            .as_ref()
            .is_some_and(|runtime| runtime.player_id() == frame.request.entity_handle)
    {
        return Err(PlayerContactStyleBlock::MissingDyingControllerRuntime);
    }
    let (normal_velocity_raw, unsupported_haptic) =
        player_contact_haptic(normal_q12, *velocity_raw, frame.request.haptic_scale_raw);
    let wreck_burst_emitted = matched
        && frame
            .runtime
            .as_mut()
            .expect("admitted runtime")
            .emit_if_due(
                frame.world_fx,
                PlayerDyingContactBurstRequest {
                    position_raw,
                    source_extent_raw: frame.source_extent_raw,
                    sea_level_raw: frame.sea_level_raw,
                    retail_tick: frame.retail_tick,
                },
            );
    velocity_raw[1] = 1000;
    Ok(PlayerDyingSurfaceContactOutcome {
        controller_matched: matched,
        normal_velocity_raw,
        unsupported_haptic: matched.then_some(unsupported_haptic).flatten(),
        wreck_burst_emitted,
        next_burst_retail_tick: matched.then(|| {
            frame
                .runtime
                .as_ref()
                .expect("admitted runtime")
                .next_burst_tick()
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity_behavior::{BehaviorChoiceListSource, BehaviorSelection};

    #[test]
    fn contact_authentication_rejects_foreign_active_owner_before_rng_or_velocity() {
        let control =
            BehaviorContextRuntime::from_published_weighted_selection(BehaviorSelection {
                choice_index: 0,
                program: audited_behavior_program(24).unwrap(),
            })
            .unwrap();
        assert!(is_player_style(control, 24));
        assert_eq!(
            player_surface_style(control),
            Some(PlayerSurfaceStyle::PlayerControl)
        );
        assert!(!is_player_style(
            control.with_initializer_failure_fallback(),
            24
        ));
        assert_eq!(
            player_surface_style(control.with_initializer_failure_fallback()),
            None
        );
        let dying = audited_behavior_program(25).unwrap();
        assert!(
            BehaviorContextRuntime::named_audited(
                dying,
                0,
                RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
                RetailRuntimeValue::Known(None),
                RetailRuntimeValue::Known(0),
                control.active_style().audited().unwrap()
            )
            .is_none(),
            "foreign active style cannot authenticate under the class25 descriptor"
        );
        let mut fx = WorldFx::new();
        let before = *fx.entity_construction_state().0;
        let mut v = [1, -2000, 3];
        let result = apply_player_dying_surface_contact(
            PlayerDyingSurfaceContactFrame {
                request: PlayerContactStyleRequest {
                    behavior_context: RetailRuntimeValue::Known(Some(control)),
                    entity_handle: 46,
                    controlled_entity_handle: Some(46),
                    haptic_scale_raw: 15,
                },
                runtime: &mut None,
                world_fx: &mut fx,
                source_extent_raw: 1,
                sea_level_raw: None,
                retail_tick: 1,
            },
            [0; 3],
            [0, 4096, 0],
            &mut v,
        );
        assert_eq!(result, Err(PlayerContactStyleBlock::UnauthenticatedStyle));
        assert_eq!(v, [1, -2000, 3]);
        assert_eq!(*fx.entity_construction_state().0, before);
        assert_eq!(fx.particle_count(), 0);
    }
}
