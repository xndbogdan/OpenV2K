//! Playing's source-order 14AE0 walk with player hull and native death custody.
//!
//! Native Base/Factory/worker callbacks publish their replacement tasks and RNG
//! before the next target. The player has the same radial admission/impulse
//! policy followed by synchronous checked damage and native death. Other
//! retained targets keep their one-allocation planner. A later block
//! preserves earlier completed targets rather than replaying their effects.

use super::{EntityManager, SpecializedActorTaskOwner, SpecializedActorTaskScheduler};
use crate::intro2_type8::NativeWorkerProfile;
use crate::native_type86::NativeFourChoiceProfile;
use crate::{
    entity::{
        DynamicRadialDeathPublication, DynamicRadialLiveBlock, DynamicRadialLiveCallbacks,
        DynamicRadialLiveOutcome, DynamicRadialLiveRequest, DynamicRadialUnresolved,
        DynamicRadialUnresolvedReason,
    },
    gameplay_notifications::GameplayNotifications,
    intro2_radial::Intro2RadialTaskCustody,
    player_hull::PlayerHull,
    radial_damage::RadialDamageTemplate,
    world_fx::WorldFx,
};

#[cfg(test)]
mod contact_prefix_tests;
#[cfg(test)]
mod fish_tests;
#[cfg(test)]
mod native_tests;
mod player;
#[cfg(test)]
mod player_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod type17_tests;
pub use player::{PlayerRadialBlock, PlayerRadialBlockReason};

pub struct PlayingRadialFrame<'a> {
    pub entities: &'a mut EntityManager,
    pub player_hull: &'a mut PlayerHull,
    pub extra_lives: crate::entity_collision_state::RetailRuntimeValue<u8>,
    pub origin_raw: [i16; 3],
    pub template: RadialDamageTemplate,
    pub world_fx: &'a mut WorldFx,
    pub notifications: &'a mut GameplayNotifications,
    pub retail_tick: u32,
    pub resources: &'a mut crate::resource_cache::ResourceCache,
    pub static_damage: &'a mut crate::static_damage::StaticDamageScheduler,
    pub active_terminal_calls: Vec<crate::intro2_radial::Intro2RadialTerminalCall>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayingRadialBlock {
    Native(DynamicRadialLiveBlock),
    Retained(DynamicRadialUnresolved),
    Player(PlayerRadialBlock),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlayingRadialOutcome {
    pub accepted_targets: usize,
    pub completed_target_ids: Vec<u32>,
    pub blocked: Option<PlayingRadialBlock>,
}

impl SpecializedActorTaskScheduler {
    pub fn apply_playing_radial_damage(
        &mut self,
        frame: PlayingRadialFrame<'_>,
    ) -> PlayingRadialOutcome {
        let PlayingRadialFrame {
            entities,
            player_hull,
            extra_lives,
            origin_raw,
            template,
            world_fx,
            notifications,
            retail_tick,
            resources,
            static_damage,
            active_terminal_calls,
        } = frame;
        let mut outcome = PlayingRadialOutcome::default();
        let mut current = entities.retail_live_order_ids().next();
        while let Some(id) = current {
            let entity = entities.iter_all().find(|entity| entity.id == id).unwrap();
            if entities.player().is_some_and(|player| player.id == id) {
                match player::apply_player_radial_target(
                    entities,
                    id,
                    player::PlayerRadialContext {
                        player_hull,
                        extra_lives,
                        origin_raw,
                        template,
                        world_fx,
                        notifications,
                        retail_tick,
                        resources,
                    },
                ) {
                    Ok(true) => {
                        outcome.accepted_targets += 1;
                        outcome.completed_target_ids.push(id);
                    }
                    Ok(false) => {}
                    Err(block) => {
                        outcome.accepted_targets += usize::from(block.accepted_target);
                        outcome.blocked = Some(PlayingRadialBlock::Player(block));
                        break;
                    }
                }
                current = entities
                    .retail_live_order_ids()
                    .skip_while(|candidate| *candidate != id)
                    .nth(1);
                continue;
            }
            let native = match entity.entity_type {
                6 => entity.main_base_runtime.is_some(),
                type_id if NativeWorkerProfile::from_entity_type(type_id).is_some() => {
                    entity.intro2_type8_runtime.is_some()
                }
                123 => entity.native_type123_runtime.is_some(),
                type_id if NativeFourChoiceProfile::from_entity_type(type_id).is_some() => {
                    entity.native_type86_runtime.is_some()
                }
                16 | 128 => entity.intro2_type16_runtime.is_some(),
                10 | 5 | 80 | 126 => entity.intro2_type10_runtime.is_some(),
                // A receipt selects the owner; its adapter authenticates it.
                13 => {
                    entity.native_type13_allocation.is_some()
                        || crate::class49_death::source_profile(entity).is_some()
                }
                17 => entity.intro2_type17_runtime.is_some(),
                47 => entity.native_type47_construction.is_some(),
                94 => entity.intro2_type94_runtime.is_some(),
                53 => entity.intro2_type53_runtime.is_some(),
                58 => entity.intro2_type58_runtime.is_some(),
                122 => entity.native_type122_runtime.is_some(),
                18 => entity.native_type18_runtime.is_some(),
                28 => entity.native_type28_runtime.is_some(),
                76 | 77 => entity.native_type76_runtime.is_some(),
                30 => entity.native_type30_runtime.is_some(),
                40 => entity.native_type40_runtime.is_some(),
                56 => entity.native_type56_runtime.is_some(),
                66 | 125 => entity.intro2_type66_runtime.is_some(),
                22 | 23 | 24 | 62 | 124 => entity.shared_fish_runtime.is_some(),
                3 | 27 => entity.rolling_boulder_runtime.is_some(),
                _ => crate::class49_death::source_profile(entity).is_some(),
            };
            if native {
                let mut native_outcome = DynamicRadialLiveOutcome::default();
                let mut callbacks = PlayingNativeCallbacks {
                    scheduler: self,
                    resources,
                    static_damage,
                    player_hull,
                    extra_lives,
                    active_terminal_calls: &active_terminal_calls,
                };
                let result = entities.apply_native_playing_radial_target(
                    id,
                    &mut DynamicRadialLiveRequest {
                        origin_raw,
                        template,
                        world_fx,
                        retail_tick,
                        notifications,
                        callbacks: &mut callbacks,
                    },
                    &mut native_outcome,
                );
                outcome.accepted_targets += native_outcome.accepted_targets;
                match result {
                    Ok(true) => outcome.completed_target_ids.push(id),
                    Ok(false) => {}
                    Err(block) => {
                        // A committed callback may already own a replacement
                        // graph. A later visit must not replay that prefix.
                        if block.target_prefix_committed {
                            callbacks.scheduler.park_intro2_type8_external_prefix(id);
                            callbacks.scheduler.park_intro2_type10_external_prefix(id);
                            callbacks.scheduler.park_native_type123_external_prefix(id);
                            callbacks.scheduler.park_native_type86_external_prefix(id);
                            callbacks.scheduler.park_intro2_type17_external_prefix(id);
                            callbacks.scheduler.park_native_type47_external_prefix(id);
                            callbacks
                                .scheduler
                                .park_cleansing_vehicle_external_prefix(id);
                            if entities.iter_all().any(|entity| {
                                entity.id == id
                                    && (entity.intro2_type16_runtime.is_some()
                                        || entity.native_type13_allocation.is_some()
                                        || entity.intro2_type94_runtime.is_some()
                                        || entity.intro2_type53_runtime.is_some()
                                        || entity.intro2_type58_runtime.is_some()
                                        || entity.native_type122_runtime.is_some()
                                        || entity.native_type18_runtime.is_some()
                                        || entity.native_type28_runtime.is_some()
                                        || entity.native_type76_runtime.is_some()
                                        || entity.native_type30_runtime.is_some()
                                        || entity.native_type40_runtime.is_some()
                                        || entity.native_type56_runtime.is_some()
                                        || entity.native_type43_runtime.is_some()
                                        || entity.native_type38_runtime.is_some()
                                        || entity.rolling_boulder_runtime.is_some()
                                        || entity.shared_fish_runtime.is_some())
                            }) {
                                callbacks.scheduler.park_native_contact_prefix(entities, id);
                            }
                        }
                        outcome.blocked = Some(PlayingRadialBlock::Native(block));
                        break;
                    }
                }
            } else {
                match self.apply_retained_radial_target(
                    entities,
                    player_hull,
                    origin_raw,
                    template,
                    id,
                ) {
                    Ok(applied) => {
                        outcome.accepted_targets += applied.accepted_targets;
                        if applied.accepted_targets != 0 {
                            outcome.completed_target_ids.push(id);
                        }
                        for sound in applied.sounds {
                            world_fx.queue_fixed_positional_sound_raw(
                                sound.sound_id,
                                sound.position_raw,
                            );
                        }
                    }
                    Err(block) => {
                        outcome.blocked = Some(PlayingRadialBlock::Retained(block));
                        break;
                    }
                }
            }
            // 14AE0 samples the successor after the callback. A deferred
            // corpse remains linked, and a newly appended tail is visible.
            current = entities
                .retail_live_order_ids()
                .skip_while(|candidate| *candidate != id)
                .nth(1);
        }
        outcome
    }

    fn apply_retained_radial_target(
        &mut self,
        entities: &mut EntityManager,
        player_hull: &mut PlayerHull,
        origin_raw: [i16; 3],
        template: RadialDamageTemplate,
        target_id: u32,
    ) -> Result<crate::entity::DynamicRadialApplied, DynamicRadialUnresolved> {
        let plan = entities.preflight_dynamic_radial_target(
            player_hull,
            origin_raw,
            template,
            target_id,
        )?;
        // A retained plan may change health/buffer at the blast center while
        // its impulse is exactly zero. Keep the external contact stop even
        // when the selected mover's velocity acknowledgment is unnecessary.
        if self.has_native_contact_prefix(target_id) && plan.changes_target(entities, target_id) {
            return Err(blocked(target_id));
        }
        for (id, velocity_after) in plan.changed_velocity_targets(entities) {
            let entity = entities.iter_all().find(|entity| entity.id == id).unwrap();
            if entity.entity_type != 9 || entity.ordinary_type9_selected_component_runtime.is_none()
            {
                continue;
            }
            let Some(index) = self.owners.iter().position(|owner| owner.entity_id() == id) else {
                return Err(blocked(id));
            };
            let lease = match &self.owners[index] {
                SpecializedActorTaskOwner::OrdinaryType9Wander(owner) => {
                    owner.completed_visit_lease(entities)
                }
                SpecializedActorTaskOwner::OrdinaryType9GoToJob(owner) => {
                    owner.completed_visit_lease(entities)
                }
                SpecializedActorTaskOwner::OrdinaryType9RunAway(owner) => {
                    owner.completed_visit_lease(entities)
                }
                SpecializedActorTaskOwner::OrdinaryType9AttractAttention(owner) => {
                    owner.completed_visit_lease(entities)
                }
                _ => None,
            };
            if lease.is_none() || lease != entities.ordinary_type9_selected_actor_lease(id) {
                return Err(blocked(id));
            }
            // This target-local plan is complete. Acknowledge only its exact
            // impending velocity; never tick/rebuild/reselect the task graph.
            let accepted = match &mut self.owners[index] {
                SpecializedActorTaskOwner::OrdinaryType9Wander(owner) => {
                    owner.acknowledge_planned_radial_velocity(entities, velocity_after)
                }
                SpecializedActorTaskOwner::OrdinaryType9GoToJob(owner) => {
                    owner.acknowledge_planned_radial_velocity(entities, velocity_after)
                }
                SpecializedActorTaskOwner::OrdinaryType9RunAway(owner) => {
                    owner.acknowledge_planned_radial_velocity(entities, velocity_after)
                }
                SpecializedActorTaskOwner::OrdinaryType9AttractAttention(owner) => {
                    owner.acknowledge_planned_radial_velocity(entities, velocity_after)
                }
                _ => unreachable!("preflight retains the selected owner"),
            };
            assert!(accepted, "preflighted radial owner remains authoritative");
        }
        Ok(entities.commit_preflighted_dynamic_radial_damage(Some(player_hull), plan))
    }
}

pub(crate) struct PlayingNativeCallbacks<'a> {
    pub(crate) scheduler: &'a mut SpecializedActorTaskScheduler,
    pub(crate) resources: &'a mut crate::resource_cache::ResourceCache,
    pub(crate) static_damage: &'a mut crate::static_damage::StaticDamageScheduler,
    pub(crate) player_hull: &'a mut PlayerHull,
    pub(crate) extra_lives: crate::entity_collision_state::RetailRuntimeValue<u8>,
    pub(crate) active_terminal_calls: &'a [crate::intro2_radial::Intro2RadialTerminalCall],
}
impl DynamicRadialLiveCallbacks for PlayingNativeCallbacks<'_> {
    fn active_terminal_call(&self, manager: &EntityManager, id: u32) -> bool {
        self.active_terminal_calls
            .iter()
            .any(|call| call.authenticates(manager, id))
    }
    fn standard_death(
        &mut self,
        entities: &mut EntityManager,
        id: u32,
        kind: u32,
        world_fx: &mut WorldFx,
        retail_tick: u32,
        notifications: &mut GameplayNotifications,
    ) -> Option<
        Result<
            crate::live_actor_checked_damage::LiveActorDeathResult<DynamicRadialDeathPublication>,
            crate::entity::DynamicRadialLiveBlockReason,
        >,
    > {
        if entities
            .iter_all()
            .any(|entity| entity.id == id && entity.shared_fish_runtime.is_some() && kind == 124)
        {
            return Some(
                crate::shared_fish::death::run_type124_auto_pilot_death(
                    crate::class49_terminal::Class49TerminalFrame {
                        entities,
                        resources: self.resources,
                        world_fx,
                        static_damage: self.static_damage,
                        notifications,
                        retail_tick,
                        world: crate::class49_terminal::Class49WorldContext::Playing {
                            scheduler: self.scheduler,
                            player_hull: self.player_hull,
                            extra_lives: self.extra_lives,
                            active_terminal_calls: self.active_terminal_calls.to_vec(),
                        },
                    },
                    id,
                )
                .map(
                    |result| crate::live_actor_checked_damage::LiveActorDeathResult {
                        returned_nonzero: result.returned_nonzero,
                        publication: None,
                    },
                )
                .map_err(crate::entity::DynamicRadialLiveBlockReason::Fish),
            );
        }
        if entities
            .iter_all()
            .any(|entity| entity.id == id && entity.shared_fish_runtime.is_some())
        {
            return Some(
                crate::shared_fish::death::begin_shared_fish_standard_death(
                    entities,
                    id,
                    world_fx,
                    self.scheduler,
                )
                .map(|result| {
                    if result.publication.is_some() {
                        self.scheduler.retire_shared_fish(id);
                    }
                    crate::live_actor_checked_damage::LiveActorDeathResult {
                        returned_nonzero: result.returned_nonzero,
                        publication: None,
                    }
                })
                .map_err(crate::entity::DynamicRadialLiveBlockReason::Fish),
            );
        }
        if kind == 27
            && crate::rolling_boulder::rolling_boulder_manager_allocation_authenticates(
                entities, id,
            )
        {
            return Some(
                crate::rolling_boulder::death::begin_rolling_boulder_split(
                    entities,
                    id,
                    crate::rolling_boulder::death::RollingBoulderSplitFrame {
                        resources: self.resources,
                        world_fx,
                        retail_tick,
                        tasks: self.scheduler,
                    },
                )
                .map(
                    |result| crate::live_actor_checked_damage::LiveActorDeathResult {
                        returned_nonzero: result.returned_nonzero,
                        publication: result.publication.map(|terminal| {
                            match terminal {
                        crate::native_ground_actor::NativeGroundTerminalPublication::CommonDying(
                            owner,
                        ) => crate::entity::DynamicRadialDeathPublication::Intro2Class12(owner),
                        crate::native_ground_actor::NativeGroundTerminalPublication::Deferred(
                            receipt,
                        ) => crate::entity::DynamicRadialDeathPublication::NativeGroundDeferred(
                            receipt,
                        ),
                    }
                        }),
                    },
                )
                .map_err(crate::entity::DynamicRadialLiveBlockReason::RollingBoulder),
            );
        }
        if kind == 40 && crate::native_type40::manager_allocation_authenticates(entities, id) {
            return Some(crate::native_type40::death::begin_type40_standard_death(
                entities, id, crate::native_type40::death::Type40Class18Frame {
                    resources: self.resources, world_fx, retail_tick, tasks: self.scheduler,
                },
            ).map(|result| crate::live_actor_checked_damage::LiveActorDeathResult {
                returned_nonzero: result.returned_nonzero,
                publication: result.publication.map(|terminal| match terminal {
                    crate::native_ground_actor::NativeGroundTerminalPublication::CommonDying(owner) =>
                        crate::entity::DynamicRadialDeathPublication::Intro2Class12(owner),
                    crate::native_ground_actor::NativeGroundTerminalPublication::Deferred(receipt) =>
                        crate::entity::DynamicRadialDeathPublication::NativeGroundDeferred(receipt),
                }),
            }).map_err(crate::entity::DynamicRadialLiveBlockReason::NativeType40));
        }
        if kind != 49
            && !entities.iter_all().any(|entity| {
                entity.id == id && crate::class49_death::source_profile(entity).is_some()
            })
        {
            return None;
        }
        Some(
            crate::class49_terminal::run_class49_standard_death(
                crate::class49_terminal::Class49TerminalFrame {
                    entities,
                    resources: self.resources,
                    world_fx,
                    static_damage: self.static_damage,
                    notifications,
                    retail_tick,
                    world: crate::class49_terminal::Class49WorldContext::Playing {
                        scheduler: self.scheduler,
                        player_hull: self.player_hull,
                        extra_lives: self.extra_lives,
                        active_terminal_calls: self.active_terminal_calls.to_vec(),
                    },
                },
                id,
            )
            .map(
                |result| crate::live_actor_checked_damage::LiveActorDeathResult {
                    returned_nonzero: result.returned_nonzero,
                    publication: None,
                },
            )
            .map_err(|error| crate::entity::DynamicRadialLiveBlockReason::Class49(Box::new(error))),
        )
    }
    fn capture_task_custody(
        &mut self,
    ) -> Option<&mut dyn crate::intro2_type17::capture::CaptureTaskCustody> {
        Some(self.scheduler)
    }
    fn before_native_actor_mutation(&mut self, manager: &EntityManager, id: u32) -> bool {
        if manager
            .iter_all()
            .any(|entity| entity.id == id && entity.shared_fish_runtime.is_some())
        {
            return self.scheduler.shared_fish_completed_owner(manager, id)
                && !self.scheduler.shared_fish_has_pending_prefix(id);
        }
        self.scheduler.prepare_native_actor_mutation(manager, id)
    }
    fn retain_death_publication(&mut self, publication: DynamicRadialDeathPublication) {
        self.scheduler
            .register_radial_death_publications(&[publication]);
    }
}

fn blocked(target_id: u32) -> DynamicRadialUnresolved {
    DynamicRadialUnresolved {
        target_id,
        reason: DynamicRadialUnresolvedReason::SelectedType9MutationCustody,
    }
}
