//! One complete retained Hive visit at its `13500` live-list position.

use super::*;
use crate::{
    entity::{EntityConstructionResources, HiveBirthManagerContext, HiveBirthManagerHost},
    entity_emitters::{AuthoredHiveComponentEffects, AuthoredRadialEmission, ComponentUpdateMode},
    hive_birth::HiveBirthVisit,
    hive_controller::AuthoredHiveComponentFrame,
    infection_evolution::{
        InfectionCellWrite, InfectionEvolutionEffects, InfectionTailSound, InfectionTerrainSnapshot,
    },
    world_complete_results::WorldCompleteTally,
};

#[cfg(test)]
mod fallback_tests;
#[cfg(test)]
mod tests;

pub struct HiveComponentProductionContext<'a> {
    pub world_complete_tally: &'a mut WorldCompleteTally,
    pub view_detail: RetailViewDetailContext,
}

pub(super) struct HiveComponentProductionPass {
    pub newborn_owners: Vec<crate::entity::NativeHiveNewborn>,
    pub terrain_changed: bool,
    pub diagnostics: Vec<String>,
}

/// The production Hive's effects: `FUN_00436960` and `FUN_00436750` write
/// the terrain through `FUN_00433720` and its listeners between their draws.
struct LiveHiveEffects<'a> {
    resources: &'a mut ResourceCache,
    world_fx: &'a mut WorldFx,
    blocked: Option<&'static str>,
}

impl InfectionEvolutionEffects for LiveHiveEffects<'_> {
    fn next_shared_random_u16(&mut self) -> u16 {
        self.world_fx.next_shared_retail_random_u16()
    }

    fn queue_infection_tail_sound(&mut self, sound: InfectionTailSound) {
        self.world_fx.queue_infection_tail_sound(sound);
    }

    fn write_infection(&mut self, write: InfectionCellWrite) {
        if let Err(listener) =
            crate::infection_listeners::write_infection(self.resources, self.world_fx, write)
        {
            self.blocked.get_or_insert(listener);
        }
    }
}

impl AuthoredHiveComponentEffects for LiveHiveEffects<'_> {
    fn emit_authored_radial(&mut self, emission: AuthoredRadialEmission) {
        self.world_fx.emit_authored_radial(emission);
    }

    fn queue_hive_detailed_sound(&mut self, sound_id: u16, position_raw: [i16; 3]) {
        self.world_fx
            .queue_hive_detailed_sound(sound_id, position_raw);
    }
}

pub(super) fn tick_hive_component(
    manager: &mut EntityManager,
    entity_id: u32,
    resources: &mut ResourceCache,
    world_fx: &mut WorldFx,
    context: &mut HiveComponentProductionContext<'_>,
    notifications: &mut GameplayNotifications,
    elapsed_us: u32,
    retail_tick: u32,
    notification_phase: GameplayNotificationPhase,
    session_aborted: bool,
) -> HiveComponentProductionPass {
    let mut pass = HiveComponentProductionPass {
        newborn_owners: Vec::new(),
        terrain_changed: false,
        diagnostics: Vec::new(),
    };
    let Some(entity) = manager.iter_all().find(|entity| {
        entity.id == entity_id && entity.active && entity.authored_radial_emitter.is_some()
    }) else {
        return pass;
    };
    let source_position_raw = entity.position_raw();
    let mode = if context
        .view_detail
        .classify(source_position_raw)
        .uses_detailed_update()
    {
        ComponentUpdateMode::Detailed
    } else {
        ComponentUpdateMode::Coarse
    };
    let terrain = resources
        .level_terrain()
        .map(InfectionTerrainSnapshot::from_terrain);
    let mut effects = LiveHiveEffects {
        resources: &mut *resources,
        world_fx: &mut *world_fx,
        blocked: None,
    };
    let writes = manager.advance_authored_hive_component_prefix(
        entity_id,
        AuthoredHiveComponentFrame {
            elapsed_us,
            terrain: None,
            retail_tick: retail_tick as i32,
            notification_phase,
            notifications,
            world_complete_tally: context.world_complete_tally,
        },
        terrain,
        mode,
        &mut effects,
    );
    if let Some(listener) = effects.blocked {
        pass.diagnostics.push(format!(
            "Hive{entity_id}: infection write blocked: {listener}"
        ));
    }
    let birth_prefix_admitted = match writes {
        Ok(writes) => {
            // Each write reached the terrain, with its listeners, as it ran.
            pass.terrain_changed = !writes.is_empty();
            true
        }
        Err(block) => {
            pass.diagnostics.push(format!(
                "Hive{entity_id}: component prefix blocked: {block:?}"
            ));
            false
        }
    };
    let emitter = manager
        .entity_mut(entity_id)
        .unwrap()
        .authored_radial_emitter
        .as_mut()
        .unwrap();
    if !emitter.behavior_enabled() {
        manager.finish_authored_hive_component(entity_id, elapsed_us, mode, world_fx);
        return pass;
    }
    let state = emitter.controller_state();
    if let Some(block) = emitter.birth_decode_error() {
        pass.diagnostics.push(format!(
            "Hive{entity_id}: authored creature rows blocked: {block:?}"
        ));
    }
    let birth_runtime = emitter.take_birth_runtime();
    if let Some(mut birth_runtime) = birth_runtime {
        {
            let waves_enabled = resources
                .level_desc()
                .and_then(|level| level.raw_u32(0x84))
                .map(|word| RetailRuntimeValue::Known(word != 0))
                .unwrap_or(RetailRuntimeValue::Unresolved);
            let model_extent = |id| resources.global_model(id).map(|model| model.radius);
            let resources = EntityConstructionResources {
                terrain: resources.level_terrain(),
                terrain_objects: resources.terrain_objects(),
                model_extent_raw: Some(&model_extent),
            };
            let mut host = HiveBirthManagerHost::new(
                manager,
                HiveBirthManagerContext {
                    resources,
                    retail_tick,
                    waves_enabled,
                },
                world_fx,
            );
            let blocks = if birth_prefix_admitted {
                birth_runtime.advance_rows(
                    HiveBirthVisit {
                        controller_state: state,
                        session_aborted,
                        elapsed_us,
                        source_id: entity_id,
                        source_position_raw,
                    },
                    &mut host,
                )
            } else {
                Vec::new()
            };
            // The mouth walk and the wreck suction run after the staged
            // controlled-player movement (crate::hive_mouth), after this 1CA90.
            let ejection_blocks = birth_runtime.advance_ejections(elapsed_us, &mut host);
            pass.newborn_owners = host.take_newborn_owners();
            pass.diagnostics.extend(
                blocks
                    .into_iter()
                    .chain(ejection_blocks)
                    .map(|block| format!("Hive{entity_id}: creature feature blocked: {block:?}")),
            );
        }
        manager
            .entity_mut(entity_id)
            .unwrap()
            .authored_radial_emitter
            .as_mut()
            .unwrap()
            .restore_birth_runtime(birth_runtime);
    }
    manager.finish_authored_hive_component(entity_id, elapsed_us, mode, world_fx);
    pass
}
