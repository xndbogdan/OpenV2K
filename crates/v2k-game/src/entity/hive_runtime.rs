//! Live Hive component custody, dynamic births and forced-contact publication.

use super::*;
use crate::hive_controller::{hive_wreck_retry_requested, HiveWreckPlayerContactFrame};
use crate::hive_impact::{
    evaluate_hive_impact, HiveImpactBlock, HiveImpactOutcome, HiveImpactRequest,
};

#[cfg(test)]
mod tests;

impl EntityManager {
    /// `4259F0 -> 1CE10` writes only the retained Sub-N latch. Its later
    /// component visit owns death, replacement tasks and nested effects.
    pub(crate) fn apply_live_hive_pair_impact(
        &mut self,
        hive_id: u32,
        opposite_id: u32,
    ) -> Result<Option<HiveImpactOutcome>, HiveImpactBlock> {
        let opposite = self
            .entities
            .iter()
            .find(|entity| entity.id == opposite_id)
            .ok_or(HiveImpactBlock::MissingOpposite {
                entity_id: opposite_id,
            })?;
        // The consuming people branch precedes the impact branch in 4259F0.
        if opposite.capability_flags & 0x0C00 != 0 || opposite.capability_flags & 0x2000 == 0 {
            return Ok(None);
        }
        let opposite_position_raw = opposite.position_raw();
        let opposite_velocity_raw = opposite.velocity_raw();
        let opposite_mass_raw = opposite.mass_raw;
        let index = self
            .entities
            .iter()
            .position(|entity| entity.id == hive_id)
            .ok_or(HiveImpactBlock::MissingHive { entity_id: hive_id })?;
        let hive = &self.entities[index];
        let metadata = self.type_metadata.get(hive.entity_type as usize);
        if !crate::hive_controller::live_component_authenticates(hive, metadata) {
            return Err(HiveImpactBlock::UnauthenticatedLiveHive { entity_id: hive_id });
        }
        let payload = metadata
            .and_then(|metadata| metadata.sub_n_payload)
            .ok_or(HiveImpactBlock::MissingDescriptorAttachment { entity_id: hive_id })?;
        let descriptor_attachment_raw = std::array::from_fn(|axis| {
            i16::from_le_bytes([payload[4 + axis * 2], payload[5 + axis * 2]])
        });
        let outcome = evaluate_hive_impact(HiveImpactRequest {
            hive_position_raw: hive.position_raw(),
            descriptor_attachment_raw,
            opposite_position_raw,
            opposite_velocity_raw,
            opposite_mass_raw,
        })?;
        if outcome.forced_death {
            let RetailRuntimeValue::Known(Some(runtime)) = &mut self.entities[index].sub_n_runtime
            else {
                return Err(HiveImpactBlock::UnresolvedSubN { entity_id: hive_id });
            };
            runtime.request_forced_death();
        }
        Ok(Some(outcome))
    }
}

impl EntityManager {
    /// One live-list callback prefix, with terrain writes visible to its successor.
    pub(crate) fn advance_authored_hive_component_prefix(
        &mut self,
        entity_id: u32,
        mut frame: AuthoredHiveComponentFrame<'_>,
        mode: ComponentUpdateMode,
        effects: &mut impl AuthoredHiveComponentEffects,
    ) -> Result<Vec<InfectionCellWrite>, AuthoredHiveInfectionBlock> {
        let mut terrain = frame.terrain.map(InfectionTerrainSnapshot::from_terrain);
        self.advance_hive_component_state(entity_id, &mut frame, mode, &mut terrain, effects)?;
        terrain
            .map(InfectionTerrainSnapshot::into_writes)
            .ok_or(AuthoredHiveInfectionBlock::MissingTerrain)
    }

    pub(super) fn advance_hive_component_state(
        &mut self,
        entity_id: u32,
        frame: &mut AuthoredHiveComponentFrame<'_>,
        mode: ComponentUpdateMode,
        infection_terrain: &mut Option<InfectionTerrainSnapshot>,
        effects: &mut impl AuthoredHiveComponentEffects,
    ) -> Result<(), AuthoredHiveInfectionBlock> {
        let Some(index) = self.entities.iter().position(|entity| {
            entity.id == entity_id && entity.active && entity.authored_radial_emitter.is_some()
        }) else {
            return Ok(());
        };
        // The 1BEB0 forced branch emits at retained Sub-N+54 before its later
        // state0 visit enters10C10. Keep that pending until the complete nested
        // effect host exists; a damage packet/instant25F60 would change ordering.
        if matches!(self.entities[index].sub_n_runtime,
            RetailRuntimeValue::Known(Some(runtime)) if runtime.forced_death_requested())
        {
            return Err(
                AuthoredHiveInfectionBlock::UnsupportedHiveComponentDeathEffects { entity_id },
            );
        }
        if !self.entities[index]
            .authored_radial_emitter
            .as_ref()
            .unwrap()
            .dying_slot0()
        {
            crate::hive_controller::live_health_inputs(
                &self.entities[index],
                self.type_metadata
                    .get(self.entities[index].entity_type as usize),
            )?;
        }
        let elapsed_us = frame.elapsed_us;
        let objective_hostile_present = objective_hostile_present(&self.entities).map_err(
            |ObjectivePredicateBlock::UnresolvedObjectiveState { entity_id }| {
                AuthoredHiveInfectionBlock::UnresolvedObjectiveState { entity_id }
            },
        );
        let entity = &mut self.entities[index];
        let position_raw = entity.position_raw();
        let source_id = entity.id;
        let dying = entity
            .authored_radial_emitter
            .as_ref()
            .unwrap()
            .dying_slot0();
        let (mut health_raw, authored_health_raw) = if dying {
            (0, 0)
        } else {
            crate::hive_controller::live_health_inputs(
                entity,
                self.type_metadata.get(entity.entity_type as usize),
            )
            .expect("live inputs preflighted before the shared component pass")
        };
        if let Some(emitter) = entity.authored_radial_emitter.as_mut() {
            match (objective_hostile_present, infection_terrain.as_mut()) {
                (Ok(objective_hostile_present), Some(infection_terrain)) => {
                    emitter.advance_component(
                        crate::entity_emitters::HiveComponentVisit {
                            objective_hostile_present,
                            mode,
                            source_position_raw: position_raw,
                            source_id,
                            health: if dying {
                                HiveComponentHealth::Dying
                            } else {
                                let RetailRuntimeValue::Known(Some(sub_n)) =
                                    &mut entity.sub_n_runtime
                                else {
                                    unreachable!("validated Sub-N")
                                };
                                HiveComponentHealth::Live(HiveLiveHealth {
                                    health_raw: &mut health_raw,
                                    authored_health_raw,
                                    grace_timer_us: sub_n.live_grace_timer_us_mut(),
                                })
                            },
                        },
                        frame,
                        infection_terrain,
                        effects,
                    );
                }
                _ => {
                    if !emitter.dying_slot0() {
                        emitter.advance_sub_k(elapsed_us, mode);
                    }
                    emitter.advance_wreck_timer(elapsed_us);
                    emitter.advance_radial(elapsed_us, mode, position_raw, source_id, |emission| {
                        effects.emit_authored_radial(emission)
                    });
                }
            }
        }
        if !dying {
            entity.collision.health_raw = RetailRuntimeValue::Known(health_raw);
        }
        objective_hostile_present.map(|_| ())
    }

    /// DCA0 follows every component phase, including creature/ejection RNG.
    pub(crate) fn finish_authored_hive_component(
        &mut self,
        entity_id: u32,
        elapsed_us: u32,
        mode: ComponentUpdateMode,
        effects: &mut impl AuthoredHiveComponentEffects,
    ) {
        if let Some(emitter) = self
            .entity_mut(entity_id)
            .and_then(|entity| entity.authored_radial_emitter.as_mut())
        {
            if emitter.dying_slot0() {
                emitter.advance_sub_k(elapsed_us, mode);
            }
        }
        let Some(entity) = self
            .entities
            .iter()
            .find(|entity| entity.id == entity_id && entity.active)
        else {
            return;
        };
        // 12DA0 -> DCA0 runs its descriptor sound gate after the complete
        // task cursor, including the Hive's Sub-N/K work. This is a
        // visible-actor stochastic cue, not a projectile or camera event.
        if mode == ComponentUpdateMode::Detailed
            && entity
                .authored_radial_emitter
                .as_ref()
                .is_some_and(|emitter| emitter.behavior_enabled())
        {
            match (
                self.type_metadata
                    .get(entity.entity_type as usize)
                    .map(|metadata| metadata.detailed_sound_policy),
                entity.collision.health_raw,
                entity.collision.state_flags_at_0x08.masked(0x800),
            ) {
                (
                    Some(RetailRuntimeValue::Known(policy)),
                    RetailRuntimeValue::Known(health),
                    RetailRuntimeValue::Known(visible),
                ) => {
                    if let Some(sound_id) =
                        policy.plan(health, visible != 0, elapsed_us, &mut || {
                            u32::from(effects.next_shared_random_u16())
                        })
                    {
                        effects.queue_hive_detailed_sound(sound_id, entity.position_raw());
                    }
                }
                _ => eprintln!(
                    "Hive entity{}: unresolved DCA0 sound policy/state; cue skipped",
                    entity.id
                ),
            }
        }
    }
}

impl EntityManager {
    /// Source1BEB0 wreck contact, after the controlled-player integration.
    /// The port stages that integration separately from13500; retain this
    /// complete no-RNG contact phase at the proven post-player boundary.
    /// Its clock was advanced exactly once by the Hive's component visit.
    /// Return456D10's interior request without selecting an authored marker;
    /// the campaign selector owns the retained arrival and current-world retry.
    pub fn apply_authored_hive_wreck_player_contacts(
        &mut self,
        frame: HiveWreckPlayerContactFrame,
    ) -> bool {
        let Some(player) = self
            .player()
            .filter(|player| hive_wreck_player_contact_eligible(player))
        else {
            return false;
        };
        let player_position_raw = player.position_raw();
        let mut player_velocity_raw = player.velocity_raw();
        let mut changed = false;
        let mut retry_requested = false;
        for entity in &self.entities {
            if !entity.active {
                continue;
            }
            let Some(origin) = hive_wreck_contact_origin(entity) else {
                continue;
            };
            let Some(emitter) = entity.authored_radial_emitter.as_ref() else {
                continue;
            };
            if let Some(velocity) = emitter.apply_wreck_suction(
                frame.elapsed_us,
                origin,
                player_position_raw,
                player_velocity_raw,
            ) {
                player_velocity_raw = velocity;
                changed = true;
                // Source admits only a dead/null Hive to the retry branch.
                // Retained task custody alone cannot forge that body state.
                let state = entity.collision.state_flags_at_0x08;
                let hive_dead = state.masked(0x4000) == RetailRuntimeValue::Known(0x4000)
                    || state.masked(u32::MAX) == RetailRuntimeValue::Known(0);
                retry_requested |= hive_dead
                    && hive_wreck_retry_requested(
                        origin,
                        player_position_raw,
                        frame.session_aborted,
                    );
            }
        }
        if changed {
            if let Some(player) = self.player_mut() {
                player.set_velocity_raw(player_velocity_raw);
            }
        }
        retry_requested
    }
}
