//! Complete source-owned Type42/59 static-model contact after the surface pass.
//!
//! 412CF0: type+8A cue, 427E20, A8B0, D920, style+1C40CEF0, then11760.
//! Capability40 excludes every nonzero427E20 branch. The actual published
//! task constructors retain05FF0's null+20 hook; living flags12/8 exclude
//! D9B0. The terminal callback can mutate the world synchronously, so11760
//! retains only the selected plane and reacquires current motion/static cells.

use super::reborrow_terminal_world;
use crate::{
    active_pair::{ActivePairBody, ActivePairModelState},
    actor_task_owner::ActorTaskSlot,
    class49_terminal::{run_class49_standard_death, Class49TerminalBlock, Class49TerminalFrame},
    damage::{velocity_delta_impact_raw, DamageDeliveryRecord, DamagePacket},
    entity_collision_state::{active_model_slot_from_state_flags, RetailRuntimeValue},
    live_actor_checked_damage::{
        apply_live_actor_checked_damage, LiveActorDamageEntry, LiveActorDamageError,
        LiveActorDamageOutcome, LiveActorDamageRequest, LiveActorDeathResult,
    },
    native_entity_weapons::{
        authenticate_weapon_metadata, EntityWeaponBlock, NativeEntityWeaponOwner,
    },
    static_contact::{
        apply_contact_response_raw, scan_deepest_static_contact, static_contact_subject_eligible,
        StaticContactError, StaticContactQuery, StaticModelContact,
    },
    static_damage::StaticDamageOutcome,
    static_damage_live::{resolve_current_static_damage_target, CurrentStaticDamageLookupError},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntityWeaponStaticBlock {
    Owner(EntityWeaponBlock),
    Scan(StaticContactError),
    Terminal(Class49TerminalBlock),
    Lookup(CurrentStaticDamageLookupError),
    UnsupportedStaticKind(u32),
    Burn(crate::static_terrain_burn::StaticTerrainBurnFailure),
    Damage(LiveActorDamageError<Class49TerminalBlock, ()>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityWeaponStaticError {
    pub reason: EntityWeaponStaticBlock,
    /// Any terminal, sound, response or hit prefix must be retained and parked.
    pub committed_prefix: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntityWeaponStaticOutcome {
    Ineligible,
    Miss,
    Applied {
        contact: StaticModelContact,
        position_before_raw: [i16; 3],
        position_after_raw: [i16; 3],
        velocity_before_raw: [i16; 3],
        velocity_after_raw: [i16; 3],
        impact_raw: i32,
        static_damage: Option<StaticDamageOutcome>,
        actor_damage: Option<LiveActorDamageOutcome<()>>,
    },
}

/// Caller supplies the current complete explosion/controller custody.
pub fn resolve_entity_weapon_static_contact(
    mut frame: Class49TerminalFrame<'_>,
    owner: NativeEntityWeaponOwner,
) -> Result<EntityWeaponStaticOutcome, EntityWeaponStaticError> {
    let mut committed = false;
    run(&mut frame, owner, None, &mut committed).map_err(|reason| EntityWeaponStaticError {
        reason,
        committed_prefix: committed,
    })
}

pub(crate) fn resolve_entity_weapon_static_contact_at_entry(
    mut frame: Class49TerminalFrame<'_>,
    owner: NativeEntityWeaponOwner,
    entry: &ActivePairBody,
) -> Result<EntityWeaponStaticOutcome, EntityWeaponStaticError> {
    let mut committed = false;
    run(&mut frame, owner, Some(entry), &mut committed).map_err(|reason| EntityWeaponStaticError {
        reason,
        committed_prefix: committed,
    })
}

fn run(
    frame: &mut Class49TerminalFrame<'_>,
    owner: NativeEntityWeaponOwner,
    retained_entry: Option<&ActivePairBody>,
    committed: &mut bool,
) -> Result<EntityWeaponStaticOutcome, EntityWeaponStaticBlock> {
    use EntityWeaponStaticBlock as Block;
    let fail = |field| Block::Owner(EntityWeaponBlock::Runtime(field));
    let id = owner.entity_id();
    let entity = frame
        .entities
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(Block::Owner(EntityWeaponBlock::Allocation))?;
    if !entity.active {
        return Ok(EntityWeaponStaticOutcome::Ineligible);
    }
    let entry_collision = retained_entry.map_or(&entity.collision, |entry| &entry.collision);
    let RetailRuntimeValue::Known(selector) = entry_collision.state_flags_at_0x08.masked(0x6000)
    else {
        return Err(fail("static model selector"));
    };
    let model_id = match retained_entry {
        Some(entry) if entry.id == id => match entry.active_model {
            ActivePairModelState::Resolved(model) => model.global_id,
            _ => return Err(fail("retained static active model")),
        },
        Some(_) => return Err(fail("retained static allocation")),
        None => entity
            .model_in_slot(active_model_slot_from_state_flags(selector))
            .ok_or(fail("static active model"))?,
    };
    let model = frame
        .resources
        .global_model(model_id)
        .ok_or(fail("static active model resource"))?;
    match static_contact_subject_eligible(entry_collision, model.collision_radius_raw) {
        RetailRuntimeValue::Known(false) => return Ok(EntityWeaponStaticOutcome::Ineligible),
        RetailRuntimeValue::Unresolved => return Err(fail("static eligibility")),
        RetailRuntimeValue::Known(true) => {}
    }
    let finished = retained_entry.is_some()
        && crate::class49_death::finished_terminal_hit_authenticates(frame.entities, id)
        && crate::class49_death::allocation_authenticates(frame.entities, id);
    if !owner.authenticates(frame.entities) && !finished {
        return Err(Block::Owner(EntityWeaponBlock::Allocation));
    }
    authenticate_weapon_metadata(
        owner.kind(),
        frame
            .entities
            .type_runtime_metadata(entity.entity_type)
            .ok_or(Block::Owner(EntityWeaponBlock::Metadata))?,
    )
    .map_err(Block::Owner)?;
    let living_style = if owner.kind().entity_type() == 42 {
        0x4c81a0
    } else {
        0x4c8350
    };
    let expected_style = if finished {
        if owner.kind().entity_type() == 42 {
            0x4c7150
        } else {
            0x4c71e0
        }
    } else {
        living_style
    };
    if !matches!(entity.current_behavior_context, RetailRuntimeValue::Known(Some(style))
        if style.active_style().style_address() == expected_style)
        || ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .filter_map(|slot| entity.actor_tasks.task_in_slot(slot))
            .any(|task| {
                entity
                    .actor_tasks
                    .wrapper_flags(task)
                    .is_none_or(|flags| !flags.alive || flags.in_callback)
            })
    {
        return Err(Block::Owner(EntityWeaponBlock::TaskChanged));
    }
    if entity.capability_flags != 0x40 {
        return Err(fail("427E20 capability exclusion"));
    }
    let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
        return Err(fail("static physical basis"));
    };
    let contact = scan_deepest_static_contact(StaticContactQuery {
        terrain: frame
            .resources
            .level_terrain()
            .ok_or(fail("static terrain"))?,
        terrain_objects: frame
            .resources
            .terrain_objects()
            .ok_or(fail("static terrain objects"))?,
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
        return Ok(EntityWeaponStaticOutcome::Miss);
    };
    let record = frame
        .resources
        .global_entity_type(owner.kind().entity_type())
        .ok_or(Block::Owner(EntityWeaponBlock::Metadata))?;
    let cue = u16::from_le_bytes(record.raw_header[0x8a..0x8c].try_into().unwrap());
    if cue != 0 {
        *committed = true;
        frame
            .world_fx
            .emit_fixed_positional_sound_raw(cue, entity.position_raw());
    }
    // Source-proven no-op427E20 and all null A8B0 hooks precede D920. Both
    // living styles preserve default flags12/8 and have terminal+1C40CEF0.
    *committed = true;
    if !finished {
        run_class49_standard_death(
            Class49TerminalFrame {
                entities: frame.entities,
                resources: frame.resources,
                world_fx: frame.world_fx,
                static_damage: frame.static_damage,
                notifications: frame.notifications,
                retail_tick: frame.retail_tick,
                world: reborrow_terminal_world(&mut frame.world),
            },
            id,
        )
        .map_err(Block::Terminal)?;
    }
    let entity = frame
        .entities
        .entity_mut(id)
        .ok_or(fail("post-static-callback allocation"))?;
    let position_before_raw = entity.position_raw();
    let velocity_before_raw = entity.velocity_raw();
    let mut position_after_raw = position_before_raw;
    let mut velocity_after_raw = velocity_before_raw;
    apply_contact_response_raw(&mut position_after_raw, &mut velocity_after_raw, contact);
    entity.set_motion_raw(position_after_raw, velocity_after_raw);
    let impact_raw = match entity.collision.state_flags_at_0x08.masked(0x8000_0000) {
        RetailRuntimeValue::Known(0) => {
            velocity_delta_impact_raw(velocity_before_raw, velocity_after_raw, entity.mass_raw)
        }
        RetailRuntimeValue::Known(_) => 0,
        RetailRuntimeValue::Unresolved => return Err(fail("11760 remote owner")),
    };
    let mut static_damage = None;
    let mut actor_damage = None;
    if impact_raw != 0 {
        // Re-resolve the cell after the terminal's radial traversal and static
        // writes; never send to the stale descriptor selected by the scan.
        if let Some(target) = resolve_current_static_damage_target(frame.resources, contact.cell)
            .map_err(Block::Lookup)?
        {
            let outcome = frame.static_damage.submit_hit(
                target,
                DamagePacket::collision(impact_raw),
                &mut || frame.world_fx.next_shared_retail_random_u16(),
            );
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
                    .map_err(Block::Burn)?;
                }
                _ => {}
            }
            static_damage = Some(outcome);
        }
        actor_damage = Some(
            apply_live_actor_checked_damage(
                frame.entities,
                frame.world_fx,
                LiveActorDamageRequest {
                    entity_id: id,
                    delivery: DamageDeliveryRecord {
                        packet: DamagePacket::collision(impact_raw),
                        source_entity_type_raw: owner.kind().entity_type() as u32,
                        owner_handle: id,
                    },
                    entry: LiveActorDamageEntry::Checked,
                    ratio_numerator: 0,
                    ratio_denominator: 0,
                    feedback: None,
                },
                |entities, world_fx, _| {
                    let result = run_class49_standard_death(
                        Class49TerminalFrame {
                            entities,
                            resources: frame.resources,
                            world_fx,
                            static_damage: frame.static_damage,
                            notifications: frame.notifications,
                            retail_tick: frame.retail_tick,
                            world: reborrow_terminal_world(&mut frame.world),
                        },
                        id,
                    )?;
                    Ok(LiveActorDeathResult {
                        returned_nonzero: result.returned_nonzero,
                        publication: None::<()>,
                    })
                },
            )
            .map_err(Block::Damage)?,
        );
    }
    Ok(EntityWeaponStaticOutcome::Applied {
        contact,
        position_before_raw,
        position_after_raw,
        velocity_before_raw,
        velocity_after_raw,
        impact_raw,
        static_damage,
        actor_damage,
    })
}
