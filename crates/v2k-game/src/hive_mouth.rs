//! A live Hive's mouth: `FUN_0041BEB0`'s walk on a Hive without the dying
//! bit. Each body in the mouth ([`EntityManager::body_in_hive_mouth`]) dies
//! first if it lacks capability bit 3 (`FUN_00410C10`: the player's craft
//! among them, which starts the player's death), and then every one of them
//! is thrown back out of the mouth (`FUN_0041C830`, the ejection a newborn
//! takes): recorded in one of the Hive's two slots for half a second without
//! `0x8000`, set beside the mouth on the terrain and launched upward.
//!
//! The walk runs after the actor pass and the craft's movement, as the dying
//! Hive's swallow does, so the craft is tested where retail's walk sees it
//! (it leads retail's live list). The Hive's own visit already ran its final
//! `FUN_0041CA90`; the slots it cleared are still held for this walk and a
//! new launch ages by the visit's time here, as in retail.

use crate::entity::{
    EntityConstructionResources, EntityManager, HiveBirthManagerContext, HiveBirthManagerHost,
    PlayerCheckedDamageBlock, PlayerCheckedDamageFrame,
};
use crate::entity_collision_state::{RetailRuntimeValue, DYING_STATE_BIT, REMOTE_OWNED_STATE_BIT};
use crate::gameplay_notifications::GameplayNotifications;
use crate::hive_birth::{HiveEjectionOutcome, HiveEjectionRequest};
use crate::main_base_abort_production::{run_generic_death, GenericDeathBlock, GenericDeathFrame};
use crate::player_hull::PlayerHull;
use crate::resource_cache::ResourceCache;
use crate::specialized_actor_task_production::SpecializedActorTaskScheduler;
use crate::static_damage::StaticDamageScheduler;
use crate::world_fx::WorldFx;

/// `FUN_0041BEB0` kills only a body without this capability bit.
const SPARED_CAPABILITY_BIT: u32 = 8;

/// The world a live mouth writes.
pub struct LiveHiveMouthFrame<'a> {
    pub entities: &'a mut EntityManager,
    pub resources: &'a mut ResourceCache,
    pub static_damage: &'a mut StaticDamageScheduler,
    pub player_hull: &'a mut PlayerHull,
    pub world_fx: &'a mut WorldFx,
    pub scheduler: &'a mut SpecializedActorTaskScheduler,
    pub notifications: &'a mut GameplayNotifications,
    pub retail_tick: u32,
    /// The Hive visit's microseconds, which the trailing `FUN_0041CA90` ages.
    pub elapsed_us: u32,
    pub extra_lives: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiveHiveMouthEvent {
    /// `FUN_00410C10` ran on the body (it does nothing to a dying or remote
    /// one, which is not listed).
    Killed { hive: u32, body: u32 },
    Ejected {
        hive: u32,
        body: u32,
        outcome: HiveEjectionOutcome,
    },
    Blocked {
        hive: u32,
        body: u32,
        block: LiveHiveMouthBlock,
    },
}

/// Why a step of the live mouth could not run; the walk goes on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiveHiveMouthBlock {
    StateUnresolved,
    PlayerDeath(PlayerCheckedDamageBlock),
    Death(GenericDeathBlock),
    /// The Hive has no ejection slots: its creature rows failed to decode.
    NoEjectionSlots,
    Ejection(String),
    /// A death in the walk killed the Hive; retail's walk would go on with
    /// the dying Hive's swallow for the bodies after this one.
    HiveDiedInWalk,
    /// A death removed the body from the live list before its successor.
    LiveListLost,
}

#[derive(Debug, Default)]
pub struct LiveHiveMouthPass {
    pub events: Vec<LiveHiveMouthEvent>,
    pub terrain_changed: bool,
}

/// The walk on every live Hive with a mouth, in live order, body by body.
pub fn apply_live_hive_mouths(frame: LiveHiveMouthFrame<'_>) -> LiveHiveMouthPass {
    let LiveHiveMouthFrame {
        entities,
        resources,
        static_damage,
        player_hull,
        world_fx,
        scheduler,
        notifications,
        retail_tick,
        elapsed_us,
        extra_lives,
    } = frame;
    let mut pass = LiveHiveMouthPass::default();
    let hives: Vec<u32> = entities
        .retail_live_order_ids()
        .filter(|id| entities.live_hive_mouth(*id).is_some())
        .collect();
    for hive in hives {
        let index = entities.iter_all().position(|entity| entity.id == hive);
        let restored = index.map_or([None; 2], |index| {
            entities.take_hive_restored_this_visit(index)
        });
        let mut cursor = entities.retail_live_order_ids().next();
        while let Some(body) = cursor {
            let blocked = |block| LiveHiveMouthEvent::Blocked { hive, body, block };
            let Some(mouth) = entities.live_hive_mouth(hive) else {
                pass.events
                    .push(blocked(LiveHiveMouthBlock::HiveDiedInWalk));
                break;
            };
            // This visit's FUN_0041CA90 restored these after retail's walk.
            if entities.body_in_hive_mouth(mouth, body) && !restored.contains(&Some(body)) {
                let entity = entities
                    .iter_all()
                    .find(|entity| entity.id == body)
                    .expect("tested in the mouth");
                if entity.capability_flags & SPARED_CAPABILITY_BIT == 0 {
                    match entity
                        .collision
                        .state_flags_at_0x08
                        .masked(REMOTE_OWNED_STATE_BIT | DYING_STATE_BIT)
                    {
                        RetailRuntimeValue::Unresolved => pass
                            .events
                            .push(blocked(LiveHiveMouthBlock::StateUnresolved)),
                        // FUN_00410C10's entry: nothing for a remote or dying body.
                        RetailRuntimeValue::Known(0) => {
                            let killed = if entities.player().is_some_and(|p| p.id == body) {
                                kill_craft(
                                    entities,
                                    PlayerCheckedDamageFrame {
                                        hull: &mut *player_hull,
                                        resources: &*resources,
                                        world_fx: &mut *world_fx,
                                        retail_tick,
                                        notifications: &mut *notifications,
                                        extra_lives,
                                    },
                                )
                                .map_err(LiveHiveMouthBlock::PlayerDeath)
                            } else {
                                run_generic_death(
                                    body,
                                    GenericDeathFrame {
                                        entities: &mut *entities,
                                        resources: &mut *resources,
                                        static_damage: &mut *static_damage,
                                        player_hull: &mut *player_hull,
                                        world_fx: &mut *world_fx,
                                        scheduler: &mut *scheduler,
                                        notifications: &mut *notifications,
                                        retail_tick,
                                        extra_lives: RetailRuntimeValue::Known(extra_lives),
                                    },
                                )
                                .map(|terrain_changed| pass.terrain_changed |= terrain_changed)
                                .map_err(LiveHiveMouthBlock::Death)
                            };
                            pass.events.push(match killed {
                                Ok(()) => LiveHiveMouthEvent::Killed { hive, body },
                                Err(block) => blocked(block),
                            });
                        }
                        RetailRuntimeValue::Known(_) => {}
                    }
                }
                pass.events.push(eject(
                    entities,
                    resources,
                    world_fx,
                    retail_tick,
                    elapsed_us,
                    restored,
                    hive,
                    body,
                ));
            }
            cursor = match entities.retail_live_successor_id(body) {
                Ok(next) => next,
                Err(()) => {
                    pass.events.push(blocked(LiveHiveMouthBlock::LiveListLost));
                    break;
                }
            };
        }
    }
    pass
}

/// `FUN_00410C10` on the craft: the hull's lethal state, then its death's
/// constructor, as a physical hull death runs it.
fn kill_craft(
    entities: &mut EntityManager,
    frame: PlayerCheckedDamageFrame<'_>,
) -> Result<(), PlayerCheckedDamageBlock> {
    frame.hull.health_raw = 0;
    frame.hull.dying = true;
    entities.sync_player_hull_collision_state(frame.hull);
    entities.begin_player_dying(frame).map(|_| ())
}

#[allow(clippy::too_many_arguments)]
fn eject(
    entities: &mut EntityManager,
    resources: &ResourceCache,
    world_fx: &mut WorldFx,
    retail_tick: u32,
    elapsed_us: u32,
    restored: [Option<u32>; 2],
    hive: u32,
    body: u32,
) -> LiveHiveMouthEvent {
    let blocked = |block| LiveHiveMouthEvent::Blocked { hive, body, block };
    let Some(mut runtime) = entities
        .entity_mut(hive)
        .and_then(|entity| entity.authored_radial_emitter.as_mut())
        .and_then(|emitter| emitter.take_birth_runtime())
    else {
        return blocked(LiveHiveMouthBlock::NoEjectionSlots);
    };
    let model_extent = |id| resources.global_model(id).map(|model| model.radius);
    let mut host = HiveBirthManagerHost::new(
        entities,
        HiveBirthManagerContext {
            resources: EntityConstructionResources {
                terrain: resources.level_terrain(),
                terrain_objects: resources.terrain_objects(),
                model_extent_raw: Some(&model_extent),
            },
            retail_tick,
            // Only a birth reads the wave policy.
            waves_enabled: RetailRuntimeValue::Unresolved,
        },
        world_fx,
    );
    let outcome = runtime.eject_mouth_body(
        HiveEjectionRequest {
            source_id: hive,
            child_handle: body,
        },
        elapsed_us,
        restored,
        &mut host,
    );
    entities
        .entity_mut(hive)
        .and_then(|entity| entity.authored_radial_emitter.as_mut())
        .expect("taken above")
        .restore_birth_runtime(runtime);
    match outcome {
        Ok(outcome) => LiveHiveMouthEvent::Ejected {
            hive,
            body,
            outcome,
        },
        Err(block) => blocked(LiveHiveMouthBlock::Ejection(format!("{block:?}"))),
    }
}

#[cfg(test)]
mod tests;
