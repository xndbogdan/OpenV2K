//! Carried Type-9 `12DA0 -> DCA0/E870 -> A800 -> 3250 -> 420520`.
//!
//! Carrying has a real None task, and advances linked Sub-I even in the coarse
//! callback. Its effective flags are `(2f | 80) & !2 = ad`: rebuild the body
//! basis and apply drag, but do not apply gravity or ground snap. The bounded
//! surface tail includes the submerged Type93 attachment in NoCD03 and the
//! source-proven expiry boundary: CE90 initializes a real living root before
//! 10C10 replaces it with class14. This visit retains its latched AD tail.

#[cfg(test)]
mod expiry_tests;
mod lifecycle;
use lifecycle::CarriedSurfaceExpiry;

use crate::{
    actor_task_owner::ActorTaskVisitControl,
    common_mover::{
        type9_attitude::Type9BodyBasis,
        type9_surface::{
            classify_actor_surface_timer_phase, decay_actor_surface_timer_ms,
            ActorSurfaceTimerFrame, ActorSurfaceTimerPhase, ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT,
        },
        type9_tail::{plan_common_master_motion, COMMON_MASTER_MOTION_REQUIRED_STATE_MASK},
    },
    entity::{apply_type9_carried_environment_raw, commit_common_master_motion, EntityManager},
    entity_collision_state::{
        CommonWorldEffectProfile, EntityTypeRuntimeMetadata, RetailRuntimeValue,
        BODY_BASIS_REBUILT_STATE_BIT, DYING_STATE_BIT, REMOTE_OWNED_STATE_BIT,
    },
    entity_scheduler::{
        commit_common_scheduler_post_callback, commit_common_scheduler_prefix,
        common_scheduler_callback_mass, plan_common_scheduler_prefix, CommonSchedulerPrefixFlow,
        COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT, SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
    },
    gameplay_notifications::GameplayNotifications,
    intro2_common_dying::{run_actor_surface_with_lifecycle, Intro2ActorSurfaceFrame},
    main_base_type9_abort::{
        exact_level_one_type9_metadata, MainBaseType9ExplodingTaskLease,
        MainBaseType9ResultScreenState, LEVEL_ONE_TYPE9_INITIALIZER_STATE_RAW,
        LEVEL_ONE_TYPE9_MASS_RAW, LEVEL_ONE_TYPE9_MODEL_ID, LEVEL_ONE_TYPE9_SUB_I_DESCRIPTOR,
    },
    ordinary_type9_cargo::Type9CarriedOwner,
    resource_cache::ResourceCache,
    world_fx::{ParticleEnvironment, TerrainCollisionContext, WorldFx},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type9CarriedProductionBlock {
    SchedulerStateUnavailable,
    UnsupportedRelation,
    MetadataUnavailable,
    UnsupportedEnvironment,
    AnimationOffsetUnavailable,
    SurfaceStateUnavailable,
    SurfaceLifecycleUnsupported,
    SoundAttachmentUnsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type9CarriedProductionDrop {
    EntityUnavailable,
    AllocationChanged,
    CarryingStateChanged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type9CarriedProductionOutcome {
    SchedulerWaiting {
        entity_id: u32,
    },
    Continuing {
        entity_id: u32,
        callback_elapsed_micros: u32,
        callback_enabled: bool,
    },
    Class14Published {
        entity_id: u32,
        callback_elapsed_micros: u32,
    },
    Blocked {
        entity_id: u32,
        reason: Type9CarriedProductionBlock,
    },
    Dropped {
        entity_id: u32,
        reason: Type9CarriedProductionDrop,
    },
}

impl Type9CarriedProductionOutcome {
    pub const fn entity_id(&self) -> u32 {
        match self {
            Self::SchedulerWaiting { entity_id }
            | Self::Continuing { entity_id, .. }
            | Self::Class14Published { entity_id, .. }
            | Self::Blocked { entity_id, .. }
            | Self::Dropped { entity_id, .. } => *entity_id,
        }
    }
}

pub(crate) struct Type9CarriedProductionTick {
    pub(crate) retained_owner: Option<Type9CarriedOwner>,
    pub(crate) class14_task_lease: Option<MainBaseType9ExplodingTaskLease>,
    pub(crate) outcome: Type9CarriedProductionOutcome,
}

pub(crate) struct Type9CarriedProductionFrame<'a> {
    pub(crate) resources: &'a ResourceCache,
    pub(crate) world_fx: &'a mut WorldFx,
    pub(crate) notifications: &'a mut GameplayNotifications,
    pub(crate) elapsed_micros: u32,
    pub(crate) retail_tick: u32,
}

struct CarriedDeepSurface<'a> {
    metadata: EntityTypeRuntimeMetadata,
    environment: TerrainCollisionContext<'a>,
    extent_raw: u16,
}

pub(crate) fn tick_type9_carried_owner(
    manager: &mut EntityManager,
    owner: Type9CarriedOwner,
    frame: Type9CarriedProductionFrame<'_>,
    next_random: &mut impl FnMut(&mut WorldFx) -> u32,
) -> Type9CarriedProductionTick {
    let Type9CarriedProductionFrame {
        resources,
        world_fx,
        notifications,
        elapsed_micros,
        retail_tick,
    } = frame;
    let entity_id = owner.entity_id();
    let dropped = |reason| Type9CarriedProductionTick {
        retained_owner: None,
        class14_task_lease: None,
        outcome: Type9CarriedProductionOutcome::Dropped { entity_id, reason },
    };
    let Some(entity) = manager.iter_all().find(|entity| entity.id == entity_id) else {
        return dropped(Type9CarriedProductionDrop::EntityUnavailable);
    };
    if manager.ordinary_type9_selected_actor_lease(entity_id) != Some(owner.actor) {
        return dropped(Type9CarriedProductionDrop::AllocationChanged);
    }
    if !owner.authenticates(entity) {
        return dropped(Type9CarriedProductionDrop::CarryingStateChanged);
    }
    let blocked = |owner, reason| Type9CarriedProductionTick {
        retained_owner: Some(owner),
        class14_task_lease: None,
        outcome: Type9CarriedProductionOutcome::Blocked { entity_id, reason },
    };
    let mask = REMOTE_OWNED_STATE_BIT
        | 0x1000
        | COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT
        | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT
        | ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT
        | COMMON_MASTER_MOTION_REQUIRED_STATE_MASK;
    let RetailRuntimeValue::Known(flags) = entity.collision.state_flags_at_0x08.masked(mask) else {
        return blocked(
            owner,
            Type9CarriedProductionBlock::SchedulerStateUnavailable,
        );
    };
    if flags & (REMOTE_OWNED_STATE_BIT | 0x1000) != 0x1000
        || entity.attached_to != Some(owner.linked_owner)
    {
        return blocked(owner, Type9CarriedProductionBlock::UnsupportedRelation);
    }
    // 12DA0 checks the live parent's ordered Sub-J list before invoking the
    // child. Missing membership would synchronously release and reselect it;
    // this adapter cannot substitute another None tick for that boundary.
    let parent_valid = manager.iter_all().find(|parent| parent.id == owner.linked_owner)
        .is_some_and(|parent| parent.active
            && parent.collision.state_flags_at_0x08.masked(REMOTE_OWNED_STATE_BIT)
                == RetailRuntimeValue::Known(0)
            && matches!(&parent.sub_j_attachment_runtime, RetailRuntimeValue::Known(Some(runtime))
                if runtime.ordered_entity_ids().contains(&entity_id)));
    if !parent_valid {
        return blocked(owner, Type9CarriedProductionBlock::UnsupportedRelation);
    }
    if entity.collision.constructor_sound_attachment_id_at_0x8c != RetailRuntimeValue::Known(None) {
        return blocked(
            owner,
            Type9CarriedProductionBlock::SoundAttachmentUnsupported,
        );
    }
    let callback_enabled = flags & COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT != 0;
    let (wind_mode, drag_strength) = manager.intro2_type13_environment();
    let mut mass = None;
    let mut surface_timer = None;
    let mut deep_surface = None;
    let mut surface_expiry = None;
    if callback_enabled {
        let expected_effects = CommonWorldEffectProfile {
            surface_selectors: [1, 0],
            surface_lifetime_ms: 5_000,
            low_health_effect_words: [0; 3],
        };
        if !manager.type_runtime_metadata(9).is_some_and(|metadata| {
            exact_level_one_type9_metadata(metadata)
                && metadata.common_world_effects == RetailRuntimeValue::Known(expected_effects)
        }) || entity.collision.default_state_flags_at_0xc8
            != RetailRuntimeValue::Known(LEVEL_ONE_TYPE9_INITIALIZER_STATE_RAW)
            || entity.model_index != Some(LEVEL_ONE_TYPE9_MODEL_ID)
            || !matches!(entity.actor_animation_runtime,
                RetailRuntimeValue::Known(Some(animation))
                    if animation.descriptor() == LEVEL_ONE_TYPE9_SUB_I_DESCRIPTOR)
        {
            return blocked(owner, Type9CarriedProductionBlock::MetadataUnavailable);
        }
        if wind_mode != 0 {
            return blocked(owner, Type9CarriedProductionBlock::UnsupportedEnvironment);
        }
        let RetailRuntimeValue::Known(value) = common_scheduler_callback_mass(
            LEVEL_ONE_TYPE9_MASS_RAW,
            entity.collision.animation_offset_at_0xb2,
        ) else {
            return blocked(
                owner,
                Type9CarriedProductionBlock::AnimationOffsetUnavailable,
            );
        };
        mass = Some(value);
        if flags & ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT == 0 {
            let Some(terrain) = resources.level_terrain() else {
                return blocked(owner, Type9CarriedProductionBlock::SurfaceStateUnavailable);
            };
            let Some(model) = resources.global_model(LEVEL_ONE_TYPE9_MODEL_ID) else {
                return blocked(owner, Type9CarriedProductionBlock::SurfaceStateUnavailable);
            };
            let RetailRuntimeValue::Known(timer) = entity.surface_lifetime_timer_ms_at_0x48 else {
                return blocked(owner, Type9CarriedProductionBlock::SurfaceStateUnavailable);
            };
            if i32::from(entity.position_raw()[1])
                < i32::from(terrain.sea_level_raw()) - i32::from(model.radius >> 2)
            {
                // 12DA0's RNG only chooses whether to wait. Every continuing
                // path has the same accumulated/capped delta and B6 override.
                // Zero probe words force Continue without touching process RNG,
                // so nested release/death can still reject before any prefix.
                let RetailRuntimeValue::Known(probe) =
                    plan_common_scheduler_prefix(&entity.collision, elapsed_micros, &mut || 0)
                else {
                    return blocked(
                        owner,
                        Type9CarriedProductionBlock::SchedulerStateUnavailable,
                    );
                };
                let CommonSchedulerPrefixFlow::Continue {
                    callback_elapsed_us,
                } = probe.flow
                else {
                    unreachable!("zero scheduler thresholds always continue");
                };
                let phase = classify_actor_surface_timer_phase(
                    timer,
                    ActorSurfaceTimerFrame {
                        state_flags: flags,
                        position_y_raw: entity.position_raw()[1],
                        active_model_extent_raw: model.radius,
                        flat_surface_y_raw: terrain.sea_level_raw(),
                        elapsed_us: callback_elapsed_us,
                        authored_lifetime_ms: expected_effects.surface_lifetime_ms,
                    },
                )
                .expect("authenticated Type9 has a nonzero surface lifetime");
                if matches!(phase, ActorSurfaceTimerPhase::DeepLifecycle { .. }) {
                    surface_expiry = CarriedSurfaceExpiry::prepare(
                        manager,
                        &owner,
                        manager.type_runtime_metadata(9).unwrap(),
                    );
                    if surface_expiry.is_none() {
                        return blocked(
                            owner,
                            Type9CarriedProductionBlock::SurfaceLifecycleUnsupported,
                        );
                    }
                }
                if matches!(phase, ActorSurfaceTimerPhase::DeepRandomEffects { .. })
                    && matches!(
                        entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT),
                        RetailRuntimeValue::Unresolved
                    )
                {
                    return blocked(owner, Type9CarriedProductionBlock::SurfaceStateUnavailable);
                }
                let Some(environment) =
                    TerrainCollisionContext::from_current_level_cache(resources)
                else {
                    return blocked(owner, Type9CarriedProductionBlock::SurfaceStateUnavailable);
                };
                deep_surface = Some(CarriedDeepSurface {
                    metadata: manager.type_runtime_metadata(9).unwrap().clone(),
                    environment,
                    extent_raw: model.radius,
                });
            } else {
                surface_timer = Some(timer);
            }
        }
    }
    // Every callback-local admission above precedes the first shared RNG word.
    let RetailRuntimeValue::Known(prefix) =
        plan_common_scheduler_prefix(&entity.collision, elapsed_micros, &mut || {
            next_random(world_fx)
        })
    else {
        return blocked(
            owner,
            Type9CarriedProductionBlock::SchedulerStateUnavailable,
        );
    };
    let entity = manager
        .ordinary_type9_selected_entity_mut(entity_id)
        .expect("validated allocation remains live through this synchronous visit");
    commit_common_scheduler_prefix(&mut entity.collision, prefix);
    let CommonSchedulerPrefixFlow::Continue {
        callback_elapsed_us,
    } = prefix.flow
    else {
        return Type9CarriedProductionTick {
            retained_owner: Some(owner),
            class14_task_lease: None,
            outcome: Type9CarriedProductionOutcome::SchedulerWaiting { entity_id },
        };
    };
    if callback_enabled {
        entity.mass_raw = mass.expect("enabled callback preflights its mass");
        let yaw = entity.rotation_heading_pitch_roll_raw()[0] as u16;
        let RetailRuntimeValue::Known(Some(animation)) = &mut entity.actor_animation_runtime else {
            unreachable!("carrying owner authenticates its linked Sub-I controller");
        };
        entity.actor_tasks.visit_slots_fresh::<()>(|tasks, visit| {
            debug_assert_eq!(visit, owner.visit);
            debug_assert!(tasks.wrapper_flags(visit.task_id).unwrap().in_callback);
            // 3250 has no detailed/coarse mode gate and does not run Sub-D.
            let selection = animation.advance(
                callback_elapsed_us,
                yaw,
                animation.linked_handle().is_some(),
            );
            debug_assert!(!selection.zero_velocity);
            ActorTaskVisitControl::Continue
        });
        let [heading, pitch, roll] = entity.rotation_heading_pitch_roll_raw();
        entity.physical_body_basis_q31 =
            RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(heading, pitch, roll));
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(BODY_BASIS_REBUILT_STATE_BIT, BODY_BASIS_REBUILT_STATE_BIT);
        let mut velocity = entity.velocity_raw();
        apply_type9_carried_environment_raw(
            &mut velocity,
            callback_elapsed_us,
            entity.mass_raw,
            wind_mode,
            drag_strength,
        );
        entity.set_velocity_raw(velocity);
        if let Some(timer) = surface_timer {
            entity.surface_lifetime_timer_ms_at_0x48 =
                RetailRuntimeValue::Known(decay_actor_surface_timer_ms(timer, callback_elapsed_us));
        }
    }
    let mut owner = Some(owner);
    let mut class14_task_lease = None;
    if let Some(surface) = deep_surface {
        // Authenticate immediately before the synchronous E370 call; its
        // lifecycle closure then consumes the single carrying receipt.
        let authenticated = owner.as_ref().unwrap().authenticates(
            manager
                .iter_all()
                .find(|entity| entity.id == entity_id)
                .unwrap(),
        );
        // Reuse the common E370 implementation so timer, bubble allocation,
        // and the independent sound gate keep their process-RNG order. F70
        // above supplies the basis. Expiry consumes carrying custody through
        // the actual release owner, then installs class14 before later RNG.
        run_actor_surface_with_lifecycle(
            manager,
            entity_id,
            Intro2ActorSurfaceFrame {
                metadata: &surface.metadata,
                terrain: surface.environment.terrain,
                active_model_extent_raw: surface.extent_raw,
                elapsed_micros: callback_elapsed_us,
                retail_tick,
                particle_environment: ParticleEnvironment::Terrain(surface.environment),
            },
            world_fx,
            &mut class14_task_lease,
            |_| authenticated,
            |manager, _, world_fx| {
                surface_expiry
                    .expect("expiry admission precedes scheduler RNG")
                    .commit(
                        manager,
                        owner
                            .take()
                            .expect("one lifecycle call consumes carrying custody"),
                        &surface.metadata,
                        world_fx,
                        notifications,
                        retail_tick,
                    )
            },
        )
        .expect("carried surface preflight resolves the complete E370 lifecycle");
    }
    let entity = manager
        .ordinary_type9_selected_entity_mut(entity_id)
        .expect("E370 retains the live allocation across class14 publication");
    commit_common_scheduler_post_callback(&mut entity.collision);
    // 12DA0 reloads the live state after E370. AD was latched only for the
    // callback's E100/DF70 policy; CE90/class14 may change master-motion bits.
    let RetailRuntimeValue::Known(master_flags) = entity
        .collision
        .state_flags_at_0x08
        .masked(COMMON_MASTER_MOTION_REQUIRED_STATE_MASK)
    else {
        unreachable!("release/death preserve resolved master-motion inputs");
    };
    let motion = plan_common_master_motion(
        entity.position_raw(),
        entity.velocity_raw(),
        master_flags,
        callback_elapsed_us,
    );
    commit_common_master_motion(entity, motion);
    debug_assert!(owner
        .as_ref()
        .is_none_or(|owner| owner.authenticates(entity)));
    let outcome = if owner.is_some() {
        Type9CarriedProductionOutcome::Continuing {
            entity_id,
            callback_elapsed_micros: callback_elapsed_us,
            callback_enabled,
        }
    } else {
        Type9CarriedProductionOutcome::Class14Published {
            entity_id,
            callback_elapsed_micros: callback_elapsed_us,
        }
    };
    Type9CarriedProductionTick {
        retained_owner: owner,
        class14_task_lease,
        outcome,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        entity::{EntityConstructionResources, EntityManager},
        entity_collision_state::EntityTypeRuntimeMetadata,
        gameplay_notifications::GameplayNotifications,
        ordinary_type9_cargo::{commit_attach, plan_attach, Type9RelationOwner},
        session::GameSession,
        specialized_actor_task_production::{
            SpecializedActorTaskProductionFrame, SpecializedActorTaskScheduler,
        },
        world_fx::WorldFx,
    };

    fn tick_carried(
        manager: &mut EntityManager,
        owner: Type9CarriedOwner,
        resources: &ResourceCache,
        elapsed_micros: u32,
        next_random: &mut impl FnMut() -> u32,
    ) -> Type9CarriedProductionTick {
        tick_type9_carried_owner(
            manager,
            owner,
            Type9CarriedProductionFrame {
                resources,
                world_fx: &mut WorldFx::new(),
                notifications: &mut GameplayNotifications::new(),
                elapsed_micros,
                retail_tick: 0,
            },
            &mut |_| next_random(),
        )
    }

    pub(super) fn carried_fixture() -> (GameSession, EntityManager, Type9CarriedOwner) {
        let mut static_damage = crate::static_damage::StaticDamageScheduler::new();
        let data = v2k_test_support::retail_dir();
        let mut session = GameSession::init(&data).expect("canonical retail corpus required");
        session.load_auxiliary_ovl(3, 1).unwrap();
        session.load_level_by_id(13, 1).unwrap();
        let metadata: Vec<_> = session
            .cache
            .global_entity_model_table()
            .iter()
            .copied()
            .enumerate()
            .map(|(entity_type, model_slots)| {
                session
                    .cache
                    .global_entity_type(entity_type)
                    .map(EntityTypeRuntimeMetadata::from_section12)
                    .unwrap_or(EntityTypeRuntimeMetadata {
                        model_slots,
                        ..Default::default()
                    })
            })
            .collect();
        let mut effects = WorldFx::new();
        let mut manager = EntityManager::from_fresh_new_game_level_with_type_metadata(
            session.cache.level_desc().unwrap(),
            &metadata,
            EntityConstructionResources::new(
                session.cache.terrain(),
                session.cache.terrain_objects(),
            ),
            0,
            &mut effects,
        )
        .unwrap();
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler
            .adopt_fresh_level1_type9_selected(&mut manager)
            .unwrap();
        for retail_tick in 0..3 {
            scheduler.tick(
                &mut manager,
                SpecializedActorTaskProductionFrame {
                    world: crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                    hive_components: None,
                    notification_phase:
                        crate::gameplay_notifications::GameplayNotificationPhase::Playing,
                    resources: &mut session.cache,
                    world_fx: &mut effects,
                    static_damage: &mut static_damage,
                    elapsed_micros: 20_000,
                    global_elapsed_micros: 20_000,
                    retail_tick,
                    main_base_abort_active: false,
                },
                &mut GameplayNotifications::new(),
            );
        }
        let actor = manager
            .iter_all()
            .filter(|entity| entity.entity_type == 9)
            .find_map(|entity| scheduler.prepare_type9_cargo_attach(&manager, entity.id))
            .unwrap();
        let parent = manager.player().unwrap();
        let relation_owner = Type9RelationOwner {
            id: parent.id,
            capability_flags: parent.capability_flags,
            position_raw: parent.position_raw(),
        };
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == actor.entity_id)
            .unwrap();
        let plan = plan_attach(entity, &metadata[9], relation_owner).unwrap();
        assert!(scheduler.take_type9_for_cargo(&manager, actor));
        let RetailRuntimeValue::Known(Some(list)) =
            &mut manager.player_mut().unwrap().sub_j_attachment_runtime
        else {
            panic!("canonical player owns Sub-J")
        };
        list.append(actor.entity_id).unwrap();
        let entity = manager
            .ordinary_type9_selected_entity_mut(actor.entity_id)
            .unwrap();
        entity.attached_to = Some(relation_owner.id);
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(0x1000, 0x1000);
        let owner = commit_attach(entity, actor, plan).unwrap();
        // Dry position deliberately above terrain, with nonzero Y speed:
        // a mistaken ordinary-Wander DF70 tail would snap/zero it.
        entity.set_motion_raw([1_000, 2_000, 3_000], [1_000, 400, 0]);
        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
        entity.collision.callback_scheduler_accumulator_us_at_0x6c = RetailRuntimeValue::Known(0);
        entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
        (session, manager, owner)
    }

    #[v2k_test_support::retail_test]
    fn detailed_and_coarse_none_tick_advance_linked_sub_i_without_sub_d_or_ground_snap() {
        for detailed in [false, true] {
            let (session, mut manager, owner) = carried_fixture();
            let id = owner.entity_id();
            let entity = manager.ordinary_type9_selected_entity_mut(id).unwrap();
            entity.collision.state_flags_at_0x08.overwrite(
                SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
                if detailed {
                    SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT
                } else {
                    0
                },
            );
            let selected_before = entity.ordinary_type9_selected_component_runtime;
            let RetailRuntimeValue::Known(Some(mut expected_animation)) =
                entity.actor_animation_runtime
            else {
                unreachable!()
            };
            expected_animation.advance(20_000, entity.heading_raw(), true);
            let task_before = owner.visit;
            let mut random_count = 0;
            let tick = tick_carried(&mut manager, owner, &session.cache, 20_000, &mut || {
                random_count += 1;
                0
            });
            assert!(
                matches!(
                    tick.outcome,
                    Type9CarriedProductionOutcome::Continuing {
                        callback_elapsed_micros: 20_000,
                        callback_enabled: true,
                        ..
                    }
                ),
                "{detailed}: {:?}",
                tick.outcome
            );
            assert_eq!(random_count, if detailed { 0 } else { 2 });
            let owner = tick.retained_owner.unwrap();
            assert_eq!(owner.visit, task_before);
            let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
            assert_eq!(
                entity.actor_animation_runtime,
                RetailRuntimeValue::Known(Some(expected_animation))
            );
            assert_eq!(
                entity.ordinary_type9_selected_component_runtime,
                selected_before
            );
            assert_eq!(entity.velocity_raw(), [978, 391, 0]);
            assert!(entity.position_raw()[1] >= 2_000);
            assert_eq!(
                entity.collision.animation_offset_at_0xb2,
                RetailRuntimeValue::Known(0)
            );
            let mut scheduler = SpecializedActorTaskScheduler::new();
            scheduler.register_type9_carried(owner).unwrap();
            assert_eq!(scheduler.actor_animation_claims().count(), 1);
        }
    }

    #[v2k_test_support::retail_test]
    fn scheduler_wait_and_unit_delta_keep_the_none_clock_exact() {
        let (session, mut manager, owner) = carried_fixture();
        let id = owner.entity_id();
        let entity = manager.ordinary_type9_selected_entity_mut(id).unwrap();
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT, 0);
        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(9);
        let animation_before = entity.actor_animation_runtime;
        let waiting = tick_carried(&mut manager, owner, &session.cache, 20_000, &mut || 0xffff);
        assert_eq!(
            waiting.outcome,
            Type9CarriedProductionOutcome::SchedulerWaiting { entity_id: id }
        );
        let entity = manager.ordinary_type9_selected_entity_mut(id).unwrap();
        assert_eq!(entity.actor_animation_runtime, animation_before);
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0)
        );
        entity.collision.scheduler_unit_delta_flag_at_0xb6 = RetailRuntimeValue::Known(1);
        let RetailRuntimeValue::Known(Some(mut expected_animation)) = animation_before else {
            unreachable!()
        };
        expected_animation.advance(1, entity.heading_raw(), true);
        let tick = tick_carried(
            &mut manager,
            waiting.retained_owner.unwrap(),
            &session.cache,
            20_000,
            &mut || 0,
        );
        assert!(
            matches!(
                tick.outcome,
                Type9CarriedProductionOutcome::Continuing {
                    callback_elapsed_micros: 1,
                    ..
                }
            ),
            "{:?}",
            tick.outcome
        );
        let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
        assert_eq!(
            entity.actor_animation_runtime,
            RetailRuntimeValue::Known(Some(expected_animation))
        );
        assert_eq!(entity.mass_raw, 10);
    }

    #[v2k_test_support::retail_test]
    fn missing_parent_membership_blocks_before_prefix_or_animation() {
        let (session, mut manager, owner) = carried_fixture();
        let id = owner.entity_id();
        let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
        let before = (
            entity.collision.clone(),
            entity.actor_animation_runtime,
            entity.position_raw(),
        );
        let RetailRuntimeValue::Known(Some(list)) =
            &mut manager.player_mut().unwrap().sub_j_attachment_runtime
        else {
            unreachable!()
        };
        list.clear();
        let tick = tick_carried(&mut manager, owner, &session.cache, 20_000, &mut || {
            panic!("blocked admission must not consume RNG")
        });
        assert_eq!(
            tick.outcome,
            Type9CarriedProductionOutcome::Blocked {
                entity_id: id,
                reason: Type9CarriedProductionBlock::UnsupportedRelation,
            }
        );
        assert!(tick.retained_owner.is_some());
        let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
        assert_eq!(
            (
                entity.collision.clone(),
                entity.actor_animation_runtime,
                entity.position_raw()
            ),
            before
        );
    }

    #[v2k_test_support::retail_test]
    fn carried_nonzero_wind_stays_explicit_unsupported_without_weakening_mode_zero() {
        let (session, mut manager, owner) = carried_fixture();
        let id = owner.entity_id();
        assert_eq!(manager.intro2_type13_environment().0, 0);
        manager.set_runtime_wind_mode_for_test(1);
        let before = manager
            .iter_all()
            .find(|entity| entity.id == id)
            .unwrap()
            .collision
            .clone();
        let tick = tick_carried(&mut manager, owner, &session.cache, 20_000, &mut || {
            panic!("nonzero-wind admission must not consume RNG")
        });
        assert_eq!(
            tick.outcome,
            Type9CarriedProductionOutcome::Blocked {
                entity_id: id,
                reason: Type9CarriedProductionBlock::UnsupportedEnvironment,
            }
        );
        let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
        assert_eq!(entity.collision, before);
    }

    #[v2k_test_support::retail_test]
    fn nocd03_submerged_materialiser_carry_accumulates_545_ms_before_landing_release() {
        // V2000-nocd03.run, 30CBDA:1C05 through 30F3B5:873. These are
        // callback deltas, not the shorter render-frame intervals. The first
        // carried visit still sees the old child pose above E370's threshold.
        let (session, mut manager, mut owner) = carried_fixture();
        let id = owner.entity_id();
        assert_eq!(session.cache.level_terrain().unwrap().sea_level_raw(), -847);
        assert_eq!(
            session
                .cache
                .global_model(LEVEL_ONE_TYPE9_MODEL_ID)
                .unwrap()
                .radius,
            165
        );
        let entity = manager.ordinary_type9_selected_entity_mut(id).unwrap();
        entity.collision.state_flags_at_0x08 =
            crate::entity_collision_state::RetailStateWord::exact(0x06c2_1025);
        entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(0);
        entity.set_velocity_raw([0; 3]);
        let task_before = owner.visit;
        let parent_before = entity.attached_to;
        let mut effects = WorldFx::new();
        for (retail_tick, elapsed_micros, position, expected_timer) in [
            (0x479, 77_000, [0x6034, -633, -25_948], 0),
            (0x47d, 91_000, [0x6025, -1_843, -25_925], 91),
            (0x481, 79_000, [0x6025, -1_843, -25_925], 170),
            (0x485, 79_000, [0x6025, -1_843, -25_925], 249),
            (0x489, 78_000, [0x6025, -1_843, -25_925], 327),
            (0x48d, 72_000, [0x6025, -1_843, -25_925], 399),
            (0x490, 73_000, [0x6025, -1_843, -25_925], 472),
            (0x494, 73_000, [0x6025, -1_843, -25_925], 545),
        ] {
            manager
                .ordinary_type9_selected_entity_mut(id)
                .unwrap()
                .set_position_raw(position);
            let tick = tick_type9_carried_owner(
                &mut manager,
                owner,
                Type9CarriedProductionFrame {
                    resources: &session.cache,
                    world_fx: &mut effects,
                    notifications: &mut GameplayNotifications::new(),
                    elapsed_micros,
                    retail_tick,
                },
                &mut |_| panic!("retail detailed carrying skips scheduler RNG"),
            );
            assert!(
                matches!(tick.outcome, Type9CarriedProductionOutcome::Continuing {
                callback_elapsed_micros, callback_enabled: true, ..
            } if callback_elapsed_micros == elapsed_micros),
                "{retail_tick:x}: {:?}",
                tick.outcome
            );
            owner = tick.retained_owner.unwrap();
            let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
            assert_eq!(
                entity.surface_lifetime_timer_ms_at_0x48,
                RetailRuntimeValue::Known(expected_timer)
            );
            assert_eq!(
                entity.position_raw(),
                position,
                "carried master motion stays disabled"
            );
            assert_eq!(entity.attached_to, parent_before);
            assert_eq!(owner.visit, task_before);
            assert_eq!(
                entity.collision.animation_offset_at_0xb2,
                RetailRuntimeValue::Known(0)
            );
        }
        assert_eq!(
            *effects.entity_construction_state().0,
            0,
            "the observed carried window never reaches E370 random effects"
        );
        assert_eq!(effects.particle_count(), 0);
        assert!(effects.take_positional_sounds().is_empty());
    }

    #[v2k_test_support::retail_test]
    fn carried_surface_effects_share_scheduler_rng_and_materialize_before_sound() {
        // Controlled static-E370 branches beyond the capture's 545-ms window.
        // These seeds accept the bubble and subsequent sound; the coarse case
        // first consumes the two continuing scheduler words (4646, 2136).
        for (detailed, seed, expected_rng, bubble_position, bubble_velocity) in [
            (
                true,
                25,
                0x50a0_dd16,
                [1_255, -1_919, 3_091],
                [291, 400, 347],
            ),
            (
                false,
                1_411,
                0xa810_6742,
                [1_243, -1_910, 3_092],
                [71, 400, 239],
            ),
        ] {
            let (session, mut manager, owner) = carried_fixture();
            let id = owner.entity_id();
            let entity = manager.ordinary_type9_selected_entity_mut(id).unwrap();
            entity.collision.state_flags_at_0x08.overwrite(
                ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT
                    | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT
                    | DYING_STATE_BIT
                    | COMMON_MASTER_MOTION_REQUIRED_STATE_MASK,
                if detailed {
                    SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT
                } else {
                    0
                },
            );
            entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(2_480);
            entity.set_motion_raw([1_000, -2_000, 3_000], [1_000, 400, 0]);
            entity.set_rotation_heading_pitch_roll_raw([0; 3]);
            entity.physical_body_basis_q31 = RetailRuntimeValue::Unresolved;
            let mut effects = WorldFx::new();
            *effects.entity_construction_state().0 = seed;
            let tick = tick_type9_carried_owner(
                &mut manager,
                owner,
                Type9CarriedProductionFrame {
                    resources: &session.cache,
                    world_fx: &mut effects,
                    notifications: &mut GameplayNotifications::new(),
                    elapsed_micros: 20_000,
                    retail_tick: 0x500,
                },
                &mut |fx| u32::from(fx.next_shared_retail_random_u16()),
            );
            assert!(
                matches!(
                    tick.outcome,
                    Type9CarriedProductionOutcome::Continuing {
                        callback_elapsed_micros: 20_000,
                        ..
                    }
                ),
                "{detailed}: {:?}",
                tick.outcome
            );
            assert_eq!(*effects.entity_construction_state().0, expected_rng);
            let particles = effects.test_particles_in_virgin_birth_order();
            assert_eq!(particles.len(), 1);
            let particle = &particles[0];
            assert_eq!(particle.source_class, 42);
            assert_eq!(particle.owner_id, Some(id));
            assert_eq!(particle.source_entity_type_at_birth, Some(9));
            assert_eq!(
                particle.position.map(|v| (v * 256.0).round() as i32),
                bubble_position
            );
            assert_eq!(
                particle
                    .velocity
                    .map(|v| (v * 256.0 * (1_048_576.0 / 1_000_000.0)).round() as i32),
                bubble_velocity
            );
            assert_eq!(effects.pending_event_count(), 1);
            effects.process_pending();
            let sounds = effects.take_positional_sounds();
            assert_eq!(sounds.len(), 1);
            assert_eq!(sounds[0].sound_id, 106);
            assert_eq!(
                sounds[0].position,
                [1_000.0, -2_000.0, 3_000.0].map(|v| v / 256.0)
            );
            let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
            assert_eq!(
                entity.surface_lifetime_timer_ms_at_0x48,
                RetailRuntimeValue::Known(2_500)
            );
            assert!(tick.retained_owner.unwrap().authenticates(entity));
        }
    }

    #[v2k_test_support::retail_test]
    fn carried_surface_uses_scheduler_accumulator_cap_and_unit_delta_before_admission() {
        for (timer, accumulator, unit_delta, expected_delta, expected_timer) in [
            (0, 70_000, 0, 90_000, 90),
            (0, 200_000, 0, 125_000, 125),
            (4_999, 200_000, 1, 1, 4_999),
        ] {
            let (session, mut manager, owner) = carried_fixture();
            let id = owner.entity_id();
            let entity = manager.ordinary_type9_selected_entity_mut(id).unwrap();
            entity.collision.state_flags_at_0x08.overwrite(
                ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
                SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
            );
            entity.set_position_raw([1_000, -2_000, 3_000]);
            entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(timer);
            entity.collision.callback_scheduler_accumulator_us_at_0x6c =
                RetailRuntimeValue::Known(accumulator);
            entity.collision.scheduler_unit_delta_flag_at_0xb6 =
                RetailRuntimeValue::Known(unit_delta);
            let tick = tick_carried(&mut manager, owner, &session.cache, 20_000, &mut || {
                panic!("detailed scheduler RNG")
            });
            assert!(
                matches!(tick.outcome, Type9CarriedProductionOutcome::Continuing {
                callback_elapsed_micros, ..
            } if callback_elapsed_micros == expected_delta),
                "{:?}",
                tick.outcome
            );
            assert_eq!(
                manager
                    .iter_all()
                    .find(|entity| entity.id == id)
                    .unwrap()
                    .surface_lifetime_timer_ms_at_0x48,
                RetailRuntimeValue::Known(expected_timer)
            );
        }
    }

    #[v2k_test_support::retail_test]
    fn carried_surface_expiry_with_unresolved_release_inputs_blocks_before_scheduler_or_callback() {
        let (session, mut manager, owner) = carried_fixture();
        let id = owner.entity_id();
        let sea = session.cache.terrain().unwrap().sea_level_raw();
        let radius = session
            .cache
            .global_model(LEVEL_ONE_TYPE9_MODEL_ID)
            .unwrap()
            .radius;
        let entity = manager.ordinary_type9_selected_entity_mut(id).unwrap();
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT, 0);
        entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(4_980);
        entity.sub_a_propulsion_runtime = RetailRuntimeValue::Unresolved;
        let pos = entity.position_raw();
        entity.set_motion_raw(
            [
                pos[0],
                sea.wrapping_sub((radius >> 2) as i16).wrapping_sub(1),
                pos[2],
            ],
            entity.velocity_raw(),
        );
        let before = entity.collision.clone();
        let tick = tick_carried(&mut manager, owner, &session.cache, 20_000, &mut || {
            panic!("unsupported surface expiry must not consume RNG")
        });
        assert_eq!(
            tick.outcome,
            Type9CarriedProductionOutcome::Blocked {
                entity_id: id,
                reason: Type9CarriedProductionBlock::SurfaceLifecycleUnsupported,
            }
        );
        let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
        assert_eq!(entity.collision, before);
        assert_eq!(
            entity.surface_lifetime_timer_ms_at_0x48,
            RetailRuntimeValue::Known(4_980)
        );
    }
}
