//! Static-object suffix of the authenticated fresh Type-17 Following mover.
//!
//! `00411AD0 -> 00412CF0` scans the current authored models, invokes the
//! surviving task's `+0x20` hook, then enters common type callback `0040D920`.
//! Follow's style has a null `+0x1C` callback and no `0x400` crush policy.
//! `00411760` therefore responds physically and delivers the same collision
//! packet to the static cell before the actor. This owner keeps that order,
//! including shared RNG, and never retries a partially committed contact.

use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    damage::{velocity_delta_impact_raw, DamageDeliveryRecord, DamagePacket},
    entity::EntityManager,
    entity_collision_state::{
        active_model_slot_from_state_flags, EntityTypeRuntimeMetadata, RetailRuntimeValue,
        ACTIVE_MODEL_SLOT_HIGH_STATE_BIT, ACTIVE_MODEL_SLOT_LOW_STATE_BIT, REMOTE_OWNED_STATE_BIT,
    },
    follow_beacons::static_contact::{
        plan_follow_beacons_static_contact, FollowBeaconsStaticContactRequest,
        FollowBeaconsStaticContactTopology,
    },
    resource_cache::ResourceCache,
    static_contact::{
        apply_contact_response_raw, scan_deepest_static_contact, static_contact_subject_eligible,
        StaticContactError, StaticContactQuery, StaticModelContact,
    },
    static_damage::{StaticDamageOutcome, StaticDamageScheduler},
    static_damage_live::{resolve_current_static_damage_target, CurrentStaticDamageLookupError},
    type17_collision_damage::{
        apply_type17_collision_damage, Type17CollisionDamageError, Type17CollisionDamageOutcome,
    },
    type17_follow_beacons_live::{Type17FollowBeaconsFollowingOwner, Type17FollowBeaconsLiveError},
    world_fx::WorldFx,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type17StaticContactError {
    EntityUnavailable,
    UnauthenticatedBirth,
    Owner(Type17FollowBeaconsLiveError),
    RuntimeUnavailable(&'static str),
    MissingModel(usize),
    Scan(StaticContactError),
    StaticDamageLookup(CurrentStaticDamageLookupError),
    UnsupportedStaticDamage(u32),
    ActorDamage(Type17CollisionDamageError),
}

/// The physical prefix and both ordered damage results for one contact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Type17StaticContactApplied {
    pub contact: StaticModelContact,
    pub position_before_raw: [i16; 3],
    pub position_after_raw: [i16; 3],
    pub velocity_before_raw: [i16; 3],
    pub velocity_after_raw: [i16; 3],
    pub impact_raw: i32,
    pub static_damage: Option<StaticDamageOutcome>,
    pub actor_damage: Option<Type17CollisionDamageOutcome>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type17StaticContactOutcome {
    Ineligible,
    Miss,
    Applied(Type17StaticContactApplied),
}

/// Runtime services at the post-integration, pre-Sub-H-geometry boundary.
pub struct Type17StaticContactFrame<'a> {
    pub resources: &'a mut ResourceCache,
    pub static_damage: &'a mut StaticDamageScheduler,
    pub world_fx: &'a mut WorldFx,
    pub retail_tick: u32,
}

pub fn resolve_type17_follow_static_contact(
    manager: &mut EntityManager,
    owner: Type17FollowBeaconsFollowingOwner,
    metadata: &EntityTypeRuntimeMetadata,
    frame: Type17StaticContactFrame<'_>,
) -> Result<Type17StaticContactOutcome, Type17StaticContactError> {
    let Type17StaticContactFrame {
        resources,
        static_damage,
        world_fx,
        retail_tick,
    } = frame;
    if !manager.is_fresh_new_game_first_world() {
        return Err(Type17StaticContactError::UnauthenticatedBirth);
    }
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == owner.entity_id())
        .ok_or(Type17StaticContactError::EntityUnavailable)?;
    owner
        .validate(entity, metadata)
        .map_err(Type17StaticContactError::Owner)?;
    if !matches!(entity.authored_spawn_index, Some(17..=20)) || entity.model_slots != [Some(256); 4]
    {
        return Err(Type17StaticContactError::UnauthenticatedBirth);
    }
    let RetailRuntimeValue::Known(selector) = entity
        .collision
        .state_flags_at_0x08
        .masked(ACTIVE_MODEL_SLOT_LOW_STATE_BIT | ACTIVE_MODEL_SLOT_HIGH_STATE_BIT)
    else {
        return Err(Type17StaticContactError::RuntimeUnavailable(
            "active model selector",
        ));
    };
    let model_id = entity
        .model_in_slot(active_model_slot_from_state_flags(selector))
        .ok_or(Type17StaticContactError::RuntimeUnavailable("active model"))?;
    let model = resources
        .global_model(model_id)
        .ok_or(Type17StaticContactError::MissingModel(model_id))?;
    match static_contact_subject_eligible(&entity.collision, model.collision_radius_raw) {
        RetailRuntimeValue::Known(false) => return Ok(Type17StaticContactOutcome::Ineligible),
        RetailRuntimeValue::Unresolved => {
            return Err(Type17StaticContactError::RuntimeUnavailable(
                "static scan eligibility",
            ))
        }
        RetailRuntimeValue::Known(true) => {}
    }
    let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
        return Err(Type17StaticContactError::RuntimeUnavailable(
            "physical body basis",
        ));
    };
    let terrain = resources
        .level_terrain()
        .ok_or(Type17StaticContactError::RuntimeUnavailable(
            "current terrain",
        ))?;
    let terrain_objects =
        resources
            .terrain_objects()
            .ok_or(Type17StaticContactError::RuntimeUnavailable(
                "terrain object table",
            ))?;
    let anim_vars = entity.presentation_anim_vars(retail_tick);
    let contact = scan_deepest_static_contact(StaticContactQuery {
        terrain,
        terrain_objects,
        model_pool: &*resources,
        tick: retail_tick,
        active_model: model,
        active_model_to_world_basis: basis
            .orientation_world_from_model()
            .map(|row| row.map(f64::from)),
        active_anim_vars: &anim_vars,
        position_raw: entity.position_raw(),
    })
    .map_err(Type17StaticContactError::Scan)?;
    let Some(contact) = contact else {
        return Ok(Type17StaticContactOutcome::Miss);
    };

    // The common initializer installs vtable 004C8A30 (+34 = 0040D920).
    // Both 00412CF0 and 00411760 read type +8A; the fresh Type-17 cue is
    // authored zero. Capabilities 8 excludes every player-only 00427E20
    // mode-1/mode-2 action. Follow validation proves its exact style/table.
    let type_record = resources
        .global_entity_type(17)
        .ok_or(Type17StaticContactError::RuntimeUnavailable("type record"))?;
    if type_record.raw_header[0x8a..0x8c] != [0, 0]
        || entity.capability_flags != 8
        || entity.collision.default_state_flags_at_0xc8 != RetailRuntimeValue::Known(0x39)
    {
        return Err(Type17StaticContactError::RuntimeUnavailable(
            "static callback policy",
        ));
    }
    // Follow owns only Primary; validate() proves that Secondary/Tertiary
    // are empty. Its exact visit must already have unwound before 0040A8B0.
    if entity
        .actor_tasks
        .wrapper_flags(owner.primary_task_id())
        .is_none_or(|flags| flags.in_callback)
    {
        return Err(Type17StaticContactError::RuntimeUnavailable(
            "task callback still active",
        ));
    }
    let Some(ActorTaskRuntime::FollowBeaconsFollowing(task)) =
        entity.actor_tasks.task_state(owner.primary_task_id())
    else {
        return Err(Type17StaticContactError::RuntimeUnavailable(
            "Following task",
        ));
    };
    let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
        return Err(Type17StaticContactError::RuntimeUnavailable(
            "Sub-A runtime",
        ));
    };
    let plan = plan_follow_beacons_static_contact(
        FollowBeaconsStaticContactRequest {
            task: *task,
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
    .expect("validated Type-17 A/B/C/D/H/J topology");
    let entity = manager
        .type17_follow_beacons_entity_mut(owner.entity_id())
        .expect("the task hook performs no entity removal");
    *entity
        .actor_tasks
        .task_state_mut(owner.primary_task_id())
        .expect("validated Following task") =
        ActorTaskRuntime::FollowBeaconsFollowing(plan.task_after);
    entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(plan.sub_a_runtime);
    entity.set_heading_raw(plan.heading_raw);

    // Re-resolve after task hooks, as 00412CF0/0040D920 do. This Follow
    // style has no static callback or crush policy; response is 00411760.
    let entity = manager
        .type17_follow_beacons_entity_mut(owner.entity_id())
        .ok_or(Type17StaticContactError::EntityUnavailable)?;
    let position_before_raw = entity.position_raw();
    let velocity_before_raw = entity.velocity_raw();
    let mut position_after_raw = position_before_raw;
    let mut velocity_after_raw = velocity_before_raw;
    apply_contact_response_raw(&mut position_after_raw, &mut velocity_after_raw, contact);
    entity.set_motion_raw(position_after_raw, velocity_after_raw);
    // 00411760 checks this again after response and before either delivery.
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
            return Err(Type17StaticContactError::RuntimeUnavailable(
                "post-response remote state",
            ))
        }
    };
    let delivery = DamageDeliveryRecord {
        packet: DamagePacket::collision(impact_raw),
        source_entity_type_raw: entity.entity_type,
        owner_handle: entity.id,
    };
    let mut applied = Type17StaticContactApplied {
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
        // Re-read the cell at delivery. In particular, never cache a fence's
        // model slot across an earlier destruction or terrain mutation.
        if let Some(target) = resolve_current_static_damage_target(resources, contact.cell)
            .map_err(Type17StaticContactError::StaticDamageLookup)?
        {
            let outcome = static_damage.submit_hit(target, delivery.packet, &mut || {
                world_fx.next_shared_retail_random_u16()
            });
            match outcome {
                StaticDamageOutcome::UnsupportedKind { kind_index } => {
                    return Err(Type17StaticContactError::UnsupportedStaticDamage(
                        kind_index,
                    ))
                }
                StaticDamageOutcome::BurnedKind10Transition { cell, .. } => {
                    resources.apply_burned_kind_10_transition(cell);
                }
                StaticDamageOutcome::ImmediateBurn { cell, .. } => {
                    crate::static_terrain_burn::apply_immediate_static_burn(
                        cell, resources, world_fx,
                    )
                    .map_err(|error| {
                        Type17StaticContactError::StaticDamageLookup(
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
            apply_type17_collision_damage(manager, metadata, world_fx, delivery)
                .map_err(Type17StaticContactError::ActorDamage)?,
        );
    }
    Ok(Type17StaticContactOutcome::Applied(applied))
}
