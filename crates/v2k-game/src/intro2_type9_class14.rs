//! Native Type9 class14 world continuation for Intro2 and authored worlds.
//!
//! The shared C3A0/01120/01430 task retains its existing allocation/task lease.
//! This owner supplies the live 12DA0 scheduler and DCA0/E870 suffix instead of
//! importing the Main Base capture's exact coarse state, timer, or mass word.

use crate::common_mover::type9_attitude::Type9BodyBasis;
use crate::common_mover::type9_tail::{
    apply_type9_ground_snap_raw, plan_common_master_motion,
    COMMON_MASTER_MOTION_REQUIRED_STATE_MASK,
};
use crate::entity::{
    apply_type13_common_environment_raw, commit_common_master_motion, Entity, EntityManager,
};
use crate::entity_collision_state::{
    RetailRuntimeValue, BODY_BASIS_REBUILT_STATE_BIT, REMOTE_OWNED_STATE_BIT,
};
use crate::entity_scheduler::*;
use crate::main_base_type9_abort::{
    exact_level_one_type9_metadata, MainBaseType9ExplodingTaskLease,
};
use crate::main_base_type9_production::{
    tick_main_base_type9_exploding_owner, MainBaseType9ExplodingProductionFrame,
    MainBaseType9ExplodingProductionOutcome, MainBaseType9ExplodingProductionOwner,
};
use crate::resource_cache::ResourceCache;
use crate::world_fx::{ParticleEnvironment, TerrainCollisionContext, WorldFx};

pub struct Intro2Type9Class14Frame<'a> {
    pub resources: &'a ResourceCache,
    pub world_fx: &'a mut WorldFx,
    pub elapsed_micros: u32,
    pub global_elapsed_micros: u32,
    pub retail_tick: u32,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Intro2Type9Class14Owner {
    lease: MainBaseType9ExplodingTaskLease,
    task: Option<MainBaseType9ExplodingProductionOwner>,
    pending: bool,
}

impl Intro2Type9Class14Owner {
    pub fn adopt(lease: MainBaseType9ExplodingTaskLease) -> Self {
        Self {
            lease,
            task: Some(MainBaseType9ExplodingProductionOwner::adopt(lease)),
            pending: false,
        }
    }
    pub const fn entity_id(&self) -> u32 {
        self.lease.actor().entity_id
    }
    pub(crate) const fn actor_lease(&self) -> crate::main_base_abort::MainBaseAbortActorLease {
        self.lease.actor()
    }
    pub(crate) fn completed_hit_boundary(&self, manager: &EntityManager) -> bool {
        !self.pending
            && self.task.is_some()
            && crate::ordinary_type9_outer_tail::outer_tail_class14_exploding_task_lease(
                manager,
                self.lease.actor(),
            ) == Some(self.lease)
    }
    pub(crate) fn fork_for_main_base_abort_transaction(&self) -> Self {
        Self {
            lease: self.lease,
            task: self
                .task
                .as_ref()
                .map(|task| task.fork_for_main_base_abort_transaction()),
            pending: self.pending,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2Type9Class14Block {
    Runtime(&'static str),
    Metadata(&'static str),
    Task(MainBaseType9ExplodingProductionOutcome),
    Surface(crate::intro2_common_dying::Intro2CommonDyingBlock),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2Type9Class14Outcome {
    Waiting {
        entity_id: u32,
    },
    Advanced {
        entity_id: u32,
        detailed: bool,
        callback_elapsed_micros: u32,
        task: Option<MainBaseType9ExplodingProductionOutcome>,
    },
    Blocked {
        entity_id: u32,
        reason: Intro2Type9Class14Block,
        prefix_committed: bool,
    },
    Pending {
        entity_id: u32,
    },
    Dropped {
        entity_id: u32,
    },
}
impl Intro2Type9Class14Outcome {
    pub const fn entity_id(&self) -> u32 {
        match self {
            Self::Waiting { entity_id }
            | Self::Advanced { entity_id, .. }
            | Self::Blocked { entity_id, .. }
            | Self::Pending { entity_id }
            | Self::Dropped { entity_id } => *entity_id,
        }
    }
}
pub struct Intro2Type9Class14Tick {
    pub outcome: Intro2Type9Class14Outcome,
    pub retained_owner: Option<Intro2Type9Class14Owner>,
}

pub fn tick_intro2_type9_class14(
    manager: &mut EntityManager,
    mut owner: Intro2Type9Class14Owner,
    frame: Intro2Type9Class14Frame<'_>,
) -> Intro2Type9Class14Tick {
    let id = owner.entity_id();
    let native = crate::ordinary_type9_construction::ordinary_type9_native_allocation_authenticates(
        manager, id,
    ) || manager
        .iter_all()
        .find(|entity| entity.id == id)
        .is_some_and(crate::intro2_type9::intro2_type9_allocation_authenticates);
    if !native
        || manager
            .main_base_abort_actor_observation(id)
            .map(|entry| entry.lease)
            != Some(owner.lease.actor())
    {
        return Intro2Type9Class14Tick {
            outcome: Intro2Type9Class14Outcome::Dropped { entity_id: id },
            retained_owner: None,
        };
    }
    if owner.pending {
        return Intro2Type9Class14Tick {
            outcome: Intro2Type9Class14Outcome::Pending { entity_id: id },
            retained_owner: Some(owner),
        };
    }
    let mut prefix_committed = false;
    match run_frame(manager, &mut owner, frame, &mut prefix_committed) {
        Ok(outcome) => Intro2Type9Class14Tick {
            outcome,
            retained_owner: owner.task.is_some().then_some(owner),
        },
        Err(reason) => {
            owner.pending = prefix_committed;
            Intro2Type9Class14Tick {
                outcome: Intro2Type9Class14Outcome::Blocked {
                    entity_id: id,
                    reason,
                    prefix_committed,
                },
                retained_owner: Some(owner),
            }
        }
    }
}

fn bits(entity: &Entity, mask: u32, label: &'static str) -> Result<u32, Intro2Type9Class14Block> {
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(bits) => Ok(bits),
        _ => Err(Intro2Type9Class14Block::Runtime(label)),
    }
}

fn run_frame(
    manager: &mut EntityManager,
    owner: &mut Intro2Type9Class14Owner,
    frame: Intro2Type9Class14Frame<'_>,
    prefix_committed: &mut bool,
) -> Result<Intro2Type9Class14Outcome, Intro2Type9Class14Block> {
    use Intro2Type9Class14Block as Block;
    let id = owner.entity_id();
    let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
    let metadata = manager
        .type_runtime_metadata(9)
        .cloned()
        .ok_or(Block::Metadata("type9"))?;
    if !exact_level_one_type9_metadata(&metadata) {
        return Err(Block::Metadata("type9 profile"));
    }
    let RetailRuntimeValue::Known(effects) = metadata.common_world_effects else {
        return Err(Block::Metadata("world effects"));
    };
    if effects.surface_selectors != [1, 0]
        || effects.surface_lifetime_ms != 5_000
        || effects.low_health_effect_words != [0; 3]
    {
        return Err(Block::Metadata("world effects profile"));
    }
    let terrain = frame
        .resources
        .level_terrain()
        .ok_or(Block::Metadata("terrain"))?;
    let particle_environment = ParticleEnvironment::Terrain(
        TerrainCollisionContext::from_current_level_cache(frame.resources)
            .ok_or(Block::Metadata("particle environment"))?,
    );
    let model = frame
        .resources
        .global_model(558)
        .ok_or(Block::Metadata("model558"))?;
    let environment = manager.intro2_type13_environment();
    if environment.0 != 0 {
        return Err(Block::Runtime("wind mode"));
    }
    let state = bits(
        entity,
        REMOTE_OWNED_STATE_BIT
            | 0x1000
            | COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT
            | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT
            | COMMON_MASTER_MOTION_REQUIRED_STATE_MASK,
        "outer state",
    )?;
    if state & REMOTE_OWNED_STATE_BIT != 0 {
        return Err(Block::Runtime("local owner"));
    }
    let relation = crate::intro2_type17::capture::prepare_native_corpse_relation(manager, id)
        .map_err(|block| Block::Runtime(block.reason))?;
    if entity.collision.constructor_sound_attachment_id_at_0x8c != RetailRuntimeValue::Known(None) {
        return Err(Block::Runtime("sound attachment"));
    }
    if entity.collision.default_state_flags_at_0xc8 != RetailRuntimeValue::Known(0x2f) {
        return Err(Block::Runtime("default flags"));
    }
    let RetailRuntimeValue::Known(prefix) =
        plan_common_scheduler_prefix(&entity.collision, frame.elapsed_micros, &mut || {
            u32::from(frame.world_fx.next_shared_retail_random_u16())
        })
    else {
        return Err(Block::Runtime("scheduler"));
    };
    let entity = manager.entity_mut(id).unwrap();
    commit_common_scheduler_prefix(&mut entity.collision, prefix);
    *prefix_committed = true;
    let CommonSchedulerPrefixFlow::Continue {
        callback_elapsed_us: dt,
    } = prefix.flow
    else {
        return Ok(Intro2Type9Class14Outcome::Waiting { entity_id: id });
    };
    // 12DA0 repairs an existing parent's missing row after scheduler age/RNG,
    // before DCA0/E870. The cached entry flags still select this visit's mode.
    crate::intro2_type17::capture::commit_native_corpse_relation(entity, relation);
    let detailed = state & SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT != 0;
    if state & COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT == 0 {
        commit_motion(entity, dt)?;
        return Ok(Intro2Type9Class14Outcome::Advanced {
            entity_id: id,
            detailed,
            callback_elapsed_micros: dt,
            task: None,
        });
    }
    let RetailRuntimeValue::Known(mass) = common_scheduler_callback_mass(
        metadata.mass_raw,
        entity.collision.animation_offset_at_0xb2,
    ) else {
        return Err(Block::Runtime("callback B2"));
    };
    entity.mass_raw = mass;
    // The native constructor receipt supplies the same ABDI descriptors consumed by the
    // existing task owner; no Main Base coarse-state facts are imported.
    let task_tick = tick_main_base_type9_exploding_owner(
        manager,
        owner.task.take().unwrap(),
        MainBaseType9ExplodingProductionFrame {
            scheduler_mode: if detailed { 0 } else { 1 },
            terrain,
            elapsed_micros: dt,
            global_elapsed_micros: frame.global_elapsed_micros,
        },
        &mut || u32::from(frame.world_fx.next_shared_retail_random_u16()),
    );
    owner.task = task_tick.retained_owner;
    let task = task_tick.outcome;
    if matches!(
        task,
        MainBaseType9ExplodingProductionOutcome::Blocked { .. }
            | MainBaseType9ExplodingProductionOutcome::Dropped { .. }
            | MainBaseType9ExplodingProductionOutcome::SchedulerWaiting { .. }
    ) {
        return Err(Block::Task(task));
    }
    let entity = manager.entity_mut(id).unwrap();
    // Effective2F has no E640 bit. F70 precedes E100 and DF70 samples old X/Z.
    let [heading, pitch, roll] = entity.rotation_heading_pitch_roll_raw();
    entity.physical_body_basis_q31 =
        RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(heading, pitch, roll));
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(BODY_BASIS_REBUILT_STATE_BIT, BODY_BASIS_REBUILT_STATE_BIT);
    let mut velocity = entity.velocity_raw();
    apply_type13_common_environment_raw(&mut velocity, dt, mass, environment.0, environment.1);
    let mut position = entity.position_raw();
    let mut ground_flags = 0;
    apply_type9_ground_snap_raw(&mut position, &mut velocity, &mut ground_flags, terrain);
    entity.set_motion_raw(position, velocity);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(0x0080_0000, ground_flags & 0x0080_0000);
    crate::intro2_common_dying::run_dying_actor_surface(
        entity,
        &metadata,
        terrain,
        model.radius,
        dt,
        frame.retail_tick,
        particle_environment,
        frame.world_fx,
    )
    .map_err(Block::Surface)?;
    commit_motion(entity, dt)?;
    Ok(Intro2Type9Class14Outcome::Advanced {
        entity_id: id,
        detailed,
        callback_elapsed_micros: dt,
        task: Some(task),
    })
}

fn commit_motion(entity: &mut Entity, dt: u32) -> Result<(), Intro2Type9Class14Block> {
    let state = bits(
        entity,
        COMMON_MASTER_MOTION_REQUIRED_STATE_MASK,
        "master motion",
    )?;
    let motion = plan_common_master_motion(entity.position_raw(), entity.velocity_raw(), state, dt);
    commit_common_scheduler_post_callback(&mut entity.collision);
    commit_common_master_motion(entity, motion);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intro2_type47_live::world::native_intro2_fixture;
    use crate::main_base_type9_abort::MainBaseType9ResultScreenState;
    use crate::ordinary_type9_standard_death::{
        OrdinaryType9StandardDeathEntry, OrdinaryType9StandardDeathOutcome,
    };

    #[v2k_test_support::retail_test]
    fn intro2_class14_block_keeps_the_checked_damage_health_prefix() {
        let Some((_session, mut manager, _)) = native_intro2_fixture() else {
            return;
        };
        let id = manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(3))
            .unwrap()
            .id;
        let entity = manager.entity_mut(id).unwrap();
        entity.current_behavior_context = RetailRuntimeValue::Known(None);
        entity.collision.state_flags_at_0x08.overwrite(
            crate::entity_collision_state::CHECKED_DAMAGE_ENABLED_STATE_BIT,
            crate::entity_collision_state::CHECKED_DAMAGE_ENABLED_STATE_BIT,
        );
        let mut world_fx = WorldFx::new();
        let result = manager.apply_fun_00411250_type9_checked_damage(
            id,
            crate::damage::FUN_0043F780_DAMAGE_PACKET,
            &mut world_fx,
            0,
            None,
        );
        assert_eq!(
            result,
            crate::entity::Fun00411250Type9DamageOutcome::StandardDeathBlocked
        );
        assert_eq!(
            manager.entity_mut(id).unwrap().collision.health_raw,
            RetailRuntimeValue::Known(-2_500)
        );
        assert_eq!(
            world_fx.pending_event_count(),
            0,
            "blocked 10C10 has not queued its death sound"
        );
    }

    #[v2k_test_support::retail_test]
    fn intro2_class14_native_first_query_preserves_world_tail_and_lifetime_in_both_modes() {
        for detailed in [false, true] {
            let Some((session, mut manager, _)) = native_intro2_fixture() else {
                return;
            };
            let id = manager
                .iter_all()
                .find(|entity| entity.authored_spawn_index == Some(3))
                .unwrap()
                .id;
            let entity = manager.entity_mut(id).unwrap();
            entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(7);
            // E370 is allocation state, independent of the just-created task.
            entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(4_500);
            let mut world_fx = WorldFx::new();
            let mut notifications = crate::gameplay_notifications::GameplayNotifications::new();
            assert!(matches!(
                manager
                    .publish_ordinary_type9_standard_death(
                        id,
                        OrdinaryType9StandardDeathEntry::GenericDeath,
                        MainBaseType9ResultScreenState::NotShown,
                        &mut world_fx,
                        0,
                        Some(&mut notifications)
                    )
                    .unwrap(),
                OrdinaryType9StandardDeathOutcome::Published { .. }
            ));
            let entity = manager.entity_mut(id).unwrap();
            assert_eq!(
                entity.collision.animation_offset_at_0xb2,
                RetailRuntimeValue::Known(7)
            );
            assert_eq!(
                entity.surface_lifetime_timer_ms_at_0x48,
                RetailRuntimeValue::Known(4_500)
            );
            // The native allocation's own first-query receipt passes through
            // standard-death component custody. No origin is installed by
            // this test or borrowed from the Level-1 allocation cohort.
            entity.collision.state_flags_at_0x08.overwrite(
                SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
                if detailed {
                    SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT
                } else {
                    0
                },
            );
            let animation_at_birth = entity.actor_animation_runtime;
            let allocation = manager.main_base_abort_actor_observation(id).unwrap().lease;
            let lease = crate::ordinary_type9_outer_tail::outer_tail_class14_exploding_task_lease(
                &manager, allocation,
            )
            .unwrap();
            let mut owner = Intro2Type9Class14Owner::adopt(lease);
            let mut terminal = false;
            for tick in 1..=9 {
                let entity = manager.entity_mut(id).unwrap();
                // Exact over-threshold branches consume no scheduler random
                // words in either detail mode, and cap the callback to 125ms.
                entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(250_000);
                entity.collision.callback_scheduler_accumulator_us_at_0x6c =
                    RetailRuntimeValue::Known(125_001);
                let result = tick_intro2_type9_class14(
                    &mut manager,
                    owner,
                    Intro2Type9Class14Frame {
                        resources: &session.cache,
                        world_fx: &mut world_fx,
                        elapsed_micros: 0,
                        global_elapsed_micros: 20_000,
                        retail_tick: tick,
                    },
                );
                let Intro2Type9Class14Outcome::Advanced {
                    task: Some(task), ..
                } = result.outcome
                else {
                    panic!("{:?}", result.outcome);
                };
                if tick == 1 {
                    assert_eq!(manager.entity_mut(id).unwrap().mass_raw, 17);
                    let animation = manager.entity_mut(id).unwrap().actor_animation_runtime;
                    if detailed {
                        let RetailRuntimeValue::Known(Some(animation)) = animation else {
                            panic!()
                        };
                        assert_eq!(
                            animation.output(),
                            39,
                            "detailed Sub-I selects exploding-person output"
                        );
                    } else {
                        assert_eq!(
                            animation, animation_at_birth,
                            "coarse mode must skip Sub-I entirely"
                        );
                    }
                }
                assert_eq!(
                    manager
                        .entity_mut(id)
                        .unwrap()
                        .collision
                        .animation_offset_at_0xb2,
                    RetailRuntimeValue::Known(0)
                );
                terminal = matches!(task, MainBaseType9ExplodingProductionOutcome::Terminal(_));
                if terminal {
                    assert!(tick > 8, "lifetime is strict");
                    assert!(result.retained_owner.is_none());
                    break;
                }
                owner = result.retained_owner.unwrap();
            }
            assert!(terminal);
            assert_eq!(manager.pending_actor_deferred_destroy_ids(), &[id]);
        }
    }

    #[v2k_test_support::retail_test]
    fn intro2_class14_removed_origin_receipt_retains_prefix_without_replay() {
        use crate::actor_task_dispatcher::ActorTaskRuntime;
        use crate::actor_task_owner::ActorTaskSlot;
        use crate::common_mover::sub_d::Type9SubDStep;
        use crate::common_mover::type9::OrdinaryType9FrameBlock;
        use crate::main_base_type9_production::MainBaseType9ExplodingProductionBlock;
        use crate::shared_retarget_mover::{SharedRetarget, SharedRetargetTrigger};

        let Some((session, mut manager, _)) = native_intro2_fixture() else {
            return;
        };
        let id = manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(3))
            .unwrap()
            .id;
        manager
            .entity_mut(id)
            .unwrap()
            .collision
            .animation_offset_at_0xb2 = RetailRuntimeValue::Known(7);
        let mut world_fx = WorldFx::new();
        let mut control = WorldFx::new();
        let mut notifications = crate::gameplay_notifications::GameplayNotifications::new();
        assert!(matches!(
            manager
                .publish_ordinary_type9_standard_death(
                    id,
                    OrdinaryType9StandardDeathEntry::GenericDeath,
                    MainBaseType9ResultScreenState::NotShown,
                    &mut world_fx,
                    0,
                    Some(&mut notifications),
                )
                .unwrap(),
            OrdinaryType9StandardDeathOutcome::Published { .. }
        ));
        let entity = manager.entity_mut(id).unwrap();
        // Remove only this fixture's authenticated first-query receipt to
        // exercise an unresolved allocator boundary. Native Intro2 births now
        // carry their own per-birth receipt established by the TTD query.
        let components = &mut entity
            .main_base_type9_death_component_runtime
            .as_mut()
            .unwrap()
            .components;
        let seed = components
            .sub_d_frame_owner
            .classifier_cache()
            .stagger_counter();
        components.sub_d_frame_owner =
            crate::common_mover::sub_d::Type9SubDFrameOwner::pending_constructor_origin(seed);
        entity.collision.state_flags_at_0x08.overwrite(
            SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
            SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
        );
        entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(250_000);
        entity.collision.callback_scheduler_accumulator_us_at_0x6c =
            RetailRuntimeValue::Known(125_001);
        let motion_before = (entity.position_raw(), entity.velocity_raw());
        let components_before = entity.main_base_type9_death_component_runtime;
        let task_before = entity.actor_task_state(ActorTaskSlot::Primary).copied();
        let allocation = manager.main_base_abort_actor_observation(id).unwrap().lease;
        let lease = crate::ordinary_type9_outer_tail::outer_tail_class14_exploding_task_lease(
            &manager, allocation,
        )
        .unwrap();
        let tick = tick_intro2_type9_class14(
            &mut manager,
            Intro2Type9Class14Owner::adopt(lease),
            Intro2Type9Class14Frame {
                resources: &session.cache,
                world_fx: &mut world_fx,
                elapsed_micros: 0,
                global_elapsed_micros: 20_000,
                retail_tick: 1,
            },
        );
        let Intro2Type9Class14Outcome::Blocked {
            reason:
                Intro2Type9Class14Block::Task(MainBaseType9ExplodingProductionOutcome::Blocked {
                    reason:
                        MainBaseType9ExplodingProductionBlock::FrameBlocked {
                            committed_prefix,
                            reason:
                                OrdinaryType9FrameBlock::SubD(Type9SubDStep::UnresolvedClassifierCache),
                            ..
                        },
                    ..
                }),
            prefix_committed: true,
            ..
        } = tick.outcome
        else {
            panic!("{:?}", tick.outcome);
        };
        assert_eq!(committed_prefix.elapsed_ms, 125);
        assert!(matches!(
            committed_prefix.retarget,
            SharedRetarget::Replaced {
                trigger: SharedRetargetTrigger::NearTargetAxis,
                ..
            }
        ));
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(entity.mass_raw, 17);
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(7),
            "the blocked callback has not reached its B2 clear"
        );
        assert_eq!(
            (entity.position_raw(), entity.velocity_raw()),
            motion_before
        );
        assert_eq!(
            entity.main_base_type9_death_component_runtime,
            components_before
        );
        let task_after = entity.actor_task_state(ActorTaskSlot::Primary).copied();
        assert_ne!(task_after, task_before);
        let Some(ActorTaskRuntime::SharedRetarget(task)) = task_after else {
            panic!()
        };
        assert_eq!(task.elapsed_ms(), 125);
        assert!(manager.pending_actor_deferred_destroy_ids().is_empty());
        // One death initializer word and two committed retarget words; the
        // over-threshold scheduler branches consume none.
        for _ in 0..3 {
            control.next_shared_retail_random_u16();
        }
        assert_eq!(
            world_fx.next_shared_retail_random_u16(),
            control.next_shared_retail_random_u16()
        );
        let sounds_before = world_fx.pending_event_count();
        let retry = tick_intro2_type9_class14(
            &mut manager,
            tick.retained_owner.unwrap(),
            Intro2Type9Class14Frame {
                resources: &session.cache,
                world_fx: &mut world_fx,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: 2,
            },
        );
        assert_eq!(
            retry.outcome,
            Intro2Type9Class14Outcome::Pending { entity_id: id }
        );
        assert!(retry.retained_owner.is_some());
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(
            entity.actor_task_state(ActorTaskSlot::Primary).copied(),
            task_after
        );
        assert_eq!(
            (entity.position_raw(), entity.velocity_raw()),
            motion_before
        );
        assert_eq!(
            entity.main_base_type9_death_component_runtime,
            components_before
        );
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(7)
        );
        assert_eq!(world_fx.pending_event_count(), sounds_before);
        assert!(manager.pending_actor_deferred_destroy_ids().is_empty());
        assert_eq!(
            world_fx.next_shared_retail_random_u16(),
            control.next_shared_retail_random_u16()
        );
    }
}
