//! E370's fish surface profiles: no selectors, or Zebra's +72=0/+73=1.
//!
//! The latter first visits 162B0 above the strict static sea plane, then
//! *still* reaches the shared timer decay at40E607. Below/equal sea therefore
//! decays twice; above sea ordinarily adds then subtracts the same delta.
//! PE40E3C8 ->40E3F1 ->40E402 ->40E607 proves this asymmetric fallthrough.

use crate::{
    common_mover::type9_surface::{
        decay_actor_surface_timer_ms, ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT,
    },
    entity::Entity,
    entity_collision_state::{CommonWorldEffectProfile, RetailRuntimeValue},
};

use super::SharedFishBlock;

/// The authenticated detailed owner calls this after rebuilding the basis and
/// applying any bit8 environment forces, before common master integration.
pub(super) fn run(
    entity: &mut Entity,
    effects: RetailRuntimeValue<CommonWorldEffectProfile>,
    flat_surface_y_raw: i16,
    elapsed_micros: u32,
) -> Result<(), SharedFishBlock> {
    use SharedFishBlock as Block;
    let RetailRuntimeValue::Known(disabled) = entity
        .collision
        .state_flags_at_0x08
        .masked(ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT)
    else {
        return Err(Block::Runtime("fish surface owner bit"));
    };
    if disabled != 0 {
        return Ok(());
    }
    let RetailRuntimeValue::Known(effects) = effects else {
        return Err(Block::Metadata);
    };
    if !matches!(effects.surface_selectors, [0, 0] | [0, 1])
        || (effects.surface_selectors == [0, 1] && effects.surface_lifetime_ms == 0)
    {
        return Err(Block::Metadata);
    }
    let RetailRuntimeValue::Known(mut timer) = entity.surface_lifetime_timer_ms_at_0x48 else {
        return Err(Block::Runtime("fish surface timer"));
    };
    if effects.surface_selectors[1] != 0 {
        if entity.position_raw()[1] > flat_surface_y_raw {
            // 162B0 publishes +48 before its signed remaining-duration test.
            timer = timer.wrapping_add(elapsed_micros / 1000);
            entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(timer);
            if effects.surface_lifetime_ms.wrapping_sub(timer) as i32 <= 0 {
                // 16750's relation release and10C10 quiet death must complete
                // synchronously before E370 can apply its final timer decay.
                // Preserve the reached prefix and let the outer owner retain
                // its pending receipt; never silently skip those callbacks.
                return Err(Block::Runtime(
                    "fish surface expiry release/death continuation",
                ));
            }
        } else {
            timer = decay_actor_surface_timer_ms(timer, elapsed_micros);
        }
    }
    // +72 is zero in both admitted profiles, so there is no model-extent
    // lookup, bubble gate, sound gate or RNG use before this final decay.
    entity.surface_lifetime_timer_ms_at_0x48 =
        RetailRuntimeValue::Known(decay_actor_surface_timer_ms(timer, elapsed_micros));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{entity::EntityKind, entity_collision_state::RetailStateWord};

    fn zebra(timer: u32, y: i16) -> Entity {
        let mut entity = Entity::unresolved_port_entity(1, EntityKind::Unknown(62), 62);
        entity.collision.state_flags_at_0x08 = RetailStateWord::exact(0);
        entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(timer);
        entity.set_position_raw([0, y, 0]);
        entity
    }

    fn effects(selectors: [u8; 2]) -> RetailRuntimeValue<CommonWorldEffectProfile> {
        RetailRuntimeValue::Known(CommonWorldEffectProfile {
            surface_selectors: selectors,
            surface_lifetime_ms: if selectors == [0, 1] { 1000 } else { 0 },
            low_health_effect_words: [0; 3],
        })
    }

    #[test]
    fn zebra_above_sea_adds_then_decays_but_submerged_and_equality_decay_twice() {
        for (selectors, y, timer_after) in [
            ([0, 0], 1, 80),
            ([0, 0], -1, 80),
            ([0, 1], 1, 100),
            ([0, 1], 0, 60),
            ([0, 1], -1, 60),
        ] {
            let mut entity = zebra(100, y);
            assert_eq!(run(&mut entity, effects(selectors), 0, 20_999), Ok(()));
            assert_eq!(
                entity.surface_lifetime_timer_ms_at_0x48,
                RetailRuntimeValue::Known(timer_after)
            );
        }
    }

    #[test]
    fn fresh_zebra_does_not_accumulate_an_invented_dry_land_timer() {
        for kind in [23, 62] {
            let mut entity = zebra(0, 1);
            entity.entity_type = kind;
            for _ in 0..100 {
                assert_eq!(run(&mut entity, effects([0, 1]), 0, 125_000), Ok(()));
            }
            assert_eq!(
                entity.surface_lifetime_timer_ms_at_0x48,
                RetailRuntimeValue::Known(0)
            );
        }
    }

    #[test]
    fn expiry_retains_the_published_timer_and_stops_before_final_decay() {
        for (timer, elapsed, after) in [(980, 20_000, 1000), (990, 20_000, 1010)] {
            let mut entity = zebra(timer, 1);
            assert_eq!(
                run(&mut entity, effects([0, 1]), 0, elapsed),
                Err(SharedFishBlock::Runtime(
                    "fish surface expiry release/death continuation"
                ))
            );
            assert_eq!(
                entity.surface_lifetime_timer_ms_at_0x48,
                RetailRuntimeValue::Known(after)
            );
        }
    }

    #[test]
    fn disabled_surface_does_not_require_the_timer_or_profile() {
        let mut entity = zebra(0, 1);
        entity.collision.state_flags_at_0x08 =
            RetailStateWord::exact(ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT);
        entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Unresolved;
        assert_eq!(
            run(&mut entity, RetailRuntimeValue::Unresolved, 0, 20_000),
            Ok(())
        );
        assert_eq!(
            entity.surface_lifetime_timer_ms_at_0x48,
            RetailRuntimeValue::Unresolved
        );
    }

    #[test]
    fn double_decay_saturates_and_rejects_unowned_selectors_before_writes() {
        let mut entity = zebra(30, 0);
        assert_eq!(run(&mut entity, effects([0, 1]), 0, 20_000), Ok(()));
        assert_eq!(
            entity.surface_lifetime_timer_ms_at_0x48,
            RetailRuntimeValue::Known(0)
        );
        entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(42);
        assert_eq!(
            run(&mut entity, effects([1, 0]), 0, 20_000),
            Err(SharedFishBlock::Metadata)
        );
        assert_eq!(
            entity.surface_lifetime_timer_ms_at_0x48,
            RetailRuntimeValue::Known(42)
        );
    }
}
