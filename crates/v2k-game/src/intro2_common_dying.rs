//! Native Intro2 class-12 publication and complete normal/coarse lifecycle.
//!
//! Types 16, 17, 26, 47, 53, 58 and 94 retain their authored component topology, model,
//! Sub-A speed and default state words through the shared class-12 program.
//! Class 12 passes a null target to 01430, so no Sub-D seed or query is read.

mod mover;
mod surface;
#[cfg(test)]
mod type16_tests;
#[cfg(test)]
mod type17_tests;
#[cfg(test)]
mod type26_tests;
#[cfg(test)]
mod type58_tests;
mod world;
pub(crate) use surface::{
    run_actor_surface_with_death, run_actor_surface_with_lifecycle, run_living_actor_surface,
    Intro2ActorSurfaceFrame,
};
pub(crate) use world::run_surface as run_dying_actor_surface;
pub use world::{
    tick_intro2_common_dying, Intro2CommonDyingFrame, Intro2CommonDyingOutcome,
    Intro2CommonDyingTick,
};

use crate::actor_death::COMMON_ACTOR_DYING_ACTIVE_STYLE;
use crate::actor_task_dispatcher::ActorTaskRuntime;
use crate::actor_task_owner::{ActorTaskSlot, ActorTaskVisit};
use crate::common_dying::{
    plan_common_dying_setup, CommonDyingComponentDescriptors, CommonDyingConstructorEffect,
    CommonDyingTaskState,
};
use crate::entity::{Entity, EntityManager};
use crate::entity_behavior::{
    audited_behavior_program, initial_behavior_state_policy, DeathCallbackPolicy,
};
use crate::entity_collision_state::{
    EntityTypeRuntimeMetadata, RetailRuntimeValue, DYING_STATE_BIT, REMOTE_OWNED_STATE_BIT,
};
use crate::main_base_abort::MainBaseAbortActorLease;
use crate::world_fx::WorldFx;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2CommonDyingBlock {
    /// A class63 carrier's BAF0/BC90 terminal blocked after its prefix.
    AutoPilot(Box<crate::class49_terminal::Class49TerminalBlock>),
    AllocationUnavailable,
    UnauthenticatedAllocation,
    Metadata(&'static str),
    Runtime(&'static str),
    TaskUnavailable,
    UnsupportedDeathHook(DeathCallbackPolicy),
    UnsupportedReleaseHook(crate::entity_behavior::ReleaseCallbackPolicy),
    Constructor(crate::common_dying::CommonDyingConstructorError),
    Mover(crate::common_mover::frame_machine::CommonMoverFrameBlock),
    MoverAdvance(crate::common_mover::frame_machine::CommonMoverFrameAdvanceError),
    MoverAction(crate::common_mover::frame_machine::CommonMoverFrameAction),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2CommonDyingOwner {
    allocation: MainBaseAbortActorLease,
    visit: ActorTaskVisit,
    /// A failed callback has committed its scheduler prefix. It cannot become
    /// a fresh frame and replay the callback or its shared RNG.
    pending: bool,
}

impl Intro2CommonDyingOwner {
    pub(crate) const fn actor_lease(self) -> MainBaseAbortActorLease {
        self.allocation
    }
    pub const fn entity_id(self) -> u32 {
        self.allocation.entity_id
    }
    pub(crate) const fn has_pending_prefix(self) -> bool {
        self.pending
    }
    pub(crate) fn park_external_prefix(&mut self) {
        self.pending = true;
    }
    pub fn adopt(manager: &EntityManager, entity_id: u32) -> Result<Self, Intro2CommonDyingBlock> {
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .ok_or(Intro2CommonDyingBlock::AllocationUnavailable)?;
        if !native_manager_allocation(manager, entity_id) {
            return Err(Intro2CommonDyingBlock::UnauthenticatedAllocation);
        }
        let visit = current_visit(entity)?;
        let allocation = manager
            .main_base_abort_actor_observation(entity_id)
            .ok_or(Intro2CommonDyingBlock::AllocationUnavailable)?
            .lease;
        Ok(Self {
            allocation,
            visit,
            pending: false,
        })
    }
    pub(crate) const fn fork_for_main_base_abort_transaction(&self) -> Self {
        *self
    }
}

fn native_allocation(entity: &Entity) -> bool {
    crate::native_type30::allocation_authenticates(entity)
        || crate::native_type122::type122_allocation_authenticates(entity)
        || crate::intro2_type16::intro2_type16_allocation_authenticates(entity)
        || crate::intro2_type58::intro2_type58_allocation_authenticates(entity)
        || crate::intro2_type94::intro2_type94_allocation_authenticates(entity)
        || crate::intro2_type17::intro2_type17_allocation_authenticates(entity)
        || crate::intro2_type47_live::intro2_type47_cohort_runtime_authenticates(entity)
        || crate::intro2_type53::intro2_type53_allocation_authenticates(entity)
        || crate::intro2_type26_defecate_virus::intro2_type26_allocation_authenticates(entity)
}

fn native_manager_allocation(manager: &EntityManager, entity_id: u32) -> bool {
    let Some(entity) = manager.iter_all().find(|entity| entity.id == entity_id) else {
        return false;
    };
    if entity.intro2_type17_runtime.is_some() {
        // The receipt carries either an authenticated native allocation or an
        // explicit retained Intro2 fixture origin. A foreign native receipt
        // cannot fall through to another family's entity-only admission.
        return crate::intro2_type17::type17_manager_allocation_authenticates(manager, entity_id);
    }
    if entity.native_type47_construction.is_some() {
        return crate::shared_type47::type47_manager_allocation_authenticates(manager, entity_id);
    }
    if entity.intro2_type53_runtime.is_some() {
        return crate::intro2_type53::type53_manager_allocation_authenticates(manager, entity_id);
    }
    if entity.intro2_type58_runtime.is_some() {
        return crate::intro2_type58::type58_manager_allocation_authenticates(manager, entity_id);
    }
    if entity.native_type30_runtime.is_some() {
        return crate::native_type30::manager_allocation_authenticates(manager, entity_id);
    }
    if entity.native_type122_runtime.is_some() {
        return crate::native_type122::type122_manager_allocation_authenticates(manager, entity_id);
    }
    if entity.native_type26_allocation.is_some() {
        return crate::intro2_type26_defecate_virus::type26_manager_allocation_authenticates(
            manager, entity_id,
        );
    }
    native_allocation(entity)
}

fn current_visit(entity: &Entity) -> Result<ActorTaskVisit, Intro2CommonDyingBlock> {
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(Intro2CommonDyingBlock::Runtime("current behavior"));
    };
    if context.active_style().style_address() != COMMON_ACTOR_DYING_ACTIVE_STYLE.frame_address {
        return Err(Intro2CommonDyingBlock::TaskUnavailable);
    }
    if entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Secondary)
        .is_some()
        || entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Tertiary)
            .is_some()
    {
        return Err(Intro2CommonDyingBlock::TaskUnavailable);
    }
    let task_id = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .ok_or(Intro2CommonDyingBlock::TaskUnavailable)?;
    if !matches!(
        entity.actor_tasks.task_state(task_id),
        Some(ActorTaskRuntime::CommonDying(_))
    ) {
        return Err(Intro2CommonDyingBlock::TaskUnavailable);
    }
    Ok(ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id,
    })
}

fn authenticate_components(
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), Intro2CommonDyingBlock> {
    let (expected_mass, expected_model, expected) =
        if crate::intro2_type16::intro2_type16_allocation_authenticates(entity) {
            (100, 257, crate::intro2_type16::TOPOLOGY)
        } else if crate::native_type30::allocation_authenticates(entity) {
            (
                100,
                crate::native_type30::MODEL as u16,
                crate::native_type30::TOPOLOGY,
            )
        } else if crate::native_type122::type122_allocation_authenticates(entity) {
            (100, 274, crate::native_type122::TOPOLOGY)
        } else if crate::intro2_type58::intro2_type58_allocation_authenticates(entity) {
            (100, 273, crate::intro2_type58::TOPOLOGY)
        } else if crate::intro2_type94::intro2_type94_allocation_authenticates(entity) {
            (100, 272, crate::intro2_type94::TOPOLOGY)
        } else if crate::intro2_type17::intro2_type17_allocation_authenticates(entity) {
            (
                100,
                256,
                crate::type17_impact_live::TYPE17_MODEL256_COMPONENT_TOPOLOGY,
            )
        } else if crate::intro2_type26_defecate_virus::intro2_type26_allocation_authenticates(
            entity,
        ) {
            (
                400,
                267,
                crate::intro2_type26_defecate_virus::INTRO2_TYPE26_COMMON_MOVER_TOPOLOGY,
            )
        } else {
            let mut topology =
                crate::ordinary_type47_live::FRESH_LEVEL1_ORDINARY_TYPE47_COMPONENT_TOPOLOGY;
            // Type53 omits E; a null class12 target skips its target correction.
            if crate::intro2_type53::intro2_type53_allocation_authenticates(entity) {
                topology.sub_e = false;
            } else if !crate::intro2_type47_live::intro2_type47_cohort_runtime_authenticates(entity)
            {
                return Err(Intro2CommonDyingBlock::UnauthenticatedAllocation);
            }
            (100, 302, topology)
        };
    if metadata.capability_flags != 8
        || entity.capability_flags != 8
        || metadata.mass_raw != expected_mass
    {
        return Err(Intro2CommonDyingBlock::Metadata(
            "local class12 capability/mass",
        ));
    }
    if metadata.common_mover_topology != RetailRuntimeValue::Known(expected) {
        return Err(Intro2CommonDyingBlock::Metadata("A/B/C/D/E/H/J topology"));
    }
    if metadata.model_slots != [expected_model; 4]
        || entity.model_slots != [Some(usize::from(expected_model)); 4]
    {
        return Err(Intro2CommonDyingBlock::Metadata("model slots"));
    }
    if !matches!(
        entity.sub_a_propulsion_runtime,
        RetailRuntimeValue::Known(Some(_))
    ) {
        return Err(Intro2CommonDyingBlock::Runtime("Sub-A"));
    }
    let RetailRuntimeValue::Known(Some(descriptor)) = &metadata.sub_h_external_frame_descriptor
    else {
        return Err(Intro2CommonDyingBlock::Metadata("Sub-H"));
    };
    let RetailRuntimeValue::Known(Some(runtime)) = &entity.sub_h_external_frame_runtime else {
        return Err(Intro2CommonDyingBlock::Runtime("Sub-H"));
    };
    if runtime.records().len() != descriptor.records.len() {
        return Err(Intro2CommonDyingBlock::Runtime("Sub-H shape"));
    }
    Ok(())
}

/// 10C10 -> common vtable DB80 -> null current death hook -> AC60 alternate12
/// -> C620. All fallible constructor checks precede this helper's first write.
pub fn publish_intro2_common_standard_death(
    manager: &mut EntityManager,
    entity_id: u32,
    world_fx: &mut WorldFx,
) -> Result<Option<Intro2CommonDyingOwner>, Intro2CommonDyingBlock> {
    publish_common_dying(manager, entity_id, world_fx, false)
}

/// AC60's already-dying alternate is a fresh C620 initialization, even when
/// the current graph is already class12 (DB80's post-D040 continuation).
pub(crate) fn publish_intro2_common_dying_alternate(
    manager: &mut EntityManager,
    entity_id: u32,
    world_fx: &mut WorldFx,
) -> Result<Option<Intro2CommonDyingOwner>, Intro2CommonDyingBlock> {
    publish_common_dying(manager, entity_id, world_fx, true)
}

fn publish_common_dying(
    manager: &mut EntityManager,
    entity_id: u32,
    world_fx: &mut WorldFx,
    alternate_only: bool,
) -> Result<Option<Intro2CommonDyingOwner>, Intro2CommonDyingBlock> {
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == entity_id)
        .ok_or(Intro2CommonDyingBlock::AllocationUnavailable)?;
    if !native_manager_allocation(manager, entity_id) {
        return Err(Intro2CommonDyingBlock::UnauthenticatedAllocation);
    }
    let flags = match entity
        .collision
        .state_flags_at_0x08
        .masked(REMOTE_OWNED_STATE_BIT | DYING_STATE_BIT)
    {
        RetailRuntimeValue::Known(flags) => flags,
        _ => return Err(Intro2CommonDyingBlock::Runtime("death entry bits")),
    };
    if flags & REMOTE_OWNED_STATE_BIT != 0 || (!alternate_only && flags & DYING_STATE_BIT != 0) {
        return Ok(None);
    }
    if alternate_only && flags & DYING_STATE_BIT == 0 {
        return Err(Intro2CommonDyingBlock::Runtime(
            "alternate requires dying entry",
        ));
    }
    let metadata = manager
        .type_runtime_metadata(entity.entity_type)
        .cloned()
        .ok_or(Intro2CommonDyingBlock::Metadata("type"))?;
    authenticate_components(entity, &metadata)?;
    if metadata
        .initializer
        .as_ref()
        .map(|init| init.alternate_behavior_class_ref)
        != Some(12)
    {
        return Err(Intro2CommonDyingBlock::Metadata("alternate class"));
    }
    if entity.collision.constructor_sound_attachment_id_at_0x8c != RetailRuntimeValue::Known(None)
        || metadata.constructor_sound_attachment_id != RetailRuntimeValue::Known(None)
    {
        return Err(Intro2CommonDyingBlock::Runtime("death sound attachment"));
    }
    let RetailRuntimeValue::Known(death_sound) = metadata.death_sound_id else {
        return Err(Intro2CommonDyingBlock::Metadata("death sound"));
    };
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(Intro2CommonDyingBlock::Runtime("current behavior"));
    };
    let death_hook = context.active_style().death_callback_policy();
    if !alternate_only && death_hook != DeathCallbackPolicy::None {
        return Err(Intro2CommonDyingBlock::UnsupportedDeathHook(death_hook));
    }
    let program = audited_behavior_program(12).unwrap();
    let selected = context
        .reselect_audited_type_default(program, 0, COMMON_ACTOR_DYING_ACTIVE_STYLE)
        .ok_or(Intro2CommonDyingBlock::Runtime("alternate context"))?;
    let prepared = CommonDyingTaskState::prepare_after_allocation(
        entity_id,
        &metadata,
        CommonDyingComponentDescriptors::default(),
    )
    .map_err(Intro2CommonDyingBlock::Constructor)?
    .map_task(ActorTaskRuntime::CommonDying);
    let allocation = manager
        .main_base_abort_actor_observation(entity_id)
        .unwrap()
        .lease;
    let entity = manager.entity_mut(entity_id).unwrap();
    let position = entity.position_raw();
    let mut velocity = entity.velocity_raw();
    if !alternate_only {
        entity.collision.health_raw = RetailRuntimeValue::Known(0);
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(DYING_STATE_BIT, DYING_STATE_BIT);
        if let Some(sound) = death_sound {
            world_fx.queue_fixed_positional_sound_raw(sound, position);
        }
    }
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(selected));
    let policy = initial_behavior_state_policy(program);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);
    let Entity {
        actor_tasks,
        sub_a_propulsion_runtime,
        sub_h_external_frame_runtime,
        ..
    } = entity;
    let RetailRuntimeValue::Known(Some(sub_a)) = sub_a_propulsion_runtime else {
        unreachable!()
    };
    let RetailRuntimeValue::Known(Some(sub_h)) = sub_h_external_frame_runtime else {
        unreachable!()
    };
    let mut prepared = Some(prepared);
    plan_common_dying_setup(entity_id)
        .apply(
            actor_tasks,
            || u32::from(world_fx.next_shared_retail_random_u16()),
            |effect| match effect {
                CommonDyingConstructorEffect::WriteSubHState08 { value } => {
                    sub_h.set_enabled(value != 0)
                }
                CommonDyingConstructorEffect::WriteSubADirection {
                    direction_multiplier,
                } => sub_a.set_direction_multiplier(direction_multiplier),
                CommonDyingConstructorEffect::WriteSubATargetSpeed {
                    target_speed_raw, ..
                } => sub_a.apply_shared_initializer_target_speed_write(target_speed_raw),
                CommonDyingConstructorEffect::WriteOwnerVerticalVelocity { velocity_raw } => {
                    velocity[1] = velocity_raw
                }
                _ => unreachable!("authenticated A/H class12 constructor"),
            },
            |_| Ok::<_, ()>(prepared.take().unwrap()),
        )
        .expect("host task allocation and prevalidated suffix");
    // C620 does not clear the transient B2 accumulator.
    entity.set_velocity_raw(velocity);
    let visit = current_visit(entity).expect("C620 installed primary last");
    Ok(Some(Intro2CommonDyingOwner {
        allocation,
        visit,
        pending: false,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity_scheduler::SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT;
    use crate::intro2_type47_live::world::native_intro2_fixture;

    #[v2k_test_support::retail_test]
    fn intro2_class12_type53_uses_authored_speed_and_preserves_b2_until_world_tail() {
        let Some((session, mut manager, _)) = native_intro2_fixture() else {
            return;
        };
        let id = manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(38))
            .unwrap()
            .id;
        let entity = manager.entity_mut(id).unwrap();
        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(9);
        entity.set_motion_raw([0, 10_000, 0], [50, -80, 100]);
        let mut fx = WorldFx::new();
        let mut control = WorldFx::new();
        let sample = control.next_shared_retail_random_u16();
        let owner = publish_intro2_common_standard_death(&mut manager, id, &mut fx)
            .unwrap()
            .unwrap();
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(entity.velocity_raw(), [50, 500, 100]);
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(9)
        );
        let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
            panic!()
        };
        assert_eq!(
            sub_a.target_speed_raw(),
            RetailRuntimeValue::Known(400 + i32::from(sample >> 8) * 400 / 0xa00)
        );
        // The positional sound is queued until the audio/event drain; that
        // drain does not consume the constructor's shared-RNG stream.
        assert_eq!(fx.pending_event_count(), 1);
        fx.process_pending();
        let sounds = fx.take_positional_sounds();
        assert_eq!(sounds.len(), 1);
        assert_eq!(sounds[0].sound_id, 75);
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            control.next_shared_retail_random_u16()
        );
        assert_eq!(
            publish_intro2_common_standard_death(&mut manager, id, &mut fx).unwrap(),
            None
        );
        manager
            .entity_mut(id)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .overwrite(
                SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
                SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
            );
        let tick = tick_intro2_common_dying(
            &mut manager,
            owner,
            Intro2CommonDyingFrame {
                resources: &session.cache,
                world_fx: &mut fx,
                elapsed_micros: 20_000,
                retail_tick: 1,
            },
        );
        assert!(
            matches!(
                tick.outcome,
                Intro2CommonDyingOutcome::Advanced {
                    detailed: true,
                    terminal: false,
                    ..
                }
            ),
            "{:?}",
            tick.outcome
        );
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(entity.mass_raw, 109);
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
        assert!(manager.pending_actor_deferred_destroy_ids().is_empty());
    }

    #[v2k_test_support::retail_test]
    fn intro2_class12_detailed_has_strict_lifetime_and_coarse_tags_immediately() {
        for detailed in [true, false] {
            let Some((session, mut manager, id)) = native_intro2_fixture() else {
                return;
            };
            let mut fx = WorldFx::new();
            let owner = publish_intro2_common_standard_death(&mut manager, id, &mut fx)
                .unwrap()
                .unwrap();
            let entity = manager.entity_mut(id).unwrap();
            entity.set_motion_raw([0, 10_000, 0], [100, 500, 100]);
            entity.collision.state_flags_at_0x08.overwrite(
                SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
                if detailed {
                    SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT
                } else {
                    0
                },
            );
            entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(250_000);
            entity.collision.callback_scheduler_accumulator_us_at_0x6c =
                RetailRuntimeValue::Known(if detailed { 0 } else { 125_001 });
            if detailed {
                let ActorTaskRuntime::CommonDying(task) = entity
                    .actor_tasks
                    .task_state_mut(owner.visit.task_id)
                    .unwrap()
                else {
                    panic!()
                };
                task.before_callback(9_000_000);
            }
            let tick = tick_intro2_common_dying(
                &mut manager,
                owner,
                Intro2CommonDyingFrame {
                    resources: &session.cache,
                    world_fx: &mut fx,
                    elapsed_micros: 0,
                    retail_tick: 0,
                },
            );
            assert!(
                matches!(tick.outcome, Intro2CommonDyingOutcome::Advanced {terminal, ..} if terminal == !detailed),
                "{:?}",
                tick.outcome
            );
            if detailed {
                let tick = tick_intro2_common_dying(
                    &mut manager,
                    tick.retained_owner.unwrap(),
                    Intro2CommonDyingFrame {
                        resources: &session.cache,
                        world_fx: &mut fx,
                        elapsed_micros: 1_000,
                        retail_tick: 1,
                    },
                );
                assert!(
                    matches!(
                        tick.outcome,
                        Intro2CommonDyingOutcome::Advanced {
                            terminal: true,
                            detailed: true,
                            ..
                        }
                    ),
                    "{:?}",
                    tick.outcome
                );
                assert!(tick.retained_owner.is_none());
            }
            assert_eq!(manager.pending_actor_deferred_destroy_ids(), [id]);
            assert!(manager
                .entity_mut(id)
                .unwrap()
                .actor_task_state(ActorTaskSlot::Primary)
                .is_none());
        }
    }
}

#[cfg(test)]
mod type94_tests;
