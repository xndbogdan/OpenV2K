//! `42E440` runs after each complete authored birth, before the next allocation.
//! Loading a completed world still constructs actors and consumes their RNG.

use super::*;
use v2k_formats::levels::EntitySpawn;

impl EntityManager {
    pub(super) fn apply_completed_world_birth(
        &mut self,
        id: u32,
        spawn: &EntitySpawn,
        world_flags: u32,
        completed_world: Option<&mut NativeSaveWorldState<'_>>,
        type_metadata: &[EntityTypeRuntimeMetadata],
        world_fx: &mut WorldFx,
    ) -> Result<(), String> {
        if world_flags & 3 == 0 {
            return Ok(());
        }
        let saved = world_flags & 1 != 0;
        let entity = self
            .entities
            .iter_mut()
            .find(|e| e.id == id)
            .ok_or("42E440 missing birth")?;
        if saved && entity.capability_flags & 0x80 != 0 {
            // 18C20(MAX) suppresses delivery messages; the production owner
            // retains the signed count/countdown, and the model view its low word.
            let RetailRuntimeValue::Known(Some(base)) = &mut entity.base_factory_runtime else {
                return Err("42E440 Sub-M runtime unavailable".into());
            };
            let production = base
                .production
                .as_mut()
                .ok_or("42E440 staffing owner unavailable")?;
            production.adjust_staffing(i32::MAX);
            base.current_scientists = production.current_scientists_raw as u16;
        }
        // Re-read the live capability after staffing, as retail does.
        if entity.capability_flags & 0x10 != 0 {
            if saved {
                self.publish_completed_world_hive_death(
                    id,
                    completed_world.ok_or("native save terrain owner")?,
                    type_metadata,
                    world_fx,
                )?;
            }
            return Ok(());
        }
        if (saved && entity.capability_flags & 8 != 0)
            || (spawn.entity_type == 61 && spawn.extra[8] == 0x3f && world_flags & 2 != 0)
        {
            // 10B70 stages removal; constructors following this one can still
            // see its allocation and exact pending-state bits until the sweep.
            entity.mark_actor_deferred_destroy_pending();
            self.queue_actor_deferred_destroy(id);
        }
        Ok(())
    }

    fn publish_completed_world_hive_death(
        &mut self,
        id: u32,
        saved: &mut NativeSaveWorldState<'_>,
        type_metadata: &[EntityTypeRuntimeMetadata],
        world_fx: &mut WorldFx,
    ) -> Result<(), String> {
        let entity = self
            .entities
            .iter()
            .find(|e| e.id == id)
            .ok_or("42E440 missing death target")?;
        if entity.entity_type != HIVE_ENTITY_TYPE {
            return Err(format!(
                "42E440 ordinary death owner unavailable for type{}",
                entity.entity_type
            ));
        }
        let metadata = type_metadata
            .get(HIVE_ENTITY_TYPE as usize)
            .ok_or("Hive metadata")?;
        if metadata.constructor_sound_attachment_id != RetailRuntimeValue::Known(None) {
            return Err("42E440 Hive attached voice owner unavailable".into());
        }
        let sound = match metadata.death_sound_id {
            RetailRuntimeValue::Known(sound) => sound,
            _ => return Err("42E440 Hive death sound unavailable".into()),
        };
        let position = entity.position_raw();
        let model =
            crate::hive_death::hive_dying_model(entity.model_slots).ok_or("Hive model pair")?;
        let extent = saved
            .resources
            .global_model(model)
            .map(|m| m.radius)
            .ok_or("Hive dying model extent")?;
        let template = crate::hive_death::hive_death_radial_template(id);

        let entity = self.entities.iter_mut().find(|e| e.id == id).unwrap();
        if entity
            .collision
            .state_flags_at_0x08
            .masked(REMOTE_OWNED_STATE_BIT)
            != RetailRuntimeValue::Known(0)
        {
            return Err("42E440 Hive remote ownership unavailable".into());
        }
        entity.collision.health_raw = RetailRuntimeValue::Known(0);
        entity
            .apply_hive_dying_initializer()
            .ok_or("42E440 Hive initializer unavailable")?;
        if let Some(sound) = sound {
            world_fx.queue_fixed_positional_sound_raw(sound, position);
        }
        crate::hive_death::emit_hive_dying_surface_burst(
            world_fx,
            position,
            extent,
            saved
                .terrain
                .water_enabled()
                .then(|| saved.terrain.sea_level_raw()),
            id,
        );
        apply_constructor_static_radial(saved, position, template, world_fx)?;

        // The player already exists at this point. Supply its actual restored
        // hull to the shared callback-free radial owner; unresolved actor
        // callbacks remain an explicit load error rather than skipped damage.
        let player_metadata = type_metadata.get(46).ok_or("native player metadata")?;
        let mut hull = crate::player_hull::PlayerHull::new(crate::player_hull::HullDamageProfile {
            max_health_raw: player_metadata
                .initial_health_raw
                .ok_or("native player max health")?,
            mass_raw: player_metadata.mass_raw,
            damage: player_metadata
                .damage_profile
                .ok_or("native player damage profile")?,
        });
        let player = self.player().ok_or("native player unavailable")?;
        let RetailRuntimeValue::Known(health) = player.collision.health_raw else {
            return Err("native player health".into());
        };
        let RetailRuntimeValue::Known(shield) = player.collision.pre_health_damage_buffer_raw
        else {
            return Err("native player shield".into());
        };
        hull.health_raw = health;
        hull.pre_health_damage_buffer_raw = shield;
        match self.apply_dynamic_radial_damage(
            radial::DynamicRadialPlayerContext::Present(&mut hull),
            position,
            template,
        ) {
            radial::DynamicRadialDamageOutcome::Applied(outcome)
                if outcome.hive_dying_bursts.is_empty() =>
            {
                for sound in outcome.sounds {
                    world_fx.queue_fixed_positional_sound_raw(sound.sound_id, sound.position_raw);
                }
                Ok(())
            }
            outcome => Err(format!("42E440 Hive radial owner: {outcome:?}")),
        }
    }
}

/// Static targets resolve the private live terrain and the canonical global
/// model pool. Immediate burns complete before the dynamic walk or next birth.
fn apply_constructor_static_radial(
    saved: &mut NativeSaveWorldState<'_>,
    origin: [i16; 3],
    template: crate::radial_damage::RadialDamageTemplate,
    world_fx: &mut WorldFx,
) -> Result<(), String> {
    use crate::static_damage::{scan_static_radial, StaticDamageOutcome};
    let table = saved
        .resources
        .terrain_objects()
        .ok_or("completed world static table")?;
    let mut lookup_error = None;
    let hits = scan_static_radial(saved.terrain, origin, template, |cell| {
        let current = *saved.terrain.cell(cell[0] as usize, cell[1] as usize)?;
        match crate::static_damage_live::resolve_static_damage_snapshot_from_cell(
            cell,
            current,
            table,
            saved.resources,
        ) {
            Ok(snapshot) => snapshot.map(|snapshot| snapshot.target.state),
            Err(error) => {
                lookup_error = Some(error);
                None
            }
        }
    });
    if let Some(error) = lookup_error {
        return Err(format!("42E440 static lookup: {error:?}"));
    }
    for hit in hits {
        let outcome = saved
            .static_damage
            .submit_hit(hit.target, hit.packet, &mut || {
                world_fx.next_shared_retail_random_u16()
            });
        match outcome {
            StaticDamageOutcome::ImmediateBurn { cell, .. } => {
                crate::static_terrain_burn::apply_constructor_static_burn(
                    cell,
                    saved.terrain,
                    table,
                    saved.resources,
                    world_fx,
                )?;
            }
            StaticDamageOutcome::BurnedKind10Transition { cell, .. } => {
                let terrain_cell =
                    &mut saved.terrain.cells[cell[0] as usize * 256 + cell[1] as usize];
                terrain_cell.terrain_type &= !8;
                terrain_cell.attribute = terrain_cell.attribute.wrapping_add(1);
            }
            StaticDamageOutcome::UnsupportedKind { .. } => {
                return Err(format!("42E440 static radial: {outcome:?}"))
            }
            _ => {}
        }
    }
    Ok(())
}
