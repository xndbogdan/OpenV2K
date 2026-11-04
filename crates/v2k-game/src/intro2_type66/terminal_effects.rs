//! Synchronous `37390` template effects, before the factory's status clear.

use super::death::{
    Intro2Type66DeathBlock, Intro2Type66TerminalEffectRequest, Intro2Type66TerminalEffectsOutcome,
};
use crate::{
    entity::EntityManager,
    resource_cache::ResourceCache,
    static_damage::StaticDamageScheduler,
    terrain_crater::{
        apply_world_terrain_crater, crater_material_for_world_style, TerrainCraterRequest,
    },
    world_fx::WorldFx,
};

/// Retail FUN_00437390 for the authored factory templates. A deferred (0x01)
/// template only queues its terrain ripple, which `4374B0` advances and
/// finishes on later frames; the radial (0x04) bit without it stays blocked.
pub(super) fn apply_intro2_type66_terminal_effects(
    manager: &mut EntityManager,
    resources: &mut ResourceCache,
    world_fx: &mut WorldFx,
    static_damage: &mut StaticDamageScheduler,
    request: Intro2Type66TerminalEffectRequest,
) -> Result<Intro2Type66TerminalEffectsOutcome, Intro2Type66DeathBlock> {
    let flags = request.template_words[0];
    let mut outcome = Intro2Type66TerminalEffectsOutcome {
        presentation_requested: false,
        terrain_changed: false,
    };
    let block = |phase, outcome: Intro2Type66TerminalEffectsOutcome| Intro2Type66DeathBlock {
        phase,
        committed_prefix: true,
        presentation_requested: outcome.presentation_requested,
        terrain_changed: outcome.terrain_changed,
    };
    if flags & 1 != 0 {
        let w = request.template_words;
        world_fx.queue_terrain_ripple(crate::terrain_ripple::TerrainRipple::new(
            request.position_raw,
            crate::terrain_ripple::TerrainRippleTemplate {
                flags,
                radius_raw: w[1] as i32,
                height: w[2] as i32,
                period_us: w[3] as i32,
                radial_words: w[4..12].try_into().expect("eight radial words"),
                crater_radius_raw: w[12] as i32,
                crater_depth: w[13] as i32,
            },
        ));
        return Ok(outcome);
    }
    outcome.presentation_requested = flags & 2 != 0;
    if flags & 4 != 0 {
        return Err(block("terminal radial template", outcome));
    }
    if flags & 8 != 0 {
        let world_style = resources
            .level_desc()
            .ok_or_else(|| block("crater world", outcome))?
            .world_style;
        let crater = TerrainCraterRequest {
            position_raw: request.position_raw,
            radius_raw: request.template_words[12] as i32,
            depth_height_units: request.template_words[13] as i32,
            replacement_material: crater_material_for_world_style(world_style),
        };
        let report =
            apply_world_terrain_crater(manager, resources, world_fx, static_damage, crater)
                .map_err(|failure| {
                    outcome.terrain_changed |= failure.terrain_changed;
                    block(failure.phase, outcome)
                })?;
        outcome.terrain_changed |= report.terrain_changed;
    }
    Ok(outcome)
}
