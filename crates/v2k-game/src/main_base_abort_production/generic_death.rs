//! `FUN_00410C10` on one live actor outside the abort walk.
//!
//! The Queen's dying visit kills every other living capability-8 body. Each
//! victim's death runs through the same per-type owner as the Main Base abort
//! walk's ordinary route, with the playing world's player and scheduler. A
//! Hive, which the walk sends through the alternate cleanup instead, takes
//! its own `FUN_00425790`.

use super::*;

/// The world a mid-frame `10C10` runs in: the scheduler holds every owner.
pub(crate) struct GenericDeathFrame<'a> {
    pub entities: &'a mut EntityManager,
    pub resources: &'a mut ResourceCache,
    pub static_damage: &'a mut StaticDamageScheduler,
    pub player_hull: &'a mut PlayerHull,
    pub world_fx: &'a mut WorldFx,
    pub scheduler: &'a mut SpecializedActorTaskScheduler,
    pub notifications: &'a mut crate::gameplay_notifications::GameplayNotifications,
    pub retail_tick: u32,
    pub extra_lives: RetailRuntimeValue<u8>,
}

/// Why a mid-frame `10C10` could not run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericDeathBlock {
    ActorMissing,
    Hive(crate::entity::HiveGenericDeathBlock),
    HiveStaticRadial(crate::static_damage_live::CurrentStaticDamageLookupError),
    HiveRadial(crate::specialized_actor_task_production::PlayingRadialBlock),
    Owner(MainBaseAbortProductionDiagnostic),
}

/// Run `10C10` on `id`; returns whether the death changed the terrain.
pub(crate) fn run_generic_death(
    id: u32,
    frame: GenericDeathFrame<'_>,
) -> Result<bool, GenericDeathBlock> {
    let GenericDeathFrame {
        entities,
        resources,
        static_damage,
        player_hull,
        world_fx,
        scheduler,
        notifications,
        retail_tick,
        extra_lives,
    } = frame;
    let actor = entities
        .main_base_abort_actor_observation(id)
        .ok_or(GenericDeathBlock::ActorMissing)?;
    if actor.entity_type == crate::hive_controller::HIVE_ENTITY_TYPE {
        let Some(death) = entities
            .apply_hive_generic_death(id, world_fx)
            .map_err(GenericDeathBlock::Hive)?
        else {
            return Ok(false);
        };
        let radial = crate::hive_death::apply_hive_death_burst_and_radial(
            crate::hive_death::HiveDeathBlastFrame {
                entities,
                cache: resources,
                world_fx,
                static_damage,
                player_hull,
                extra_lives,
                actor_tasks: scheduler,
                notifications,
                retail_tick,
            },
            death.blast,
        );
        if let Err(error) = radial.static_outcomes {
            return Err(GenericDeathBlock::HiveStaticRadial(error));
        }
        if let Some(block) = radial.dynamic.blocked {
            return Err(GenericDeathBlock::HiveRadial(block));
        }
        return Ok(radial.terrain_changed);
    }
    let geometry = snapshot_main_base_abort_terrain_geometry(resources).map_err(|error| {
        GenericDeathBlock::Owner(MainBaseAbortProductionDiagnostic::WorldEffects(error))
    })?;
    let mut effects =
        MainBaseAbortWorldEffects::new(&geometry, resources, static_damage).map_err(|error| {
            GenericDeathBlock::Owner(MainBaseAbortProductionDiagnostic::WorldEffects(error))
        })?;
    let result = dispatch_ordinary_actor(
        actor,
        &mut None,
        entities,
        geometry.terrain(),
        player_hull,
        world_fx,
        &mut effects,
        scheduler,
        &mut MainBaseAbortPublicationCounts::default(),
        &mut MainBaseAbortGameplayContext {
            notifications,
            retail_tick,
            extra_lives,
        },
    );
    let report = effects.finish();
    match result {
        Ok(_) => Ok(report.terrain_dirty()),
        Err(failure) => Err(GenericDeathBlock::Owner(failure.diagnostic.unwrap_or_else(
            || {
                MainBaseAbortProductionDiagnostic::ActorCallbackBlocked {
                    actor,
                    error: failure
                        .callback_error
                        .expect("dispatch failures identify callback or scheduler custody"),
                }
            },
        ))),
    }
}
