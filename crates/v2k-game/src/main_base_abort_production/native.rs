//! Receipt-backed ordinary-world callbacks within the synchronous 170A0 walk.
//!
//! A native allocation uses the same 10C10 publisher as checked/radial death.
//! Captured Level-1 task/pose bridges remain a separate admission path. The
//! caller owns the speculative manager/scheduler transaction: any callback or
//! custody block rolls back the complete abort, including RNG and sound.

use super::*;
use crate::actor_task_owner::ActorTaskSlot;
use crate::intro2_radial::Intro2RadialTaskCustody;
use crate::intro2_type8::NativeWorkerProfile;
use crate::native_type86::NativeFourChoiceProfile;
use crate::ordinary_type9_standard_death::{
    OrdinaryType9StandardDeathBlock, OrdinaryType9StandardDeathEntry,
    OrdinaryType9StandardDeathOutcome,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeMainBaseAbortDeathBlock {
    ActorLeaseMismatch,
    TaskCustodyUnavailable,
    Type8(crate::intro2_type8::Intro2Type8Block),
    Type123(crate::native_type123::Type123Block),
    Type86(crate::native_type86::Type86Block),
    Type9(OrdinaryType9StandardDeathBlock),
    Type9TaskPublicationUnavailable,
    Type17(crate::intro2_common_dying::Intro2CommonDyingBlock),
    Type26(crate::intro2_common_dying::Intro2CommonDyingBlock),
    Type17Capture(crate::intro2_type17::capture::CaptureBlock),
    Type47(crate::intro2_common_dying::Intro2CommonDyingBlock),
    Type53(crate::intro2_common_dying::Intro2CommonDyingBlock),
    Type58(crate::intro2_common_dying::Intro2CommonDyingBlock),
    Type122(crate::native_actor_capture::CaptureBlock),
    Type66(crate::intro2_type66::death::Intro2Type66DeathBlock),
    MainBase(crate::main_base_runtime::MainBaseDeathBlock),
    Fish(crate::shared_fish::death::SharedFishDeathBlock),
    GunTurret(crate::class49_terminal::Class49TerminalBlock),
    Type10(crate::intro2_type10::death::Intro2Type10DeathBlock),
}

pub(super) fn dispatch_native_class49(
    actor: MainBaseAbortActorObservation,
    effects: &mut MainBaseAbortWorldEffects<'_>,
    frame: crate::main_base_abort_world_effects::MainBaseAbortClass49Frame<'_>,
    publications: &mut MainBaseAbortPublicationCounts,
) -> Option<Result<DispatchSuccess, DispatchFailure>> {
    let entity = frame
        .entities
        .iter_all()
        .find(|entity| entity.id == actor.lease.entity_id)?;
    // Native BAC0/BD20/BC90 terminal families share BAF0's nested radial owner,
    // with explicit class1/class49/class63 suffix policies and distinct allocations.
    let admitted = (matches!(entity.entity_type, 97 | 104 | 115)
        && entity.intro2_gun_turret_runtime.is_some())
        || (entity.entity_type == 49 && entity.cleansing_vehicle_runtime.is_some())
        || (entity.entity_type == 61 && crate::native_type61::has_native_allocation(entity))
        // Ordinary Type13's 10C10 -> DB80 enters alternate class1, BAC0.
        || (entity.entity_type == 13 && entity.native_type13_allocation.is_some())
        // So does the emitter-only Type43's.
        || (entity.entity_type == crate::native_type43::ENTITY_TYPE
            && entity.native_type43_runtime.is_some())
        // Type124 and the Type80/126 carriers enter class63, BC90: BAF0 then a
        // tail-appended Type61.
        || (entity.entity_type == 124 && entity.shared_fish_runtime.is_some())
        || crate::intro2_type10::type10_auto_pilot_profile(entity).is_some()
        || crate::intro2_type16::type16_auto_pilot_row(entity).is_some();
    if !admitted {
        return None;
    }
    Some((|| {
        let id = actor.lease.entity_id;
        if frame
            .entities
            .main_base_abort_actor_observation(id)
            .is_none_or(|current| current.lease != actor.lease)
            || !crate::class49_death::allocation_authenticates(frame.entities, id)
        {
            return Err(block(NativeMainBaseAbortDeathBlock::ActorLeaseMismatch));
        }
        let entity = frame
            .entities
            .iter_all()
            .find(|entity| entity.id == id)
            .unwrap();
        let no_op = matches!(entity.collision.state_flags_at_0x08.masked(REMOTE_OWNED_STATE_BIT | DYING_STATE_BIT), RetailRuntimeValue::Known(bits) if bits != 0);
        let completed = if no_op {
            true
        } else if entity.entity_type == 61 {
            // Class23's initializer publishes no task. Its native receipt
            // proves that absence; a fabricated or executing wrapper blocks.
            ActorTaskSlot::IN_RETAIL_TICK_ORDER
                .into_iter()
                .all(|slot| entity.actor_tasks.task_in_slot(slot).is_none())
        } else {
            frame
                .scheduler
                .prepare_native_actor_mutation(frame.entities, id)
        };
        if !completed {
            return Err(block(NativeMainBaseAbortDeathBlock::TaskCustodyUnavailable));
        }
        let count = |entities: &crate::entity::EntityManager, entity_type| {
            entities
                .iter_all()
                .filter(|entity| entity.entity_type == entity_type)
                .count()
        };
        let rings_before = count(frame.entities, 60);
        let power_ups_before = count(frame.entities, 61);
        let entities = &mut *frame.entities;
        effects
            .run_class49_standard_death(
                id,
                crate::main_base_abort_world_effects::MainBaseAbortClass49Frame {
                    entities,
                    world_fx: frame.world_fx,
                    notifications: frame.notifications,
                    retail_tick: frame.retail_tick,
                    scheduler: frame.scheduler,
                    player_hull: frame.player_hull,
                    extra_lives: frame.extra_lives,
                },
            )
            .map_err(|error| block(NativeMainBaseAbortDeathBlock::GunTurret(error)))?;
        publications.appended_type60_actors += count(entities, 60).saturating_sub(rings_before);
        publications.appended_type61_actors += count(entities, 61).saturating_sub(power_ups_before);
        let (successor, successor_available) =
            match entities.main_base_abort_successor_after_callback(id) {
                Ok(next) => (next, true),
                Err(()) => (None, false),
            };
        Ok(DispatchSuccess {
            disposition: MainBaseAbortActorDisposition::Class49Death,
            successor,
            successor_available,
        })
    })())
}

#[derive(Clone, Copy)]
enum NativeActor {
    MainBase,
    Type10Family,
    Worker,
    Type123Person,
    FourChoice,
    Peasant,
    CapturingInsect,
    Type26,
    ShootingInsect,
    Type53,
    Type58,
    Type122,
    Factory,
    SharedFish,
}

pub(super) fn dispatch_native_actor(
    actor: MainBaseAbortActorObservation,
    entities: &mut EntityManager,
    world_fx: &mut WorldFx,
    specialized_tasks: &mut SpecializedActorTaskScheduler,
    publications: &mut MainBaseAbortPublicationCounts,
    gameplay: &mut MainBaseAbortGameplayContext<'_>,
) -> Option<Result<DispatchSuccess, DispatchFailure>> {
    let entity = entities
        .iter_all()
        .find(|entity| entity.id == actor.lease.entity_id)?;
    // Presence selects the native policy; authentication occurs inside its
    // publisher. A foreign/stale receipt must never fall through to a captured
    // predecessor merely because its public type/spawn happens to match.
    let kind = match entity.entity_type {
        type_id
            if NativeWorkerProfile::from_entity_type(type_id).is_some()
                && entity.intro2_type8_runtime.is_some() =>
        {
            NativeActor::Worker
        }
        123 if entity.native_type123_runtime.is_some() => NativeActor::Type123Person,
        type_id
            if NativeFourChoiceProfile::from_entity_type(type_id).is_some()
                && entity.native_type86_runtime.is_some() =>
        {
            NativeActor::FourChoice
        }
        9 if entity.ordinary_type9_native_receipt.is_some() => NativeActor::Peasant,
        17 if entity.intro2_type17_runtime.is_some() => NativeActor::CapturingInsect,
        26 if entity.native_type26_allocation.is_some() => NativeActor::Type26,
        47 if entity.native_type47_construction.is_some() => NativeActor::ShootingInsect,
        53 if entity.intro2_type53_runtime.is_some() => NativeActor::Type53,
        58 if entity.intro2_type58_runtime.is_some() => NativeActor::Type58,
        122 if entity.native_type122_runtime.is_some() => NativeActor::Type122,
        6 if entity.main_base_runtime.is_some() => NativeActor::MainBase,
        66 if entity.intro2_type66_runtime.is_some() => NativeActor::Factory,
        type_id
            if crate::intro2_type10::Type10Profile::from_entity_type(type_id)
                .is_some_and(|profile| profile.alternate_behavior_class() == 11)
                && entity
                    .intro2_type10_runtime
                    .is_some_and(|runtime| runtime.ordinary_allocation.is_some()) =>
        {
            NativeActor::Type10Family
        }
        22 | 23 | 24 | 62 if entity.shared_fish_runtime.is_some() => NativeActor::SharedFish,
        _ => return None,
    };
    Some(dispatch(
        kind,
        actor,
        entities,
        world_fx,
        specialized_tasks,
        publications,
        gameplay,
    ))
}

fn dispatch(
    kind: NativeActor,
    actor: MainBaseAbortActorObservation,
    entities: &mut EntityManager,
    world_fx: &mut WorldFx,
    specialized_tasks: &mut SpecializedActorTaskScheduler,
    publications: &mut MainBaseAbortPublicationCounts,
    gameplay: &mut MainBaseAbortGameplayContext<'_>,
) -> Result<DispatchSuccess, DispatchFailure> {
    use NativeMainBaseAbortDeathBlock as Block;
    if entities
        .main_base_abort_actor_observation(actor.lease.entity_id)
        .is_none_or(|current| current.lease != actor.lease)
    {
        return Err(block(Block::ActorLeaseMismatch));
    }
    let id = actor.lease.entity_id;
    if matches!(kind, NativeActor::CapturingInsect)
        && !crate::intro2_type17::type17_manager_allocation_authenticates(entities, id)
    {
        return Err(block(Block::Type17(
            crate::intro2_common_dying::Intro2CommonDyingBlock::UnauthenticatedAllocation,
        )));
    }
    let entity = entities.iter_all().find(|entity| entity.id == id).unwrap();
    if matches!(kind, NativeActor::ShootingInsect)
        && !crate::shared_type47::type47_manager_allocation_authenticates(entities, id)
    {
        return Err(block(Block::Type47(
            crate::intro2_common_dying::Intro2CommonDyingBlock::UnauthenticatedAllocation,
        )));
    }
    if matches!(kind, NativeActor::Type53)
        && !crate::intro2_type53::type53_manager_allocation_authenticates(entities, id)
    {
        return Err(block(Block::Type53(
            crate::intro2_common_dying::Intro2CommonDyingBlock::UnauthenticatedAllocation,
        )));
    }
    if matches!(kind, NativeActor::Type10Family)
        && !crate::intro2_type10::type10_manager_allocation_authenticates(entities, id)
    {
        return Err(block(Block::Type10(
            crate::intro2_type10::death::Intro2Type10DeathBlock::Allocation,
        )));
    }
    if matches!(kind, NativeActor::Type58)
        && !crate::intro2_type58::type58_manager_allocation_authenticates(entities, id)
    {
        return Err(block(Block::Type58(
            crate::intro2_common_dying::Intro2CommonDyingBlock::UnauthenticatedAllocation,
        )));
    }
    let generic_noop = match entity
        .collision
        .state_flags_at_0x08
        .masked(REMOTE_OWNED_STATE_BIT)
    {
        RetailRuntimeValue::Known(value) if value != 0 => true,
        RetailRuntimeValue::Known(0) => matches!(
            entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT),
            RetailRuntimeValue::Known(value) if value != 0
        ),
        RetailRuntimeValue::Known(_) | RetailRuntimeValue::Unresolved => false,
    };
    // Remote/already-dying 10C10 returns before task mutation. In particular,
    // revisiting a class-14 corpse preserves its elapsed time and RNG stream.
    if !generic_noop {
        let ready = match kind {
            NativeActor::MainBase => {
                crate::actor_task_owner::ActorTaskSlot::IN_RETAIL_TICK_ORDER
                    .into_iter()
                    .filter_map(|slot| entity.actor_tasks.task_in_slot(slot))
                    .all(|task| {
                        entity
                            .actor_tasks
                            .wrapper_flags(task)
                            .is_some_and(|flags| flags.alive && !flags.in_callback)
                    })
                    && specialized_tasks.prepare_native_actor_mutation(entities, id)
            }
            NativeActor::Peasant => {
                // This read-only lease proves the four selected Type9 graphs
                // have no suspended task/root/outer phase. It is the same
                // boundary required by other synchronous graph replacements.
                specialized_tasks.prepare_type9_cargo_attach(entities, id) == Some(actor.lease)
            }
            NativeActor::Worker => {
                specialized_tasks.begin_intro2_type8_external_mutation(entities, id)
            }
            NativeActor::Type123Person => {
                specialized_tasks.begin_native_type123_external_mutation(entities, id)
            }
            NativeActor::FourChoice => {
                specialized_tasks.begin_native_type86_external_mutation(entities, id)
            }
            NativeActor::SharedFish => {
                specialized_tasks.shared_fish_completed_owner(entities, id)
                    && !specialized_tasks.shared_fish_has_pending_prefix(id)
            }
            NativeActor::CapturingInsect
            | NativeActor::Type26
            | NativeActor::ShootingInsect
            | NativeActor::Type53
            | NativeActor::Type58
            | NativeActor::Type122
            | NativeActor::Type10Family
            | NativeActor::Factory => specialized_tasks.prepare_native_actor_mutation(entities, id),
        };
        if !ready {
            return Err(block(Block::TaskCustodyUnavailable));
        }
    }
    let disposition = match kind {
        NativeActor::Type10Family => {
            // 10C10 -> DB80's null living hook -> AC60's alternate class11:
            // C660 publishes the falling Tumble; C750 later owns its blast.
            let owner = crate::intro2_type10::death::publish_intro2_type10_standard_death(
                entities, id, world_fx,
            )
            .map_err(|error| block(Block::Type10(error)))?;
            if let Some(owner) = owner {
                specialized_tasks.register_intro2_type10_tumble(owner);
                publications.type10_tumble += 1;
            }
            MainBaseAbortActorDisposition::Type10Death
        }
        NativeActor::Type26 => {
            // DB80's null living death hook enters C620/class12, exactly as
            // Type26's existing checked damage and E370 expiry publisher.
            let owner = crate::intro2_common_dying::publish_intro2_common_standard_death(
                entities, id, world_fx,
            )
            .map_err(|error| block(Block::Type26(error)))?;
            if let Some(owner) = owner {
                specialized_tasks.register_intro2_common_dying(owner);
                publications.type26_common_dying += 1;
            }
            MainBaseAbortActorDisposition::Type26Death
        }
        NativeActor::MainBase => {
            let death =
                crate::main_base_runtime::publish_main_base_standard_death(entities, id, world_fx)
                    .map_err(|error| block(Block::MainBase(error)))?;
            if let Some(owner) = death.owner {
                specialized_tasks.register_main_base(owner);
                publications.main_base_production += 1;
            }
            MainBaseAbortActorDisposition::MainBaseDeath
        }
        NativeActor::SharedFish => {
            // These B/D/F swimmers can move away from the authored anchor.
            // Use their live allocation/task custody, not a birth-pose receipt.
            let death = crate::shared_fish::death::begin_shared_fish_standard_death(
                entities,
                id,
                world_fx,
                specialized_tasks,
            )
            .map_err(|error| block(Block::Fish(error)))?;
            if death.publication.is_some() {
                specialized_tasks.retire_shared_fish(id);
            }
            MainBaseAbortActorDisposition::QuietDeath
        }
        NativeActor::Type122 => {
            let owner = crate::native_actor_capture::publish_native_captor_standard_death(
                entities,
                id,
                &mut crate::native_actor_capture::CaptureContext {
                    tasks: specialized_tasks,
                    world_fx,
                    notifications: gameplay.notifications,
                    retail_tick: gameplay.retail_tick,
                    result_screen: MainBaseType9ResultScreenState::AlreadyShownByMainBaseAbort,
                    hive_dying: Default::default(),
                },
            )
            .map_err(|error| block(Block::Type122(error)))?;
            if let Some(owner) = owner {
                specialized_tasks.register_intro2_common_dying(owner);
                publications.type122_common_dying += 1;
            }
            MainBaseAbortActorDisposition::Type122Death
        }
        NativeActor::Worker => {
            let death = crate::intro2_type8::impact::run_intro2_type8_standard_death(
                entities, id, world_fx,
            )
            .map_err(|error| block(Block::Type8(error)))?;
            if let Some(owner) = death.publication {
                specialized_tasks.register_intro2_type8(owner);
                publications.type8_exploding += 1;
                publications.same_family_replacements += 1;
            }
            MainBaseAbortActorDisposition::Type8Death
        }
        NativeActor::Type123Person => {
            let death = crate::native_type123::impact::run_native_type123_standard_death(
                entities, id, world_fx,
            )
            .map_err(|error| block(Block::Type123(error)))?;
            if let Some(owner) = death.publication {
                specialized_tasks.register_native_type123(owner);
                publications.type123_exploding += 1;
                publications.same_family_replacements += 1;
            }
            MainBaseAbortActorDisposition::Type123Death
        }
        NativeActor::FourChoice => {
            let death = crate::native_type86::impact::run_native_type86_standard_death(
                entities, id, world_fx,
            )
            .map_err(|error| block(Block::Type86(error)))?;
            if let Some(owner) = death.publication {
                specialized_tasks.register_native_type86(owner);
                publications.type86_exploding += 1;
                publications.same_family_replacements += 1;
            }
            MainBaseAbortActorDisposition::Type86Death
        }
        NativeActor::Peasant => {
            let death = entities
                .publish_ordinary_type9_standard_death(
                    id,
                    OrdinaryType9StandardDeathEntry::GenericDeath,
                    MainBaseType9ResultScreenState::AlreadyShownByMainBaseAbort,
                    world_fx,
                    // The abort session byte suppresses C6. No timestamp consumer
                    // executes, and this route owns no notification queue.
                    0,
                    None,
                )
                .map_err(|error| block(Block::Type9(error)))?;
            if matches!(
                death,
                OrdinaryType9StandardDeathOutcome::Published { .. }
                    | OrdinaryType9StandardDeathOutcome::InitializerFallback { .. }
            ) {
                let lease =
                    crate::ordinary_type9_outer_tail::outer_tail_class14_exploding_task_lease(
                        entities,
                        actor.lease,
                    )
                    .ok_or_else(|| block(Block::Type9TaskPublicationUnavailable))?;
                match specialized_tasks.adopt_ordinary_type9_class14_after_checked_death(lease) {
                    Ok(displaced) => {
                        publications.type9_exploding += 1;
                        publications.same_family_replacements += usize::from(displaced.is_some());
                    }
                    Err(failure) => {
                        return Err(conflicting_owner(
                            failure.conflict,
                            MainBaseAbortUnscheduledOwner::OrdinaryType9Class14(failure.rejected),
                        ))
                    }
                }
            }
            MainBaseAbortActorDisposition::Type9Death
        }
        NativeActor::CapturingInsect => {
            // 170A0 invokes the same 10C10 -> DB80 -> AC60/class12 -> C620
            // chain as an ordinary checked death. The completed living owner
            // is replaced only after this source-ordered publication succeeds.
            let owner = crate::intro2_type17::capture::publish_type17_standard_death(
                entities,
                id,
                &mut crate::intro2_type17::capture::CaptureContext {
                    tasks: specialized_tasks,
                    world_fx,
                    notifications: gameplay.notifications,
                    retail_tick: gameplay.retail_tick,
                    result_screen: MainBaseType9ResultScreenState::AlreadyShownByMainBaseAbort,
                    hive_dying: Default::default(),
                },
            )
            .map_err(|error| block(Block::Type17Capture(error)))?;
            if let Some(owner) = owner {
                specialized_tasks.register_intro2_common_dying(owner);
                publications.type17_common_dying += 1;
            }
            MainBaseAbortActorDisposition::Type17Death
        }
        NativeActor::Factory => {
            let death = crate::intro2_type66::death::publish_intro2_type66_standard_death(
                entities, id, world_fx,
            )
            .map_err(|error| block(Block::Type66(error)))?;
            if let Some(owner) = death.owner {
                specialized_tasks.register_intro2_type66(owner);
                publications.type66_production += 1;
                publications.same_family_replacements += 1;
            }
            MainBaseAbortActorDisposition::Type66Death
        }
        NativeActor::ShootingInsect => {
            let owner = crate::intro2_common_dying::publish_intro2_common_standard_death(
                entities, id, world_fx,
            )
            .map_err(|error| block(Block::Type47(error)))?;
            if let Some(owner) = owner {
                specialized_tasks.register_intro2_common_dying(owner);
                publications.type47_common_dying += 1;
            }
            MainBaseAbortActorDisposition::Type47Death
        }
        NativeActor::Type53 => {
            // Same 10C10 -> DB80 -> AC60/class12 -> C620 chain as the shared
            // Type47 owner: Type53 lethal hits already publish this exact
            // class12 owner, so the abort reuses it with Type53 custody.
            let owner = crate::intro2_common_dying::publish_intro2_common_standard_death(
                entities, id, world_fx,
            )
            .map_err(|error| block(Block::Type53(error)))?;
            if let Some(owner) = owner {
                specialized_tasks.register_intro2_common_dying(owner);
                publications.type53_common_dying += 1;
            }
            MainBaseAbortActorDisposition::Type53Death
        }
        NativeActor::Type58 => {
            // Type58's null death hook enters the same native10C10/C620
            // common12 publisher used by its checked lethal damage.
            let owner = crate::intro2_common_dying::publish_intro2_common_standard_death(
                entities, id, world_fx,
            )
            .map_err(|error| block(Block::Type58(error)))?;
            if let Some(owner) = owner {
                specialized_tasks.register_intro2_common_dying(owner);
                publications.type58_common_dying += 1;
            }
            MainBaseAbortActorDisposition::Type58Death
        }
    };
    // Sample only after all synchronous callback writes and publications.
    // The manager retains dying actors until the later deferred-list splice.
    let (successor, successor_available) =
        match entities.main_base_abort_successor_after_callback(id) {
            Ok(next) => (next, true),
            Err(()) => (None, false),
        };
    Ok(DispatchSuccess {
        disposition,
        successor,
        successor_available,
    })
}

fn block(error: NativeMainBaseAbortDeathBlock) -> DispatchFailure {
    callback_error(MainBaseAbortActorCallbackBlock::Native(error))
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod type122_tests;
#[cfg(test)]
mod type62_tests;

#[cfg(test)]
mod main_base_tests;

#[cfg(test)]
mod type26_tests;

#[cfg(test)]
mod carrier_tests;
#[cfg(test)]
mod ring_custody_tests;
#[cfg(test)]
mod type124_tests;
#[cfg(test)]
mod type128_tests;
#[cfg(test)]
mod type13_tests;
#[cfg(test)]
mod type43_tests;
#[cfg(test)]
mod type5_tests;
#[cfg(test)]
mod type61_tests;
#[cfg(test)]
mod type7_tests;
#[cfg(test)]
mod type97_tests;
#[cfg(test)]
mod worker79_91_tests;
