//! Late11AD0 static contact and its successor-pair phase for ordinary Type9.
//!
//! This owner retains the authored static geometry response, including the
//! Level1 spider-pen fences, and A8B0's current task +20
//! hooks before D920/11760. Type9's Sub-I branch turns heading and retargets
//! X/Z without rebuilding the physical basis. Damage runs only on a
//! nonzero impact, so gentle fence brushes stay silent as in retail.

use crate::{
    actor_task_dispatcher::{ActorTaskRuntime, ActorTaskRuntimeFamily},
    actor_task_owner::ActorTaskSlot,
    damage::{velocity_delta_impact_raw, DamageDeliveryRecord, DamagePacket},
    entity::EntityManager,
    entity_collision_state::{
        active_model_slot_from_state_flags, EntityTypeRuntimeMetadata, RetailRuntimeValue,
        ACTIVE_MODEL_SLOT_HIGH_STATE_BIT, ACTIVE_MODEL_SLOT_LOW_STATE_BIT, REMOTE_OWNED_STATE_BIT,
    },
    follow_beacons::static_contact::{
        plan_wander_private_static_contact, FollowBeaconsStaticContactTopology,
        WanderPrivateStaticContactRequest,
    },
    gameplay_notifications::GameplayNotifications,
    intro2_type9_class14::Intro2Type9Class14Owner,
    live_actor_checked_damage::{
        apply_live_actor_checked_damage, LiveActorDamageEntry, LiveActorDamageError,
        LiveActorDamageOutcome, LiveActorDamageRequest, LiveActorDeathResult,
    },
    main_base_type9_abort::{
        exact_level_one_type9_metadata, MainBaseType9ExplodingTaskLease,
        MainBaseType9ResultScreenState,
    },
    ordinary_type9_construction::ordinary_type9_native_allocation_authenticates,
    ordinary_type9_standard_death::{
        OrdinaryType9StandardDeathBlock, OrdinaryType9StandardDeathEntry,
        OrdinaryType9StandardDeathOutcome,
    },
    resource_cache::ResourceCache,
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
pub enum OrdinaryType9StaticContactBlock {
    Runtime(&'static str),
    UnsupportedStyle(u32),
    UnsupportedTask {
        slot: ActorTaskSlot,
        family: ActorTaskRuntimeFamily,
    },
    Scan(StaticContactError),
    StaticLookup(CurrentStaticDamageLookupError),
    UnsupportedStaticKind(u32),
    UnsupportedStaticSound(u16),
    Damage(LiveActorDamageError<OrdinaryType9StandardDeathBlock, MainBaseType9ExplodingTaskLease>),
}

/// The physical prefix and both ordered damage results for one contact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrdinaryType9StaticContactApplied {
    pub contact: StaticModelContact,
    pub position_before_raw: [i16; 3],
    pub position_after_raw: [i16; 3],
    pub velocity_before_raw: [i16; 3],
    pub velocity_after_raw: [i16; 3],
    pub impact_raw: i32,
    pub static_damage: Option<StaticDamageOutcome>,
    pub actor_damage: Option<LiveActorDamageOutcome<MainBaseType9ExplodingTaskLease>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrdinaryType9StaticContactOutcome {
    Ineligible,
    Miss,
    Applied(OrdinaryType9StaticContactApplied),
    Blocked {
        reason: OrdinaryType9StaticContactBlock,
        committed_prefix: bool,
    },
}

/// Runtime services at the post-integration contact boundary.
pub struct OrdinaryType9StaticContactFrame<'a> {
    pub resources: &'a mut ResourceCache,
    pub static_damage: &'a mut StaticDamageScheduler,
    pub world_fx: &'a mut WorldFx,
    pub scheduler: &'a mut SpecializedActorTaskScheduler,
    pub notifications: &'a mut GameplayNotifications,
    pub retail_tick: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrdinaryType9PairSuffix {
    StaticBlocked,
    Visited(crate::native_actor_capture::pair::NativeCaptorPairOutcome),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrdinaryType9LateContactOutcome {
    pub static_contact: OrdinaryType9StaticContactOutcome,
    pub pair_suffix: OrdinaryType9PairSuffix,
}

/// One ordinary peasant's late11AD0 prefix and successor-only pair suffix.
/// The source retains entry flags/model/+70 before12CF0, even if11760 kills
/// the actor. Current pose, behavior and A900 slots are read after static
/// response. An unowned static suffix stops this subject, not the live pass.
pub fn resolve_ordinary_type9_late_contact(
    frame: &mut crate::intro2_contacts::Intro2ContactFrame<'_>,
    id: u32,
    metadata: &EntityTypeRuntimeMetadata,
    playing_player: Option<crate::native_actor_capture::pair::PlayingPlayerContact<'_>>,
) -> OrdinaryType9LateContactOutcome {
    let subject_entry = frame
        .entities
        .iter_all()
        .find(|entity| entity.id == id)
        .map(|entity| {
            crate::player_active_contact::active_pair_body_from_entity(entity, frame.resources)
        });
    let static_contact = resolve_ordinary_type9_static_contact(
        frame.entities,
        id,
        metadata,
        OrdinaryType9StaticContactFrame {
            resources: frame.resources,
            static_damage: frame.static_damage,
            world_fx: frame.world_fx,
            scheduler: frame.actor_tasks,
            notifications: frame.notifications,
            retail_tick: frame.retail_tick,
        },
    );
    let pair_suffix = if matches!(
        static_contact,
        OrdinaryType9StaticContactOutcome::Blocked { .. }
    ) {
        OrdinaryType9PairSuffix::StaticBlocked
    } else {
        use crate::native_actor_capture::pair::{
            resolve_native_actor_active_contacts_at_entry, CaptureFeedbackPolicy,
            NativeActorPairRequest,
        };
        OrdinaryType9PairSuffix::Visited(resolve_native_actor_active_contacts_at_entry(
            frame,
            NativeActorPairRequest {
                id,
                feedback: CaptureFeedbackPolicy::Gameplay,
                playing_player,
                subject_entry: subject_entry.as_ref(),
            },
        ))
    };
    OrdinaryType9LateContactOutcome {
        static_contact,
        pair_suffix,
    }
}

pub fn resolve_ordinary_type9_static_contact(
    manager: &mut EntityManager,
    id: u32,
    metadata: &EntityTypeRuntimeMetadata,
    mut frame: OrdinaryType9StaticContactFrame<'_>,
) -> OrdinaryType9StaticContactOutcome {
    let mut committed = false;
    match resolve(manager, id, metadata, &mut frame, &mut committed) {
        Ok(outcome) => outcome,
        Err(reason) => {
            if committed {
                frame.scheduler.park_native_contact_prefix(manager, id);
            }
            OrdinaryType9StaticContactOutcome::Blocked {
                reason,
                committed_prefix: committed,
            }
        }
    }
}

fn resolve(
    manager: &mut EntityManager,
    id: u32,
    metadata: &EntityTypeRuntimeMetadata,
    frame: &mut OrdinaryType9StaticContactFrame<'_>,
    committed: &mut bool,
) -> Result<OrdinaryType9StaticContactOutcome, OrdinaryType9StaticContactBlock> {
    use OrdinaryType9StaticContactBlock as Block;
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(Block::Runtime("allocation"))?;
    if entity.entity_type != 9 || !ordinary_type9_native_allocation_authenticates(manager, id) {
        return Ok(OrdinaryType9StaticContactOutcome::Ineligible);
    }
    if !exact_level_one_type9_metadata(metadata) {
        return Err(Block::Runtime("noncanonical Type9 metadata"));
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
        RetailRuntimeValue::Known(false) => {
            return Ok(OrdinaryType9StaticContactOutcome::Ineligible)
        }
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
        return Ok(OrdinaryType9StaticContactOutcome::Miss);
    };
    // 12CF0 reads type +0x8A before27E20/A8B0/D920. An authored nonzero cue
    // has unaudited sound semantics; fail closed instead of playing nothing.
    let type_record = frame
        .resources
        .global_entity_type(9)
        .ok_or(Block::Runtime("type record"))?;
    let sound = u16::from_le_bytes(
        type_record.raw_header[0x8a..0x8c]
            .try_into()
            .map_err(|_| Block::Runtime("static sound word"))?,
    );
    if sound != 0 {
        return Err(Block::UnsupportedStaticSound(sound));
    }
    // Preserve the walking-family admission. Both zero-damage separation and
    // velocity response below are external writes after the completed12DA0
    // tail, so consume that exact observation before11760 can mutate it.
    // Task clocks, current family and next transaction identity survive.
    if !frame
        .scheduler
        .ordinary_type9_completed_walking_owner(manager, id)
    {
        return Ok(OrdinaryType9StaticContactOutcome::Ineligible);
    }
    //27E20's kind actions require capability1, which canonical Type9 lacks.
    // The actual living6/10/45/54 styles have null+1C. Wander4C79C0
    // clears21000; the other masks are zero (4C7618/7660,4C86B0/86F8,
    //4C8788). None enables400, and their C8=2F
    // therefore never enters D9B0's400 crushing branch. Audit only the
    // admitted walking graph; Carried/Class14 retain their separate owner.
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(Block::Runtime("current static style"));
    };
    if !matches!(
        context.active_style().style_address(),
        0x004c_79c0 | 0x004c_7618 | 0x004c_7660 | 0x004c_86b0 | 0x004c_86f8 | 0x004c_8788
    ) {
        return Err(Block::UnsupportedStyle(
            context.active_style().style_address(),
        ));
    }
    if frame
        .scheduler
        .begin_native_type9_external_mutation(manager, id)
        .is_none()
    {
        return Err(Block::Runtime("completed static motion custody"));
    }
    apply_contact(manager, id, frame, contact, committed)
        .map(OrdinaryType9StaticContactOutcome::Applied)
}

fn apply_contact(
    manager: &mut EntityManager,
    id: u32,
    frame: &mut OrdinaryType9StaticContactFrame<'_>,
    contact: StaticModelContact,
    committed: &mut bool,
) -> Result<OrdinaryType9StaticContactApplied, OrdinaryType9StaticContactBlock> {
    use OrdinaryType9StaticContactBlock as Block;
    apply_task_static_hooks(manager, id, frame.world_fx, committed)?;
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
    let mut applied = OrdinaryType9StaticContactApplied {
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
        // Re-read the cell at delivery: never cache a fence's model slot
        // across an earlier destruction or terrain mutation.
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
                |manager, fx, _feedback| {
                    let death = manager.publish_ordinary_type9_standard_death(
                        id,
                        OrdinaryType9StandardDeathEntry::GenericDeath,
                        MainBaseType9ResultScreenState::NotShown,
                        fx,
                        frame.retail_tick as i32,
                        Some(frame.notifications),
                    )?;
                    let publication = manager
                        .main_base_abort_actor_observation(id)
                        .and_then(|observation| {
                            crate::ordinary_type9_outer_tail::outer_tail_class14_exploding_task_lease(
                                manager,
                                observation.lease,
                            )
                        });
                    Ok(LiveActorDeathResult {
                        returned_nonzero: matches!(
                            death,
                            OrdinaryType9StandardDeathOutcome::Published { .. }
                                | OrdinaryType9StandardDeathOutcome::InitializerFallback { .. }
                        ),
                        publication,
                    })
                },
            )
            .map_err(|error| {
                *committed |= error.committed_prefix;
                if let Some(lease) = error.death_publication {
                    frame
                        .scheduler
                        .register_intro2_type9_class14(Intro2Type9Class14Owner::adopt(lease));
                }
                Block::Damage(error)
            })?,
        );
        if let Some(lease) = applied
            .actor_damage
            .as_ref()
            .and_then(|outcome| outcome.death_publication)
        {
            frame
                .scheduler
                .register_intro2_type9_class14(Intro2Type9Class14Owner::adopt(lease));
        }
    }
    Ok(applied)
}

///01270 visits the freshly read slots in0/1/2 order. The common Type9
/// constructors install02CA0 at402E7B (Wander),402B6B (retarget),4032FB
/// (Attract local wander),4036B3 (job/target route) and403E83 (fleeing).
/// Candidate01F80 and acquisition02050 use405F80; cue02A10 preserves
///405FF0's null+20 at406019. Carried and Class14 have separate admission.
fn task_has_static_retarget(task: &ActorTaskRuntime) -> Option<bool> {
    match task {
        ActorTaskRuntime::OrdinaryType9Wander(_)
        | ActorTaskRuntime::SharedRetarget(_)
        | ActorTaskRuntime::GoToJob(_)
        | ActorTaskRuntime::RunAway(_)
        | ActorTaskRuntime::AttractAttentionTargetRoute(_) => Some(true),
        ActorTaskRuntime::TargetAcquisition(_)
        | ActorTaskRuntime::AttractAttentionCandidate(_)
        | ActorTaskRuntime::AttractAttentionCue(_) => Some(false),
        _ => None,
    }
}

fn apply_task_static_hooks(
    manager: &mut EntityManager,
    id: u32,
    fx: &mut WorldFx,
    committed: &mut bool,
) -> Result<(), OrdinaryType9StaticContactBlock> {
    use OrdinaryType9StaticContactBlock as Block;
    for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
        let entity = manager.entity_mut(id).ok_or(Block::Runtime("allocation"))?;
        let Some(task_id) = entity.actor_tasks.task_in_slot(slot) else {
            continue;
        };
        let task = entity
            .actor_tasks
            .task_state(task_id)
            .ok_or(Block::Runtime("static task"))?;
        match task_has_static_retarget(task) {
            Some(false) => continue,
            None => {
                return Err(Block::UnsupportedTask {
                    slot,
                    family: task.family(),
                })
            }
            Some(true) => {}
        }
        // Type9's authenticated A/B/D/I topology always takes the Sub-I
        // branch: no A/F/G direction propagation, animation visit, Sub-D
        // reversal write or body-basis rebuild belongs to this callback.
        let private_state = crate::native_actor_descriptor_contact::private_state(task)
            .expect("audited static task private layout");
        let plan = plan_wander_private_static_contact(
            WanderPrivateStaticContactRequest {
                private_state,
                controlled_position_raw: entity.position_raw(),
                heading_raw: entity.heading_raw(),
                topology: RetailRuntimeValue::Known(FollowBeaconsStaticContactTopology {
                    sub_i: true,
                    sub_a: None,
                    sub_f: false,
                    sub_g: false,
                }),
            },
            || u32::from(fx.next_shared_retail_random_u16()),
        )
        .expect("authenticated Type9 topology");
        *committed = true;
        crate::native_actor_descriptor_contact::commit_private(
            entity.actor_tasks.task_state_mut(task_id).unwrap(),
            plan.private_state_after,
        );
        entity.set_heading_raw(plan.heading_raw);
    }
    Ok(())
}

#[cfg(test)]
#[path = "ordinary_type9_static_contact/late_tests.rs"]
mod late_tests;
