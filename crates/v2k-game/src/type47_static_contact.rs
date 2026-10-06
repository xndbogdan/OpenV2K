//! Late 11AD0 static contact for Type47 newants, ordinary and Intro2.
//!
//! Without this pass newants walk through static geometry (notably the
//! level-1 fences) and, in Intro2, through live structures like the hive
//! approach: no Type47 static-contact owner exists and the pair lanes only
//! cover Type17-involved pairs. This owner mirrors
//! [`crate::ordinary_type9_static_contact`] without the task-specific 02CA0
//! retarget, which stays open pending the Type47 style-hook matrix: the
//! physical response alone restores containment. Damage runs only on a
//! nonzero impact, so gentle brushes stay silent as in retail.

use crate::{
    damage::{velocity_delta_impact_raw, DamageDeliveryRecord, DamagePacket},
    entity::EntityManager,
    entity_collision_state::{
        active_model_slot_from_state_flags, EntityTypeRuntimeMetadata, RetailRuntimeValue,
        ACTIVE_MODEL_SLOT_HIGH_STATE_BIT, ACTIVE_MODEL_SLOT_LOW_STATE_BIT, REMOTE_OWNED_STATE_BIT,
    },
    intro2_common_dying::{
        publish_intro2_common_standard_death, Intro2CommonDyingBlock, Intro2CommonDyingOwner,
    },
    intro2_type47_live::authenticate_intro2_type47,
    live_actor_checked_damage::{
        apply_live_actor_checked_damage, LiveActorDamageEntry, LiveActorDamageError,
        LiveActorDamageOutcome, LiveActorDamageRequest, LiveActorDeathResult,
    },
    ordinary_type47_death_live::{
        publish_fresh_level_one_type47_standard_death, FreshLevelOneType47CommonDyingOwner,
        Type47CommonDyingPublicationError, Type47StandardDeathOutcome,
    },
    resource_cache::ResourceCache,
    shared_type47::type47_manager_allocation_authenticates,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    static_contact::{
        apply_contact_response_raw, scan_deepest_static_contact, static_contact_subject_eligible,
        StaticContactError, StaticContactQuery, StaticModelContact,
    },
    static_damage::{StaticDamageOutcome, StaticDamageScheduler},
    static_damage_live::{resolve_current_static_damage_target, CurrentStaticDamageLookupError},
    world_fx::WorldFx,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type47StaticDeathBlock {
    Ordinary(Type47CommonDyingPublicationError),
    Intro2(Intro2CommonDyingBlock),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type47StaticDeathOwner {
    Ordinary(FreshLevelOneType47CommonDyingOwner),
    Intro2(Intro2CommonDyingOwner),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type47StaticContactBlock {
    Runtime(&'static str),
    Scan(StaticContactError),
    StaticLookup(CurrentStaticDamageLookupError),
    UnsupportedStaticKind(u32),
    UnsupportedStaticSound(u16),
    Damage(LiveActorDamageError<Type47StaticDeathBlock, Type47StaticDeathOwner>),
}

/// The physical prefix and both ordered damage results for one contact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Type47StaticContactApplied {
    pub contact: StaticModelContact,
    pub position_before_raw: [i16; 3],
    pub position_after_raw: [i16; 3],
    pub velocity_before_raw: [i16; 3],
    pub velocity_after_raw: [i16; 3],
    pub impact_raw: i32,
    pub static_damage: Option<StaticDamageOutcome>,
    pub actor_damage: Option<LiveActorDamageOutcome<Type47StaticDeathOwner>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type47StaticContactOutcome {
    Ineligible,
    Miss,
    Applied(Type47StaticContactApplied),
    Blocked {
        reason: Type47StaticContactBlock,
        committed_prefix: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Type47StaticDeathKind {
    Ordinary,
    Intro2,
}

/// Runtime services at the post-integration contact boundary.
pub struct Type47StaticContactFrame<'a> {
    pub resources: &'a mut ResourceCache,
    pub static_damage: &'a mut StaticDamageScheduler,
    pub world_fx: &'a mut WorldFx,
    pub scheduler: &'a mut SpecializedActorTaskScheduler,
    pub retail_tick: u32,
}

pub fn resolve_ordinary_type47_static_contact(
    manager: &mut EntityManager,
    id: u32,
    metadata: &EntityTypeRuntimeMetadata,
    frame: Type47StaticContactFrame<'_>,
) -> Type47StaticContactOutcome {
    resolve(
        manager,
        id,
        metadata,
        frame,
        Type47StaticDeathKind::Ordinary,
    )
}

pub fn resolve_intro2_type47_static_contact(
    manager: &mut EntityManager,
    id: u32,
    metadata: &EntityTypeRuntimeMetadata,
    frame: Type47StaticContactFrame<'_>,
) -> Type47StaticContactOutcome {
    resolve(manager, id, metadata, frame, Type47StaticDeathKind::Intro2)
}

fn resolve(
    manager: &mut EntityManager,
    id: u32,
    metadata: &EntityTypeRuntimeMetadata,
    mut frame: Type47StaticContactFrame<'_>,
    death: Type47StaticDeathKind,
) -> Type47StaticContactOutcome {
    let mut committed = false;
    match run(manager, id, metadata, &mut frame, death, &mut committed) {
        Ok(outcome) => outcome,
        Err(reason) => Type47StaticContactOutcome::Blocked {
            reason,
            committed_prefix: committed,
        },
    }
}

fn run(
    manager: &mut EntityManager,
    id: u32,
    metadata: &EntityTypeRuntimeMetadata,
    frame: &mut Type47StaticContactFrame<'_>,
    death: Type47StaticDeathKind,
    committed: &mut bool,
) -> Result<Type47StaticContactOutcome, Type47StaticContactBlock> {
    use Type47StaticContactBlock as Block;
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(Block::Runtime("allocation"))?;
    if entity.entity_type != 47 {
        return Ok(Type47StaticContactOutcome::Ineligible);
    }
    let authenticated = match death {
        Type47StaticDeathKind::Ordinary => type47_manager_allocation_authenticates(manager, id),
        Type47StaticDeathKind::Intro2 => authenticate_intro2_type47(entity, metadata).is_ok(),
    };
    if !authenticated {
        return Ok(Type47StaticContactOutcome::Ineligible);
    }
    let selector = match entity
        .collision
        .state_flags_at_0x08
        .masked(ACTIVE_MODEL_SLOT_LOW_STATE_BIT | ACTIVE_MODEL_SLOT_HIGH_STATE_BIT)
    {
        RetailRuntimeValue::Known(selector) => selector,
        _ => return Err(Block::Runtime("active model selector")),
    };
    let model_id = entity
        .model_in_slot(active_model_slot_from_state_flags(selector))
        .ok_or(Block::Runtime("active model"))?;
    let model = frame
        .resources
        .global_model(model_id)
        .ok_or(Block::Runtime("active model resource"))?;
    match static_contact_subject_eligible(&entity.collision, model.collision_radius_raw) {
        RetailRuntimeValue::Known(false) => return Ok(Type47StaticContactOutcome::Ineligible),
        RetailRuntimeValue::Unresolved => {
            return Err(Block::Runtime("static scan eligibility"));
        }
        RetailRuntimeValue::Known(true) => {}
    }
    let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
        return Err(Block::Runtime("physical body basis"));
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
        return Ok(Type47StaticContactOutcome::Miss);
    };
    // D920 reads type +0x8A for a static-contact cue. An authored nonzero cue
    // has unaudited sound semantics; fail closed instead of playing nothing.
    let type_record = frame
        .resources
        .global_entity_type(47)
        .ok_or(Block::Runtime("type record"))?;
    let sound = u16::from_le_bytes(
        type_record.raw_header[0x8a..0x8c]
            .try_into()
            .map_err(|_| Block::Runtime("static sound word"))?,
    );
    if sound != 0 {
        return Err(Block::UnsupportedStaticSound(sound));
    }
    // Read-only scheduler custody: static contact replaces no hit graph.
    // CommonDying owners stay with the death path.
    if !frame
        .scheduler
        .type47_completed_scheduler_owner(manager, id)
    {
        return Ok(Type47StaticContactOutcome::Ineligible);
    }
    apply_contact(manager, id, frame, death, contact, committed)
        .map(Type47StaticContactOutcome::Applied)
}

fn apply_contact(
    manager: &mut EntityManager,
    id: u32,
    frame: &mut Type47StaticContactFrame<'_>,
    death: Type47StaticDeathKind,
    contact: StaticModelContact,
    committed: &mut bool,
) -> Result<Type47StaticContactApplied, Type47StaticContactBlock> {
    use Type47StaticContactBlock as Block;
    let entity = manager.entity_mut(id).ok_or(Block::Runtime("allocation"))?;
    let position_before_raw = entity.position_raw();
    let velocity_before_raw = entity.velocity_raw();
    let mut position_after_raw = position_before_raw;
    let mut velocity_after_raw = velocity_before_raw;
    apply_contact_response_raw(&mut position_after_raw, &mut velocity_after_raw, contact);
    *committed = true;
    entity.set_motion_raw(position_after_raw, velocity_after_raw);
    // 00411760 checks remote ownership again after response and before either
    // delivery. A zero impact skips both, so gentle brushes stay silent.
    let impact_raw = match entity
        .collision
        .state_flags_at_0x08
        .masked(REMOTE_OWNED_STATE_BIT)
    {
        RetailRuntimeValue::Known(0) => {
            velocity_delta_impact_raw(velocity_before_raw, velocity_after_raw, entity.mass_raw)
        }
        RetailRuntimeValue::Known(_) => 0,
        RetailRuntimeValue::Unresolved => {
            return Err(Block::Runtime("post-response remote state"));
        }
    };
    let delivery = DamageDeliveryRecord {
        packet: DamagePacket::collision(impact_raw),
        source_entity_type_raw: entity.entity_type,
        owner_handle: entity.id,
    };
    let mut applied = Type47StaticContactApplied {
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
        // Re-read the cell at delivery: never cache a model's slot across an
        // earlier destruction or terrain mutation.
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
                    return Err(Block::UnsupportedStaticKind(kind_index));
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
        applied.actor_damage = Some(
            apply_live_actor_checked_damage(
                manager,
                frame.world_fx,
                LiveActorDamageRequest {
                    ratio_numerator: 0,
                    ratio_denominator: 0,
                    feedback: None,
                    entity_id: id,
                    delivery,
                    entry: LiveActorDamageEntry::Checked,
                },
                |manager, world_fx, _feedback| match death {
                    Type47StaticDeathKind::Ordinary => {
                        match publish_fresh_level_one_type47_standard_death(
                            manager,
                            id,
                            &metadata_cloned(manager),
                            world_fx,
                        ) {
                            Err(reason) => Err(Type47StaticDeathBlock::Ordinary(reason)),
                            Ok(outcome) => match outcome {
                                Type47StandardDeathOutcome::Published(publication) => {
                                    Ok(LiveActorDeathResult {
                                        returned_nonzero: true,
                                        publication: Some(Type47StaticDeathOwner::Ordinary(
                                            publication.owner,
                                        )),
                                    })
                                }
                                Type47StandardDeathOutcome::RemoteOwnedNoOp
                                | Type47StandardDeathOutcome::AlreadyDyingNoOp => {
                                    Ok(LiveActorDeathResult {
                                        returned_nonzero: false,
                                        publication: None,
                                    })
                                }
                            },
                        }
                    }
                    Type47StaticDeathKind::Intro2 => {
                        match publish_intro2_common_standard_death(manager, id, world_fx) {
                            Err(reason) => Err(Type47StaticDeathBlock::Intro2(reason)),
                            Ok(owner) => Ok(LiveActorDeathResult {
                                returned_nonzero: owner.is_some(),
                                publication: owner.map(Type47StaticDeathOwner::Intro2),
                            }),
                        }
                    }
                },
            )
            .map_err(|error| {
                *committed |= error.committed_prefix;
                adopt_death_lease(frame.scheduler, error.death_publication.clone());
                Block::Damage(error)
            })?,
        );
        if let Some(owner) = applied
            .actor_damage
            .as_ref()
            .and_then(|outcome| outcome.death_publication)
        {
            adopt_death_lease(frame.scheduler, Some(owner));
        }
    }
    Ok(applied)
}

fn metadata_cloned(manager: &EntityManager) -> EntityTypeRuntimeMetadata {
    manager
        .type_runtime_metadata(47)
        .cloned()
        .expect("Type47 metadata accompanies a natively constructed Type47")
}

fn adopt_death_lease(
    scheduler: &mut SpecializedActorTaskScheduler,
    owner: Option<Type47StaticDeathOwner>,
) {
    match owner {
        Some(Type47StaticDeathOwner::Ordinary(owner)) => {
            let _ = scheduler.register_type47_common_dying(owner);
        }
        Some(Type47StaticDeathOwner::Intro2(owner)) => {
            scheduler.register_intro2_common_dying(owner);
        }
        None => {}
    }
}
