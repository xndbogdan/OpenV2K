//! One complete retained Hive visit at its `13500` live-list position.

use super::*;
use crate::{
    entity::{EntityConstructionResources, HiveBirthManagerContext, HiveBirthManagerHost},
    entity_emitters::ComponentUpdateMode,
    hive_birth::HiveBirthVisit,
    hive_controller::AuthoredHiveComponentFrame,
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
    pub newborn_owners: Vec<Intro2FlyerSchedulerOwner>,
    pub terrain_changed: bool,
    pub diagnostics: Vec<String>,
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
    let writes = manager.advance_authored_hive_component_prefix(
        entity_id,
        AuthoredHiveComponentFrame {
            elapsed_us,
            terrain: resources.level_terrain(),
            retail_tick: retail_tick as i32,
            notification_phase,
            notifications,
            world_complete_tally: context.world_complete_tally,
        },
        mode,
        world_fx,
    );
    let birth_prefix_admitted = match writes {
        Ok(writes) => {
            pass.terrain_changed = resources
                .apply_level_infection_writes(&writes)
                .is_some_and(|changed| changed != 0);
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
            // Wreck suction runs after the staged controlled-player movement.
            // Live people-consume/eject contact remains a separate
            // unowned feature; its absence never suppresses final1CA90.
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
