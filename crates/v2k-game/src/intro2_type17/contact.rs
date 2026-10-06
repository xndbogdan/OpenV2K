//! Late11AD0/12CF0 static contact for shared Type17 living and Class12 allocations.
//!
//! This runs after particles, outside12DA0 and its body-basis rebuild. It
//! preserves the original selected plane across02CA0 and delivers11760's
//! collision packet to the static cell before the actor.

use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    damage::{velocity_delta_impact_raw, DamageDeliveryRecord, DamagePacket},
    entity_collision_state::{
        active_model_slot_from_state_flags, ACTIVE_MODEL_SLOT_HIGH_STATE_BIT,
        ACTIVE_MODEL_SLOT_LOW_STATE_BIT, REMOTE_OWNED_STATE_BIT,
    },
    follow_beacons::static_contact::{
        plan_wander_private_static_contact, FollowBeaconsStaticContactTopology,
        WanderPrivateStaticContactRequest,
    },
    intro2_common_dying::{Intro2CommonDyingBlock, Intro2CommonDyingOwner},
    intro2_contacts::Intro2ContactFrame,
    intro2_radial::Intro2RadialTaskCustody,
    live_actor_checked_damage::{
        apply_live_actor_checked_damage, LiveActorDamageEntry, LiveActorDamageError,
        LiveActorDamageOutcome, LiveActorDamageRequest, LiveActorDeathResult,
    },
    static_contact::{
        apply_contact_response_raw, scan_deepest_static_contact, static_contact_subject_eligible,
        StaticContactError, StaticContactQuery, StaticModelContact,
    },
    static_damage::StaticDamageOutcome,
    static_damage_live::{resolve_current_static_damage_target, CurrentStaticDamageLookupError},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type17ContactBlock {
    Runtime(&'static str),
    UnsupportedStyle(u32),
    Scan(StaticContactError),
    StaticLookup(CurrentStaticDamageLookupError),
    UnsupportedStaticKind(u32),
    Damage(LiveActorDamageError<Intro2CommonDyingBlock, Intro2CommonDyingOwner>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Type17ContactApplied {
    pub contact: StaticModelContact,
    pub position_before_raw: [i16; 3],
    pub position_after_raw: [i16; 3],
    pub velocity_before_raw: [i16; 3],
    pub velocity_after_raw: [i16; 3],
    pub impact_raw: i32,
    pub static_damage: Option<StaticDamageOutcome>,
    pub actor_damage: Option<LiveActorDamageOutcome<Intro2CommonDyingOwner>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type17ContactOutcome {
    Ineligible,
    Miss,
    Applied(Type17ContactApplied),
    Blocked {
        reason: Type17ContactBlock,
        committed_prefix: bool,
    },
}

pub fn resolve_type17_static_contact(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
) -> Type17ContactOutcome {
    let mut committed = false;
    match resolve(frame, id, &mut committed) {
        Ok(outcome) => outcome,
        Err(reason) => {
            if committed {
                // A completed death publication has already replaced the living
                // owner. The scheduler parks whichever exact prefix survived.
                frame.actor_tasks.park_intro2_type17_external_prefix(id);
            }
            Type17ContactOutcome::Blocked {
                reason,
                committed_prefix: committed,
            }
        }
    }
}

fn resolve(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
    committed: &mut bool,
) -> Result<Type17ContactOutcome, Type17ContactBlock> {
    use Type17ContactBlock as Block;
    let entity = frame
        .entities
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(Block::Runtime("allocation"))?;
    if entity.intro2_type17_runtime.is_none() {
        return Ok(Type17ContactOutcome::Ineligible);
    }
    if !type17_manager_allocation_authenticates(frame.entities, id) {
        return Err(Block::Runtime("native allocation"));
    }
    let selector = bits(
        entity,
        ACTIVE_MODEL_SLOT_LOW_STATE_BIT | ACTIVE_MODEL_SLOT_HIGH_STATE_BIT,
    )?;
    let model_id = entity
        .model_in_slot(active_model_slot_from_state_flags(selector))
        .ok_or(Block::Runtime("active model"))?;
    let model = frame
        .resources
        .global_model(model_id)
        .ok_or(Block::Runtime("active model resource"))?;
    match static_contact_subject_eligible(&entity.collision, model.collision_radius_raw) {
        RetailRuntimeValue::Known(false) => return Ok(Type17ContactOutcome::Ineligible),
        RetailRuntimeValue::Unresolved => return Err(Block::Runtime("static scan eligibility")),
        RetailRuntimeValue::Known(true) => {}
    }
    let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
        return Err(Block::Runtime("physical basis"));
    };
    let contact = scan_deepest_static_contact(StaticContactQuery {
        terrain: frame
            .resources
            .level_terrain()
            .ok_or(Block::Runtime("terrain"))?,
        terrain_objects: frame
            .resources
            .terrain_objects()
            .ok_or(Block::Runtime("terrain objects"))?,
        model_pool: &*frame.resources,
        tick: frame.retail_tick,
        active_model: model,
        active_model_to_world_basis: basis
            .orientation_world_from_model()
            .map(|row| row.map(f64::from)),
        active_anim_vars: &entity.presentation_anim_vars(frame.retail_tick),
        position_raw: entity.position_raw(),
    })
    .map_err(Block::Scan)?;
    let Some(contact) = contact else {
        return Ok(Type17ContactOutcome::Miss);
    };
    // A8B0 is reached only after the static scan hits. The living styles
    // and Class12 have null+1C. Class12's2015 disable mask still excludes400.
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(Block::Runtime("current style"));
    };
    if !matches!(
        context.active_style().style_address(),
        0x4c7b28
            | 0x4c7b70
            | 0x4c7ff0
            | 0x4c8038
            | 0x4c7618
            | 0x4c7660
            | 0x4c7ed0
            | 0x4c8080
            | 0x4c80c8
            | 0x4c8110
            | 0x4c8158
    ) {
        return Err(Block::UnsupportedStyle(
            context.active_style().style_address(),
        ));
    }
    if !frame
        .actor_tasks
        .prepare_native_actor_mutation(frame.entities, id)
    {
        return Err(Block::Runtime("completed contact owner"));
    }
    apply_contact(frame, id, contact, committed).map(Type17ContactOutcome::Applied)
}

fn apply_contact(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
    contact: StaticModelContact,
    committed: &mut bool,
) -> Result<Type17ContactApplied, Type17ContactBlock> {
    use Type17ContactBlock as Block;
    let metadata = frame
        .entities
        .type_runtime_metadata(17)
        .ok_or(Block::Runtime("metadata"))?;
    authenticate_metadata(metadata).map_err(|_| Block::Runtime("metadata"))?;
    let type_record = frame
        .resources
        .global_entity_type(17)
        .ok_or(Block::Runtime("type record"))?;
    let entity = frame
        .entities
        .entity_mut(id)
        .ok_or(Block::Runtime("allocation"))?;
    if type_record.raw_header[0x8a..0x8c] != [0, 0]
        || entity.capability_flags != 8
        || entity.collision.default_state_flags_at_0xc8 != RetailRuntimeValue::Known(0x39)
    {
        return Err(Block::Runtime("Type17 static callback policy"));
    }
    let hook = contact_task_hook(entity)?;
    if let Type17StaticTaskHook::WanderPrivate(private_state) = hook {
        let primary = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
            return Err(Block::Runtime("Sub-A runtime"));
        };
        let plan = plan_wander_private_static_contact(
            WanderPrivateStaticContactRequest {
                private_state,
                controlled_position_raw: entity.position_raw(),
                heading_raw: entity.heading_raw(),
                topology: RetailRuntimeValue::Known(FollowBeaconsStaticContactTopology {
                    sub_i: false,
                    sub_a: Some(sub_a),
                    sub_f: false,
                    sub_g: false,
                }),
            },
            || u32::from(frame.world_fx.next_shared_retail_random_u16()),
        )
        .expect("authenticated Type17 A/B/C/D/H/J topology");
        *committed = true;
        match entity.actor_tasks.task_state_mut(primary).unwrap() {
            ActorTaskRuntime::SharedRetarget(task) => {
                task.apply_static_contact_private_state(plan.private_state_after)
            }
            ActorTaskRuntime::FollowBeaconsFollowing(task)
            | ActorTaskRuntime::CapturePeopleFollowing(task) => {
                let mut stage = task.stage_callback();
                *stage.private_state_mut() = plan.private_state_after;
                stage.commit(task);
            }
            ActorTaskRuntime::CapturePeoplePursuit(task) => {
                let mut stage = task.stage_callback();
                *stage.private_state_mut() = plan.private_state_after;
                stage.commit(task);
            }
            ActorTaskRuntime::RunAway(task) => {
                let mut stage = task.stage_callback();
                *stage.private_state_mut() = plan.private_state_after;
                stage.commit(task);
            }
            _ => unreachable!("preflight retains exact task family"),
        }
        entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(plan.sub_a_runtime);
        entity.set_heading_raw(plan.heading_raw);
    }
    // Class12's404120 retains05FF0's null+20: no task, Sub-A or RNG writes.
    // These styles have null+1C and their effective policy excludes400. The common
    // D920 callback therefore reaches11760 without replacing the graph. The
    // body basis belongs to the next DCA0/E870 rebuild, not this contact hook.
    let position_before_raw = entity.position_raw();
    let velocity_before_raw = entity.velocity_raw();
    let mut position_after_raw = position_before_raw;
    let mut velocity_after_raw = velocity_before_raw;
    apply_contact_response_raw(&mut position_after_raw, &mut velocity_after_raw, contact);
    *committed = true;
    entity.set_motion_raw(position_after_raw, velocity_after_raw);
    let impact_raw = if bits(entity, REMOTE_OWNED_STATE_BIT)? == 0 {
        velocity_delta_impact_raw(velocity_before_raw, velocity_after_raw, entity.mass_raw)
    } else {
        0
    };
    let mut applied = Type17ContactApplied {
        contact,
        position_before_raw,
        position_after_raw,
        velocity_before_raw,
        velocity_after_raw,
        impact_raw,
        static_damage: None,
        actor_damage: None,
    };
    if impact_raw != 0 {
        let delivery = DamageDeliveryRecord {
            packet: DamagePacket::collision(impact_raw),
            source_entity_type_raw: 17,
            owner_handle: id,
        };
        if let Some(target) = resolve_current_static_damage_target(frame.resources, contact.cell)
            .map_err(Block::StaticLookup)?
        {
            let outcome = frame
                .static_damage
                .submit_hit(target, delivery.packet, &mut || {
                    frame.world_fx.next_shared_retail_random_u16()
                });
            match outcome {
                StaticDamageOutcome::UnsupportedKind { kind_index } => {
                    return Err(Block::UnsupportedStaticKind(kind_index))
                }
                StaticDamageOutcome::BurnedKind10Transition { cell, .. } => {
                    frame.resources.apply_burned_kind_10_transition(cell);
                }
                StaticDamageOutcome::ImmediateBurn { cell, .. } => {
                    crate::static_terrain_burn::apply_immediate_static_burn(
                        cell,
                        frame.resources,
                        frame.world_fx,
                    )
                    .map_err(|error| {
                        Block::StaticLookup(
                            crate::static_damage_live::CurrentStaticDamageLookupError::BurnCallback(
                                error,
                            ),
                        )
                    })?;
                }
                _ => {}
            }
            applied.static_damage = Some(outcome);
        }
        let result = apply_live_actor_checked_damage(
            frame.entities,
            frame.world_fx,
            LiveActorDamageRequest {
                ratio_numerator: 0,
                ratio_denominator: 0,
                entity_id: id,
                delivery,
                entry: LiveActorDamageEntry::Checked,
                feedback: Some(crate::live_actor_checked_damage::LiveActorDamageFeedback {
                    notifications: frame.notifications,
                    retail_tick: frame.retail_tick,
                }),
            },
            |manager, fx, feedback| {
                let feedback = feedback.ok_or(Intro2CommonDyingBlock::Runtime(
                    "capture static death context",
                ))?;
                super::capture::publish_type17_standard_death(
                    manager,
                    id,
                    &mut super::capture::CaptureContext {
                        tasks: frame.actor_tasks,
                        world_fx: fx,
                        notifications: feedback.notifications,
                        retail_tick: feedback.retail_tick,
                        result_screen:
                            crate::main_base_type9_abort::MainBaseType9ResultScreenState::NotShown,
                        hive_dying: Default::default(),
                    },
                )
                .map_err(|block| Intro2CommonDyingBlock::Runtime(block.reason))
                .map(|owner| LiveActorDeathResult {
                    returned_nonzero: owner.is_some(),
                    publication: owner,
                })
            },
        );
        match result {
            Ok(result) => {
                if let Some(owner) = result.death_publication {
                    frame.actor_tasks.register_intro2_common_dying(owner);
                }
                applied.actor_damage = Some(result);
            }
            Err(error) => {
                if let Some(owner) = error.death_publication {
                    frame.actor_tasks.register_intro2_common_dying(owner);
                }
                return Err(Block::Damage(error));
            }
        }
    }
    Ok(applied)
}

fn bits(entity: &Entity, mask: u32) -> Result<u32, Type17ContactBlock> {
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(bits) => Ok(bits),
        RetailRuntimeValue::Unresolved => Err(Type17ContactBlock::Runtime("contact state bits")),
    }
}

enum Type17StaticTaskHook {
    Null,
    WanderPrivate(crate::wander_near_location::WanderNearPrivateState),
}

fn contact_task_hook(entity: &Entity) -> Result<Type17StaticTaskHook, Type17ContactBlock> {
    use ActorTaskRuntime as Task;
    use Type17ContactBlock as Block;
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(Block::Runtime("current style"));
    };
    let primary = entity.actor_task_state(ActorTaskSlot::Primary);
    let secondary = entity.actor_task_state(ActorTaskSlot::Secondary);
    if entity.actor_task_state(ActorTaskSlot::Tertiary).is_some() {
        return Err(Block::Runtime("contact task graph"));
    }
    //2050/2100 call05F80/05FF0 and leave task+20 null.02B10,
    //03650,03B70 and03E20 each explicitly replace it with02CA0.
    match (context.active_style().style_address(), primary, secondary) {
        (0x4c7b28, Some(Task::SharedRetarget(task)), Some(Task::FollowBeaconAcquisition(_))) => {
            Ok(Type17StaticTaskHook::WanderPrivate(task.private_state()))
        }
        (
            0x4c7ff0 | 0x4c7618,
            Some(Task::SharedRetarget(task)),
            Some(Task::TargetAcquisition(_)),
        ) => Ok(Type17StaticTaskHook::WanderPrivate(task.private_state())),
        (0x4c7b70, Some(Task::FollowBeaconsFollowing(task)), None) => {
            Ok(Type17StaticTaskHook::WanderPrivate(task.private_state()))
        }
        (0x4c8038 | 0x4c8080, Some(Task::CapturePeoplePursuit(task)), None) => {
            Ok(Type17StaticTaskHook::WanderPrivate(task.private_state()))
        }
        (0x4c80c8, Some(Task::SharedRetarget(task)), Some(Task::CaptureBeaconAcquisition)) => {
            Ok(Type17StaticTaskHook::WanderPrivate(task.private_state()))
        }
        (0x4c8110, Some(Task::CapturePeopleFollowing(task)), None) => {
            Ok(Type17StaticTaskHook::WanderPrivate(task.private_state()))
        }
        (0x4c8158, Some(Task::SharedRetarget(task)), None) => {
            Ok(Type17StaticTaskHook::WanderPrivate(task.private_state()))
        }
        (0x4c7660, Some(Task::RunAway(task)), None) => {
            Ok(Type17StaticTaskHook::WanderPrivate(task.private_state()))
        }
        (0x4c7ed0, Some(Task::CommonDying(_)), None) => Ok(Type17StaticTaskHook::Null),
        _ => Err(Block::Runtime("contact task graph")),
    }
}

#[cfg(test)]
#[path = "contact_tests.rs"]
mod tests;
