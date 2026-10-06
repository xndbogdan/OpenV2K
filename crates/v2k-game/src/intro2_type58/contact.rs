//! Native Type58 12CF0 -> A8B0 -> D920 -> 11760 static-object contact.
//!
//! Follow/Search retain their Primary's02CA0 hook and current task graph.
//! TrashFurniture's task hook is null; its style+1C runs C890/C690 before
//! the physical tail. Class12 has null task/style hooks. The selected contact
//! plane survives every admitted callback.

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
    intro2_common_dying::{
        publish_intro2_common_standard_death, Intro2CommonDyingBlock, Intro2CommonDyingOwner,
    },
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
    world_fx::WorldFx,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2Type58ContactBlock {
    Runtime(&'static str),
    UnsupportedStyle(u32),
    Scan(StaticContactError),
    StaticLookup(CurrentStaticDamageLookupError),
    UnsupportedStaticKind(u32),
    Behavior(Intro2Type58Block),
    Damage(LiveActorDamageError<Intro2CommonDyingBlock, Intro2CommonDyingOwner>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Intro2Type58ContactApplied {
    pub contact: StaticModelContact,
    pub furniture_damage: Option<StaticDamageOutcome>,
    pub position_before_response_raw: [i16; 3],
    pub position_after_response_raw: [i16; 3],
    pub velocity_before_response_raw: [i16; 3],
    pub velocity_after_response_raw: [i16; 3],
    pub collision_impact_raw: i32,
    pub collision_static_damage: Option<StaticDamageOutcome>,
    pub actor_damage: Option<LiveActorDamageOutcome<Intro2CommonDyingOwner>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2Type58ContactOutcome {
    Ineligible,
    Miss,
    Applied(Intro2Type58ContactApplied),
    Blocked {
        reason: Intro2Type58ContactBlock,
        committed_prefix: bool,
    },
}

pub fn resolve_intro2_type58_static_contact(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
) -> Intro2Type58ContactOutcome {
    let mut committed_prefix = false;
    match resolve(frame, id, &mut committed_prefix) {
        Ok(outcome) => outcome,
        Err(reason) => {
            retain_failed_prefix(frame, id, committed_prefix);
            Intro2Type58ContactOutcome::Blocked {
                reason,
                committed_prefix,
            }
        }
    }
}

fn retain_failed_prefix(frame: &mut Intro2ContactFrame<'_>, id: u32, committed: bool) {
    if committed {
        // A completed class12 publication outranks the living failure receipt.
        // Its source callback and any later failure already retain that owner.
        if let Ok(owner) = Intro2Type58Owner::adopt_blocked_prefix(frame.entities, id) {
            frame.actor_tasks.register_intro2_type58(owner);
        }
        // Retain the current scheduler owner even when11760 has already
        // published Class12. No later pair or scheduler visit may replay it.
        frame
            .actor_tasks
            .park_native_contact_prefix(frame.entities, id);
    }
}

fn resolve(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
    committed: &mut bool,
) -> Result<Intro2Type58ContactOutcome, Intro2Type58ContactBlock> {
    use Intro2Type58ContactBlock as Block;
    let entity = frame
        .entities
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(Block::Runtime("allocation"))?;
    if !intro2_type58_allocation_authenticates(entity) {
        return Ok(Intro2Type58ContactOutcome::Ineligible);
    }
    if !type58_manager_allocation_authenticates(frame.entities, id) {
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
        RetailRuntimeValue::Known(false) => return Ok(Intro2Type58ContactOutcome::Ineligible),
        RetailRuntimeValue::Unresolved => return Err(Block::Runtime("static scan eligibility")),
        RetailRuntimeValue::Known(true) => {}
    }
    if !frame
        .actor_tasks
        .prepare_native_actor_mutation(frame.entities, id)
    {
        return Err(Block::Runtime("current completed contact owner"));
    }
    contact_task_hook(entity)?;
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
        return Ok(Intro2Type58ContactOutcome::Miss);
    };
    apply_selected_contact(frame, id, contact, committed).map(Intro2Type58ContactOutcome::Applied)
}

fn bits(entity: &Entity, mask: u32) -> Result<u32, Intro2Type58ContactBlock> {
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(bits) => Ok(bits),
        RetailRuntimeValue::Unresolved => {
            Err(Intro2Type58ContactBlock::Runtime("contact state bits"))
        }
    }
}

/// Execute the callback and physical tail using the original selected plane.
/// This is distinct from selecting contact geometry and requires one of the
/// authenticated current task graphs below.
fn apply_selected_contact(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
    contact: StaticModelContact,
    committed: &mut bool,
) -> Result<Intro2Type58ContactApplied, Intro2Type58ContactBlock> {
    use Intro2Type58ContactBlock as Block;
    let metadata = frame
        .entities
        .type_runtime_metadata(58)
        .cloned()
        .ok_or(Block::Runtime("metadata"))?;
    authenticate_metadata(&metadata).map_err(|_| Block::Runtime("metadata"))?;
    let type_record = frame
        .resources
        .global_entity_type(58)
        .ok_or(Block::Runtime("type record"))?;
    let entity = frame
        .entities
        .entity_mut(id)
        .ok_or(Block::Runtime("allocation"))?;
    // 12CF0 and 11760 both read +8A. The canonical Type58 field is null;
    // nonzero gain-scaled contact audio is not silently treated as silence.
    if type_record.raw_header[0x8a..0x8c] != [0, 0]
        || entity.capability_flags != 8
        || entity.collision.default_state_flags_at_0xc8 != RetailRuntimeValue::Known(0x39)
    {
        return Err(Block::Runtime("Type58 static callback policy"));
    }
    // 27E20's pickup modes require player capability1. Living effective39 and
    // Class12 effective28 (39 & ~2015) exclude D920's400 generic crush branch.
    let hook = contact_task_hook(entity)?;
    let furniture_damage = match hook {
        Type58StaticTaskHook::Null => None,
        Type58StaticTaskHook::WanderPrivate(private_state) => {
            apply_wander_private_hook(entity, private_state, frame.world_fx)?;
            *committed = true;
            // Follow/Search style+1C is null: no C890 hit or C690 reselection.
            None
        }
        Type58StaticTaskHook::Furniture => {
            // 405FF0 zeroes task+20 and401CA0 leaves it null. C890 instead
            // ignores the static filter/program result and calls C690 directly,
            // including NoDamage, ChanceRejected, Duplicate or a changed cell.
            let damage = deliver_static(
                frame,
                contact.cell,
                DamagePacket {
                    channels: [1, 0],
                    amounts_raw: [40000, 0],
                },
            )?;
            *committed = true;
            super::behavior::reselect(
                frame.entities,
                id,
                frame.resources,
                frame.world_fx,
                super::behavior::ReselectionEntry::DirectCallback,
            )
            .map_err(Block::Behavior)?;
            let owner = Intro2Type58Owner::adopt(frame.entities, id).map_err(Block::Behavior)?;
            frame.actor_tasks.register_intro2_type58(owner);
            damage
        }
    };

    // D920 re-resolves the actor after any style initializer. Keep the old
    // contact plane/cell, but capture motion and mass only now for11760.
    let entity = frame
        .entities
        .entity_mut(id)
        .ok_or(Block::Runtime("post-static-callback allocation"))?;
    let before_position = entity.position_raw();
    let before_velocity = entity.velocity_raw();
    let mut position = before_position;
    let mut velocity = before_velocity;
    apply_contact_response_raw(&mut position, &mut velocity, contact);
    entity.set_motion_raw(position, velocity);
    *committed = true;
    let impact = if bits(entity, REMOTE_OWNED_STATE_BIT)? == 0 {
        velocity_delta_impact_raw(before_velocity, velocity, entity.mass_raw)
    } else {
        0
    };
    let mut applied = Intro2Type58ContactApplied {
        contact,
        furniture_damage,
        position_before_response_raw: before_position,
        position_after_response_raw: position,
        velocity_before_response_raw: before_velocity,
        velocity_after_response_raw: velocity,
        collision_impact_raw: impact,
        collision_static_damage: None,
        actor_damage: None,
    };
    if impact != 0 {
        let delivery = DamageDeliveryRecord {
            packet: DamagePacket::collision(impact),
            source_entity_type_raw: 58,
            owner_handle: id,
        };
        applied.collision_static_damage = deliver_static(frame, contact.cell, delivery.packet)?;
        let result = apply_live_actor_checked_damage(
            frame.entities,
            frame.world_fx,
            LiveActorDamageRequest {
                ratio_numerator: 0,
                ratio_denominator: 0,
                feedback: None,
                entity_id: id,
                delivery,
                entry: LiveActorDamageEntry::Checked,
            },
            |manager, fx, _feedback| {
                publish_intro2_common_standard_death(manager, id, fx).map(|owner| {
                    LiveActorDeathResult {
                        returned_nonzero: owner.is_some(),
                        publication: owner,
                    }
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

enum Type58StaticTaskHook {
    Null,
    Furniture,
    WanderPrivate(crate::wander_near_location::WanderNearPrivateState),
}

fn contact_task_hook(entity: &Entity) -> Result<Type58StaticTaskHook, Intro2Type58ContactBlock> {
    use ActorTaskRuntime as Task;
    use Intro2Type58ContactBlock as Block;
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(Block::Runtime("current style"));
    };
    let style = context.active_style().style_address();
    if !matches!(
        style,
        0x4c7738 | 0x4c7b28 | 0x4c7b70 | 0x4c7a50 | 0x4c7a98 | 0x4c7ed0
    ) {
        return Err(Block::UnsupportedStyle(style));
    }
    if ActorTaskSlot::IN_RETAIL_TICK_ORDER
        .into_iter()
        .filter_map(|slot| entity.actor_tasks.task_in_slot(slot))
        .any(|task| {
            entity
                .actor_tasks
                .wrapper_flags(task)
                .is_none_or(|flags| !flags.alive || flags.in_callback)
        })
    {
        return Err(Block::Runtime("static contact task graph"));
    }
    let primary = entity.actor_task_state(ActorTaskSlot::Primary);
    let secondary = entity.actor_task_state(ActorTaskSlot::Secondary);
    let tertiary = entity.actor_task_state(ActorTaskSlot::Tertiary);
    // A8B0 visits current Primary/Secondary/Tertiary in order. Acquiring
    // secondaries and ADE0's Tertiary Aim retain05FF0's null static hook.
    // 02B10/03B70/03360 install02CA0 at template+20 (402B6B/403BDB/4033C3).
    match (style, primary, secondary, tertiary) {
        (0x4c7ed0, Some(Task::CommonDying(_)), None, None)
            if entity.collision.state_flags_at_0x08.masked(0x4000)
                == RetailRuntimeValue::Known(0x4000) =>
        {
            // 404120 retains05FF0's null+20;4C7ED0 also has null+1C.
            // Neither callback changes tasks, components or the shared RNG.
            Ok(Type58StaticTaskHook::Null)
        }
        (0x4c7738, Some(Task::TrashFurniture(_)), None, None) => {
            Ok(Type58StaticTaskHook::Furniture)
        }
        (
            0x4c7b28,
            Some(Task::SharedRetarget(task)),
            Some(Task::FollowBeaconAcquisition(_)),
            None,
        )
        | (0x4c7a50, Some(Task::SharedRetarget(task)), Some(Task::TargetAcquisition(_)), None) => {
            Ok(Type58StaticTaskHook::WanderPrivate(task.private_state()))
        }
        (0x4c7b70, Some(Task::FollowBeaconsFollowing(task)), None, None) => {
            Ok(Type58StaticTaskHook::WanderPrivate(task.private_state()))
        }
        (0x4c7a98, Some(Task::ChaseTarget(task)), None, None | Some(Task::AimAndFire(_))) => {
            Ok(Type58StaticTaskHook::WanderPrivate(task.private_state()))
        }
        _ => Err(Block::Runtime("static contact task graph")),
    }
}

fn apply_wander_private_hook(
    entity: &mut Entity,
    private_state: crate::wander_near_location::WanderNearPrivateState,
    world_fx: &mut WorldFx,
) -> Result<(), Intro2Type58ContactBlock> {
    let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
        return Err(Intro2Type58ContactBlock::Runtime("Sub-A runtime"));
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
        || u32::from(world_fx.next_shared_retail_random_u16()),
    )
    .expect("authenticated Type58 A/B/C/D/E/H topology");
    let primary = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .unwrap();
    match entity.actor_tasks.task_state_mut(primary).unwrap() {
        ActorTaskRuntime::SharedRetarget(task) => {
            task.apply_static_contact_private_state(plan.private_state_after)
        }
        ActorTaskRuntime::FollowBeaconsFollowing(task) => {
            let mut stage = task.stage_callback();
            *stage.private_state_mut() = plan.private_state_after;
            stage.commit(task);
        }
        ActorTaskRuntime::ChaseTarget(task) => {
            let mut stage = task.stage_callback();
            *stage.private_state_mut() = plan.private_state_after;
            stage.commit(task);
        }
        _ => unreachable!("preflight retains exact static-hook task"),
    }
    entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(plan.sub_a_runtime);
    // The no-Sub-I hook leaves heading and incoming physical basis unchanged.
    // 019C0 does not perform the pair hook's immediate Sub-D reversal write.
    Ok(())
}

fn deliver_static(
    frame: &mut Intro2ContactFrame<'_>,
    cell: [u8; 2],
    packet: DamagePacket,
) -> Result<Option<StaticDamageOutcome>, Intro2Type58ContactBlock> {
    use Intro2Type58ContactBlock as Block;
    let Some(target) =
        resolve_current_static_damage_target(frame.resources, cell).map_err(Block::StaticLookup)?
    else {
        return Ok(None);
    };
    let outcome = frame.static_damage.submit_hit(target, packet, &mut || {
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
                    crate::static_damage_live::CurrentStaticDamageLookupError::BurnCallback(error),
                )
            })?;
        }
        _ => {}
    }
    Ok(Some(outcome))
}

#[cfg(test)]
#[path = "contact_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "contact_wander_tests.rs"]
mod wander_tests;

#[cfg(test)]
#[path = "contact_class12_tests.rs"]
mod class12_tests;
