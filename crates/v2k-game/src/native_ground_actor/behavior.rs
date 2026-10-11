//! Weighted C690 reentry and acquiring callback ownership.

use super::*;
use crate::{
    actor_task_dispatcher::{
        prepare_follow_beacons_following_runtime_task, ActorTaskRuntime,
        FollowBeaconsFollowingConstructorEffect,
    },
    actor_task_owner::ActorTaskVisit,
    entity::EntityManager,
    entity_behavior::{audited_behavior_style, behavior_program, BehaviorDescriptorIdentity},
    entity_collision_state::DYING_STATE_BIT,
    follow_beacons::*,
    search_attack_live::{
        apply_search_attack_acquisition_live, SearchAttackLiveAcquisitionOutcome,
        SearchAttackLiveHandoffRequirement, SearchAttackLiveSamePassAim,
    },
    world_fx::WorldFx,
};

pub(crate) enum ReselectionEntry {
    TaskResult,
    Impact,
}

pub(crate) fn reselect<P: NativeGroundActorProfile>(
    manager: &mut EntityManager,
    id: u32,
    tick: u32,
    world_fx: &mut WorldFx,
    resources: Option<&crate::resource_cache::ResourceCache>,
    entry: ReselectionEntry,
) -> Result<(), NativeGroundActorBlock> {
    use NativeGroundActorBlock as Block;
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(Block::Allocation)?;
    if !P::manager_authenticates(manager, id) {
        return Err(Block::Allocation);
    }
    let metadata = manager
        .type_runtime_metadata(P::ENTITY_TYPE)
        .cloned()
        .ok_or(Block::Metadata)?;
    if !P::metadata_authenticates(&metadata) {
        return Err(Block::Metadata);
    }
    let required = match entry {
        ReselectionEntry::TaskResult => 0x1000 | DYING_STATE_BIT,
        ReselectionEntry::Impact => DYING_STATE_BIT,
    };
    let flags = super::live::bits(entity, required)?;
    if matches!(entry, ReselectionEntry::TaskResult) && flags & 0x1000 != 0 {
        return Ok(());
    }
    if flags & DYING_STATE_BIT != 0 {
        return Err(Block::Runtime("dying reentry belongs to class12"));
    }
    let RetailRuntimeValue::Known(Some(previous)) = entity.current_behavior_context else {
        return Err(Block::Graph);
    };
    let needs = |rule| {
        P::CHOICES
            .iter()
            .any(|choice| BehaviorWeightRule::from_raw(choice.weight_rule_id) == Some(rule))
    };
    //425680 evaluates only rules present in the actual choice list. An
    // unrelated candidate or last-hit field cannot block an unused rule.
    let under_attack = if needs(BehaviorWeightRule::UnderAttack) {
        let RetailRuntimeValue::Known(last_hit) =
            entity.collision.last_hit_presentation_tick_at_0x34
        else {
            return Err(Block::Runtime("last hit tick"));
        };
        tick.wrapping_sub(last_hit) < 250 && (tick as i32) > 249
    } else {
        false
    };
    let mut owner = candidate(entity);
    // 22C10 ignores candidate relations, but reentry reads the owner's real
    // +60 relation. Only the constructor may authenticate this word as zero.
    owner.attached_entity_handle = entity.collision.recent_relation_id_at_0x60;
    let candidates: Vec<_> = manager
        .iter_all()
        .filter(|entity| entity.active)
        .map(candidate)
        .collect();
    let people = if needs(BehaviorWeightRule::PeopleNearby) {
        nearby::<P>(owner, &candidates, 0xc00).map_err(|_| Block::Runtime("people selector"))?
    } else {
        false
    };
    let player = if needs(BehaviorWeightRule::PlayerNearby) {
        nearby::<P>(owner, &candidates, 1).map_err(|_| Block::Runtime("player selector"))?
    } else {
        false
    };
    // Rule8 scans terrain objects within a quarter of the current axis.
    let furniture = if needs(BehaviorWeightRule::FurnitureNearby) {
        furniture_nearby(
            entity,
            resources.ok_or(Block::Runtime("furniture selector resources"))?,
        )?
    } else {
        false
    };
    let selection = select_initial_behavior(
        P::CHOICES,
        |rule| match rule {
            BehaviorWeightRule::Always => 1,
            BehaviorWeightRule::UnderAttack => i32::from(under_attack),
            BehaviorWeightRule::PeopleNearby => i32::from(people),
            BehaviorWeightRule::PlayerNearby => i32::from(player),
            BehaviorWeightRule::FurnitureNearby => i32::from(furniture),
            _ => unreachable!(),
        },
        || u32::from(world_fx.next_shared_retail_random_u16()),
    )
    .map_err(|_| Block::Metadata)?
    .ok_or(Block::Metadata)?;
    let context = previous
        .reselect_named_type_default(
            selection.program,
            selection.program.initial_style_table_index_raw,
            selection.program.initial_style,
        )
        .ok_or(Block::Graph)?;
    let entity = manager.entity_mut(id).unwrap();
    if !P::publish_acquiring(entity, &metadata, selection, context, &mut || {
        u32::from(world_fx.next_shared_retail_random_u16())
    }) {
        return Err(Block::Runtime("initializer fallback"));
    }
    Ok(())
}

/// `425680`'s rule8: any terrain object within a quarter of the entity's
/// current strict axis, with no kind filter. The scan draws no RNG.
pub(crate) fn furniture_nearby(
    entity: &Entity,
    resources: &crate::resource_cache::ResourceCache,
) -> Result<bool, NativeGroundActorBlock> {
    let RetailRuntimeValue::Known(axis) = entity.actor_common_axis_descriptor else {
        return Err(NativeGroundActorBlock::Runtime("furniture selector axis"));
    };
    let terrain = resources
        .level_terrain()
        .ok_or(NativeGroundActorBlock::Runtime(
            "furniture selector terrain",
        ))?;
    Ok(crate::trash_furniture::find_furniture(
        terrain,
        resources.terrain_objects(),
        entity.position_raw(),
        axis.strict_axis_limit_raw / 4,
        -1,
    )
    .is_some())
}

pub(crate) fn secondary<P: NativeGroundActorProfile>(
    manager: &mut EntityManager,
    id: u32,
    dt: u32,
    retail_tick: u32,
    world_fx: &mut WorldFx,
) -> Result<(), NativeGroundActorBlock> {
    use NativeGroundActorBlock as Block;
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(Block::Allocation)?;
    let Some(task) = entity.actor_task_state(ActorTaskSlot::Secondary) else {
        return Ok(());
    };
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(Block::Graph);
    };
    let BehaviorDescriptorIdentity::Named(program) = context.descriptor() else {
        return Err(Block::Graph);
    };
    if matches!(task, ActorTaskRuntime::FollowBeaconAcquisition(_)) && program.class_id == 33 {
        return acquire_beacon::<P>(manager, id, world_fx);
    }
    if matches!(task, ActorTaskRuntime::CaptureBeaconAcquisition)
        && P::CAPTURE_POLICY == NativeCapturePolicy::Transport
    {
        return crate::native_actor_capture::carry_tasks::acquire(manager, id, world_fx)
            .map_err(|error| Block::CaptureTask(Box::new(error)));
    }
    if !matches!(task, ActorTaskRuntime::TargetAcquisition(_)) {
        return Err(Block::Graph);
    }
    if program.class_id == 10 {
        let metadata = manager
            .type_runtime_metadata(P::ENTITY_TYPE)
            .cloned()
            .ok_or(Block::Metadata)?;
        // Acquisition cannot call the mover: widen its Infallible error arm
        // to the shared mover-bearing diagnostic without inventing a failure.
        return super::run_away::acquire(manager, id, &metadata, world_fx).map_err(|error| {
            super::run_away::map_ground(match error {
                super::run_away::NativeRunAwayBlock::Allocation => {
                    super::run_away::NativeRunAwayBlock::Allocation
                }
                super::run_away::NativeRunAwayBlock::Graph => {
                    super::run_away::NativeRunAwayBlock::Graph
                }
                super::run_away::NativeRunAwayBlock::Runtime(reason) => {
                    super::run_away::NativeRunAwayBlock::Runtime(reason)
                }
                super::run_away::NativeRunAwayBlock::Acquisition(reason) => {
                    super::run_away::NativeRunAwayBlock::Acquisition(reason)
                }
                super::run_away::NativeRunAwayBlock::Mover(never) => match never {},
            })
        });
    }
    if program.class_id == 9 {
        return P::acquire_capture(manager, id, retail_tick, world_fx);
    }
    let result = apply_search_attack_acquisition_live(
        manager,
        id,
        dt,
        world_fx,
        if program.class_id == 7 {
            SearchAttackLiveHandoffRequirement::RequiredClass7Variant0
        } else {
            SearchAttackLiveHandoffRequirement::Optional
        },
        SearchAttackLiveSamePassAim::Skip,
    );
    match result {
        SearchAttackLiveAcquisitionOutcome::Blocked { reason, .. } => {
            Err(Block::Acquisition(reason))
        }
        SearchAttackLiveAcquisitionOutcome::Applied { .. } => {
            let current = manager
                .iter_all()
                .find(|entity| entity.id == id)
                .ok_or(Block::Allocation)?;
            let RetailRuntimeValue::Known(Some(context)) = current.current_behavior_context else {
                return Err(Block::Graph);
            };
            if context.active_style().style_address() == 0x004c7a98
                && !super::search::pursuing_graph_authenticates::<P>(current)
            {
                return Err(Block::Runtime("ADE0 task publication"));
            }
            Ok(())
        }
        SearchAttackLiveAcquisitionOutcome::NotApplicable => Err(Block::Graph),
    }
}

fn acquire_beacon<P: NativeGroundActorProfile>(
    manager: &mut EntityManager,
    id: u32,
    world_fx: &mut WorldFx,
) -> Result<(), NativeGroundActorBlock> {
    use NativeGroundActorBlock as Block;
    let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
    let task_id = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Secondary)
        .ok_or(Block::Graph)?;
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Secondary,
        task_id,
    };
    let prefix = entity
        .actor_tasks
        .begin_exact_visit_with(visit, |runtime| {
            let ActorTaskRuntime::FollowBeaconAcquisition(task) = runtime else {
                unreachable!()
            };
            task.before_callback()
        })
        .ok_or(Block::Graph)?;
    entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(CommonAxisDescriptor {
        strict_axis_limit_raw: prefix.route_range().raw(),
        raw_word_at_0x04: 0x100,
    });
    let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
    let candidates: Vec<_> = manager
        .iter_all()
        .map(|entity| FollowBeaconEntityRef {
            id: entity.id,
            position_raw: entity.position_raw(),
            capability_flags: RetailRuntimeValue::Known(entity.capability_flags),
            score_at_0x88: entity
                .authored_follow_beacon_priority_raw
                .map_or(RetailRuntimeValue::Unresolved, RetailRuntimeValue::Known),
            collision: &entity.collision,
        })
        .collect();
    let selected = select_follow_beacon(
        FollowBeaconSelectionRequest {
            prefix,
            owner: FollowBeaconOwnerRef {
                id,
                position_raw: entity.position_raw(),
                collision: &entity.collision,
            },
            candidates_in_intrusive_order: &candidates,
        },
        || u32::from(world_fx.next_shared_retail_random_u16()),
    );
    let metadata = manager
        .type_runtime_metadata(P::ENTITY_TYPE)
        .cloned()
        .ok_or(Block::Metadata)?;
    let entity = manager.entity_mut(id).unwrap();
    let result = match selected {
        Ok(FollowBeaconSelection::Success { target }) => {
            publish_following(entity, &metadata, target.id, world_fx)
        }
        Ok(FollowBeaconSelection::TaggedNoPositiveScore { .. }) => Ok(()),
        Err(_) => Err(Block::Runtime("beacon selector")),
    };
    if entity
        .actor_tasks
        .wrapper_flags(visit.task_id)
        .is_none_or(|flags| !flags.in_callback)
    {
        return Err(Block::Graph);
    }
    // AFD0 clears the currently executing Secondary on successful handoff.
    // finish_exact_visit reports survival, not whether unwind succeeded.
    // Retiring that old wrapper is the expected successful transition.
    entity.actor_tasks.finish_exact_visit(visit);
    result
}

fn publish_following(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    target_id: u32,
    world_fx: &mut WorldFx,
) -> Result<(), NativeGroundActorBlock> {
    use NativeGroundActorBlock as Block;
    let RetailRuntimeValue::Known(Some(previous)) = entity.current_behavior_context else {
        return Err(Block::Graph);
    };
    let program = behavior_program(33).unwrap();
    let style = *audited_behavior_style(33, 1).unwrap();
    let context = BehaviorContextRuntime::named_audited(
        program,
        1,
        previous.choice_list_source(),
        RetailRuntimeValue::Known(Some(target_id)),
        previous.auxiliary_word_at_0x0c(),
        style,
    )
    .ok_or(Block::Graph)?;
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(context));
    let id = entity.id;
    let position = entity.position_raw();
    let Entity {
        actor_tasks,
        sub_a_propulsion_runtime,
        sub_h_external_frame_runtime,
        ..
    } = entity;
    let RetailRuntimeValue::Known(Some(a)) = sub_a_propulsion_runtime else {
        return Err(Block::Runtime("Sub-A"));
    };
    let RetailRuntimeValue::Known(Some(h)) = sub_h_external_frame_runtime else {
        return Err(Block::Runtime("Sub-H"));
    };
    let result = apply_follow_beacons_following_task_setup(actor_tasks, target_id, |preparation| {
        prepare_follow_beacons_following_runtime_task(preparation, id, position, metadata).map(
            |prepared| {
                prepared.apply_suffix(
                    || u32::from(world_fx.next_shared_retail_random_u16()),
                    |effect| match effect {
                        FollowBeaconsFollowingConstructorEffect::Generic(
                            SharedGenericConstructorEffect::WriteSubHState08 { value },
                        ) => h.set_enabled(value != 0),
                        FollowBeaconsFollowingConstructorEffect::Generic(
                            SharedGenericConstructorEffect::WriteSubADirection {
                                direction_multiplier,
                            },
                        ) => a.set_direction_multiplier(direction_multiplier),
                        FollowBeaconsFollowingConstructorEffect::Generic(
                            SharedGenericConstructorEffect::WriteSubATargetSpeed {
                                target_speed_raw,
                                ..
                            },
                        ) => a.apply_shared_initializer_target_speed_write(target_speed_raw),
                        FollowBeaconsFollowingConstructorEffect::ApplyFixedSubATargetSpeed {
                            suffix,
                            ..
                        } => {
                            if let Some(speed) = suffix.sub_a_target_speed_raw() {
                                a.apply_shared_initializer_target_speed_write(speed);
                            }
                        }
                    },
                )
            },
        )
    });
    if result.is_err() {
        entity.publish_behavior_initializer_failure_fallback(context);
        return Err(Block::Runtime("following initializer fallback"));
    }
    Ok(())
}
