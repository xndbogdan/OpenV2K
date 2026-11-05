//! The possessed body's live hooks: the release (`FUN_00446D70`), its static
//! contact's style hook (`FUN_00447C90`) and `FUN_00411760` deliveries, and
//! its death, whose `FUN_00447280` initializer releases it.

use super::{Possession, PossessionCustody};
use crate::damage::{DamageDeliveryRecord, DamagePacket};
use crate::entity::{EntityManager, PossessedStaticContact};
use crate::entity_collision_state::{RetailRuntimeValue, DYING_STATE_BIT};
use crate::gameplay_notifications::GameplayNotifications;
use crate::live_actor_checked_damage::{
    apply_live_actor_checked_damage, LiveActorDamageEntry, LiveActorDamageFeedback,
    LiveActorDamageRequest, LiveActorDeathResult,
};
use crate::resource_cache::ResourceCache;
use crate::specialized_actor_task_production::SpecializedActorTaskScheduler;
use crate::static_damage::{StaticCraterOutcome, StaticDamageOutcome, StaticDamageScheduler};
use crate::world_fx::WorldFx;

/// `FUN_00447C90`'s strength scale: the settings word `+40` it reads, as the
/// craft's own static hook.
const HAPTIC_SCALE_RAW: i32 = crate::terrain_contact::DEFAULT_PLAYER_TERRAIN_HAPTIC_SCALE_RAW;
/// `FUN_00447C90` destroys a touched object of this kind (`FUN_00427B20`).
const CRATER_KIND: u32 = 9;

/// The world a possessed body's hooks write.
pub struct PossessionWorld<'a> {
    pub entities: &'a mut EntityManager,
    pub tasks: &'a mut SpecializedActorTaskScheduler,
    pub world_fx: &'a mut WorldFx,
    pub retail_tick: u32,
}

/// What the release left.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Released {
    pub body: u32,
    pub craft: u32,
    /// The MindBall `FUN_00409950` left at the body, which the session
    /// follows; on its failure the session follows the craft.
    pub mind_ball: Result<u32, String>,
    /// The body's own behavior could not be reinstalled.
    pub behavior_stop: Option<String>,
}

/// `FUN_00446D70`: the craft takes Type46's behavior source and the
/// controller back (its install draws one selector word), the body its own
/// (`FUN_0040AC60` with no source: a living body's weighted choice, back in
/// its owner's custody; a dying one's alternate, `FUN_00425660`, without a
/// word), then the MindBall is born at the body.
pub fn release(
    world: &mut PossessionWorld<'_>,
    resources: &ResourceCache,
    custody: PossessionCustody,
) -> Released {
    let PossessionCustody {
        state: Possession { body, craft, .. },
        native_owner,
        previous_context,
    } = custody;
    // Player Control's context leaves before the reinstall reads it (retail
    // clears the body's `+D0` to its type's list): the body's own context
    // from before the takeover returns, whose target and auxiliary words the
    // port's Player Control never writes.
    if let Some(entity) = world.entities.entity_mut(body) {
        entity.current_behavior_context = RetailRuntimeValue::Known(Some(previous_context));
    }
    world.world_fx.next_shared_retail_random_u16();
    let dying = world
        .entities
        .iter_all()
        .find(|entity| entity.id == body)
        .map(|entity| entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT))
        == Some(RetailRuntimeValue::Known(DYING_STATE_BIT));
    let behavior_stop = if dying && native_owner {
        match crate::intro2_common_dying::publish_intro2_common_dying_alternate(
            world.entities,
            body,
            world.world_fx,
        ) {
            Ok(Some(owner)) => {
                world.tasks.register_intro2_common_dying(owner);
                None
            }
            Ok(None) => None,
            Err(block) => Some(format!("dying alternate: {block:?}")),
        }
    } else if dying {
        // The fresh Level-1 Type17's class-12 owner.
        match world
            .entities
            .type_runtime_metadata(17)
            .cloned()
            .map(|metadata| {
                crate::actor_standard_death_live::publish_fresh_level_one_type17_dying_alternate(
                    world.entities,
                    body,
                    &metadata,
                    world.world_fx,
                )
            }) {
            Some(Ok(owner)) => world
                .tasks
                .register_type17_common_dying(owner)
                .err()
                .map(|failure| format!("dying owner: {:?}", failure.conflict)),
            Some(Err(error)) => Some(format!("dying alternate: {error:?}")),
            None => Some("dying alternate: Type17 metadata".to_owned()),
        }
    } else if native_owner {
        world
            .tasks
            .release_possessed_intro2_type17(
                world.entities,
                body,
                world.retail_tick,
                world.world_fx,
            )
            .err()
            .map(|block| format!("{block:?}"))
    } else {
        match world
            .entities
            .release_possessed_type17(body, world.retail_tick, world.world_fx)
        {
            Ok(crate::type17_impact_reselection::Type17ImpactInstallDecision::FollowBeaconsAcquiring) => {
                world.tasks.adopt_released_type17(world.entities, body);
                None
            }
            // Capture People and Run Away stay out of custody, as their
            // births do.
            Ok(_) => None,
            Err(block) => Some(format!("{block:?}")),
        }
    };
    let at = world
        .entities
        .iter_all()
        .find(|entity| entity.id == body)
        .map(|entity| entity.position_raw());
    let mind_ball = match at {
        Some(at) => world
            .entities
            .construct_mind_ball(at, resources, world.world_fx, world.retail_tick)
            .map_err(|block| format!("{block:?}")),
        None => Err("the body is gone".to_owned()),
    };
    Released {
        body,
        craft,
        mind_ball,
        behavior_stop,
    }
}

/// Why a possessed body's static contact or death stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PossessedContactBlock {
    UnsupportedCraterKind,
    StaticLookup(String),
    UnsupportedStaticKind(u32),
    Damage(String),
    Death(&'static str),
}

/// The static contact's tail in `FUN_00412CF0`'s order: the style's `+1C`
/// hook (`FUN_00447C90`: the controller's force feedback for an inward speed
/// over `0x200`, and `FUN_00427B20`'s destruction of a kind-9 object), then
/// `FUN_00411760`'s collision packet, first to the touched object's cell and
/// then to the body (`FUN_00415040`). A lethal hit runs the body's death,
/// whose initializer releases it: the returned release.
#[allow(clippy::too_many_arguments)]
pub fn deliver_static_contact(
    world: &mut PossessionWorld<'_>,
    resources: &mut ResourceCache,
    static_damage: &mut StaticDamageScheduler,
    notifications: &mut GameplayNotifications,
    contact: PossessedStaticContact,
    custody: PossessionCustody,
) -> Result<Option<Released>, PossessedContactBlock> {
    if let Some((sound, position)) = contact.contact_sound {
        world
            .world_fx
            .queue_fixed_positional_sound_raw(sound, position);
    }
    let (_, haptic) = crate::player_contact_style::player_contact_haptic(
        contact.contact.normal_q12,
        contact.velocity_before_raw,
        HAPTIC_SCALE_RAW,
    );
    if let Some(haptic) = haptic {
        crate::pad_vibration::submit(haptic.strength_raw);
    }
    if contact.contact.kind_index == CRATER_KIND
        && matches!(
            static_damage.submit_crater_destruction(contact.contact.cell, CRATER_KIND),
            StaticCraterOutcome::UnsupportedKind { .. }
        )
    {
        return Err(PossessedContactBlock::UnsupportedCraterKind);
    }
    if contact.impact_raw == 0 {
        return Ok(None);
    }
    let body = custody.state.body;
    let entity_type = world
        .entities
        .iter_all()
        .find(|entity| entity.id == body)
        .map_or(0, |entity| entity.entity_type);
    let delivery = DamageDeliveryRecord {
        packet: DamagePacket::collision(contact.impact_raw),
        source_entity_type_raw: entity_type,
        owner_handle: body,
    };
    if let Some(target) = crate::static_damage_live::resolve_current_static_damage_target(
        resources,
        contact.contact.cell,
    )
    .map_err(|error| PossessedContactBlock::StaticLookup(format!("{error:?}")))?
    {
        match static_damage.submit_hit(target, delivery.packet, &mut || {
            world.world_fx.next_shared_retail_random_u16()
        }) {
            StaticDamageOutcome::UnsupportedKind { kind_index } => {
                return Err(PossessedContactBlock::UnsupportedStaticKind(kind_index))
            }
            StaticDamageOutcome::BurnedKind10Transition { cell, .. } => {
                resources.apply_burned_kind_10_transition(cell);
            }
            StaticDamageOutcome::ImmediateBurn { cell, .. } => {
                crate::static_terrain_burn::apply_immediate_static_burn(
                    cell,
                    resources,
                    world.world_fx,
                )
                .map_err(|error| PossessedContactBlock::StaticLookup(format!("{error:?}")))?;
            }
            _ => {}
        }
    }
    checked_damage(world, resources, notifications, delivery, custody).map(|(released, _)| released)
}

/// `FUN_00410C10` on the possessed body: health 0, the dying bit and the
/// type's death sound; `FUN_0040DB80` finds no cleanup in the Player Control
/// style and installs the body's Type46 source while dying, whose alternate
/// initializer `FUN_00447280` drops the body's cargo (`FUN_00447590`), bursts
/// it (`FUN_004475F0`) and, the craft waiting at `+6C`, releases it.
pub fn die(
    world: &mut PossessionWorld<'_>,
    resources: &ResourceCache,
    custody: PossessionCustody,
) -> Result<Released, PossessedContactBlock> {
    let body = custody.state.body;
    let entity = world
        .entities
        .iter_all()
        .find(|entity| entity.id == body)
        .ok_or(PossessedContactBlock::Death("allocation"))?;
    let metadata = world
        .entities
        .type_runtime_metadata(entity.entity_type)
        .ok_or(PossessedContactBlock::Death("metadata"))?;
    let RetailRuntimeValue::Known(death_sound) = metadata.death_sound_id else {
        return Err(PossessedContactBlock::Death("death sound"));
    };
    if entity.collision.constructor_sound_attachment_id_at_0x8c != RetailRuntimeValue::Known(None) {
        return Err(PossessedContactBlock::Death("sound attachment"));
    }
    if !matches!(&entity.sub_j_attachment_runtime,
        RetailRuntimeValue::Known(Some(hold)) if hold.is_empty())
    {
        // FUN_00447590 pops each row through FUN_00418500.
        return Err(PossessedContactBlock::Death("cargo drop"));
    }
    let entity = world.entities.entity_mut(body).expect("found above");
    entity.collision.health_raw = RetailRuntimeValue::Known(0);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(DYING_STATE_BIT, DYING_STATE_BIT);
    let position = entity.position_raw();
    if let Some(sound) = death_sound {
        world
            .world_fx
            .queue_networked_positional_sound_raw(sound, position);
    }
    // FUN_004475F0: the burst sized by half the radius of the model the
    // dying state selects, and sound 0x3E.
    let RetailRuntimeValue::Known(flags) = entity.collision.state_flags_at_0x08.masked(u32::MAX)
    else {
        return Err(PossessedContactBlock::Death("state"));
    };
    let extent = entity
        .model_in_slot(crate::entity_collision_state::active_model_slot_from_state_flags(flags))
        .and_then(|model| resources.global_model(model))
        .map(|model| model.radius >> 1)
        .ok_or(PossessedContactBlock::Death("dying model"))?;
    let sea_level = resources
        .terrain()
        .and_then(|terrain| terrain.water_enabled().then(|| terrain.sea_level_raw()));
    world
        .world_fx
        .emit_player_wreck_burst_raw(position, extent, sea_level, body);
    Ok(release(world, resources, custody))
}

/// A particle reaching the possessed body: `FUN_00410EB0` (a primary hit
/// stamps the last-hit tick `+34`), or `FUN_00411250`/`FUN_00411320` (the
/// infect or cure model switch), then the common vtable `0x4C8A30`'s
/// `FUN_0040DAC0`/`FUN_0040DA00`/`FUN_0040DA60`, whose style hooks (`+28`,
/// `+20`, `+24`) are null in every Player Control style; then
/// `FUN_00411030`'s impulse and `FUN_00415040`'s checked damage, whose lethal
/// hit runs the body's death and so its release; and the primary hit's
/// suffix (the accepted-hit sound unless dying, and capability bit 3's class
/// 5 particle).
pub fn hit_by_particle(
    world: &mut PossessionWorld<'_>,
    resources: &ResourceCache,
    notifications: &mut GameplayNotifications,
    impact: crate::world_fx::ParticleEntityImpact,
    custody: PossessionCustody,
) -> Result<Option<Released>, PossessedContactBlock> {
    use crate::damage::EntityHitEntry;
    use crate::entity_collision_state::active_model_slot_from_state_flags;
    use crate::impact_reaction::{
        apply_impact_reaction, ImpactReactionBody, IMPACT_REACTION_ENABLED_STATE_BIT,
        IMPACT_REACTION_NETWORKED_STATE_BIT, IMPACT_REACTION_SUPPRESSED_STATE_BIT,
    };
    let id = custody.state.body;
    let delivery = impact
        .packet_delivery()
        .map_err(|()| PossessedContactBlock::Damage("particle provenance".to_owned()))?;
    let entry = impact.entity_hit_entry();
    let primary = !matches!(entry, EntityHitEntry::Infected | EntityHitEntry::Cured);
    if primary {
        world
            .entities
            .entity_mut(id)
            .ok_or(PossessedContactBlock::Death("allocation"))?
            .collision
            .last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(world.retail_tick);
    } else {
        let applied = world
            .entities
            .apply_model_switch_particle_hit_prefix(id, entry)
            .ok_or(PossessedContactBlock::Damage(
                "model-switch prefix".to_owned(),
            ))?;
        if let Some(sound) = applied.sound_id {
            world
                .world_fx
                .queue_fixed_positional_sound_raw(sound, applied.position_raw);
        }
    }
    let Some(delivery) = delivery else {
        return Ok(None);
    };
    let entity = world.entities.entity_mut(id).expect("found above");
    let RetailRuntimeValue::Known(flags) = entity.collision.state_flags_at_0x08.masked(
        IMPACT_REACTION_ENABLED_STATE_BIT
            | IMPACT_REACTION_SUPPRESSED_STATE_BIT
            | IMPACT_REACTION_NETWORKED_STATE_BIT,
    ) else {
        return Err(PossessedContactBlock::Death("state"));
    };
    if flags & IMPACT_REACTION_NETWORKED_STATE_BIT != 0 {
        return Err(PossessedContactBlock::Damage("network impact".to_owned()));
    }
    let mut body = ImpactReactionBody {
        state_flags_at_0x08: flags,
        mass_raw_at_0xb0: entity.mass_raw,
        linear_velocity_xyz_raw: entity.velocity_raw(),
        angular_heading_pitch_roll_raw: entity.rotation_heading_pitch_roll_raw(),
    };
    let fx = &mut *world.world_fx;
    apply_impact_reaction(
        &mut body,
        id,
        delivery.packet.impact_sum_raw(),
        impact.velocity_raw,
        || u32::from(fx.next_shared_retail_random_u16()),
    )
    .map_err(|_| PossessedContactBlock::Damage("zero impact mass".to_owned()))?;
    entity.set_velocity_raw(body.linear_velocity_xyz_raw);
    entity.set_rotation_heading_pitch_roll_raw(body.angular_heading_pitch_roll_raw);
    let entity_type = entity.entity_type;
    let (released, filtered) = checked_damage(world, resources, notifications, delivery, custody)?;
    if primary && filtered != 0 {
        let metadata = world
            .entities
            .type_runtime_metadata(entity_type)
            .ok_or(PossessedContactBlock::Death("metadata"))?;
        let RetailRuntimeValue::Known(sound) = metadata.accepted_hit_presentation_sound_id else {
            return Err(PossessedContactBlock::Damage(
                "accepted-hit sound".to_owned(),
            ));
        };
        let entity = world.entities.entity_mut(id).expect("found above");
        let RetailRuntimeValue::Known(flags) = entity
            .collision
            .state_flags_at_0x08
            .masked(DYING_STATE_BIT | 0x2000 | 0x4000 | 0x8000_0000)
        else {
            return Err(PossessedContactBlock::Death("state"));
        };
        if flags & DYING_STATE_BIT == 0 {
            if let Some(sound) = sound {
                world
                    .world_fx
                    .queue_fixed_positional_sound_raw(sound, entity.position_raw());
            }
        }
        if entity.capability_flags & 8 != 0 {
            let slot = active_model_slot_from_state_flags(flags);
            let model = entity.model_slots[slot].ok_or(PossessedContactBlock::Death("model"))?;
            let extent = resources
                .global_model(model)
                .ok_or(PossessedContactBlock::Death("model"))?
                .radius;
            let mut position = entity.position_raw();
            position[2] = position[2].wrapping_sub(extent as i16);
            world.world_fx.emit_primary_hit_capability_follow_up_raw(
                crate::primary_hit::PrimaryHitCapabilityEmission {
                    target_handle: id,
                    target_allocation_identity: u64::from(id),
                    selected_model_slot: slot,
                    selected_global_model_id: model as u16,
                    position_raw: position,
                    particle_class: 5,
                    particle_scale_raw: 0x800,
                    owner_sign: flags >> 31,
                },
            );
        }
    }
    Ok(released)
}

/// `FUN_00415040` on the possessed body with `delivery`: a lethal hit runs
/// its death, which releases it. Returns the release and the filtered damage.
fn checked_damage(
    world: &mut PossessionWorld<'_>,
    resources: &ResourceCache,
    notifications: &mut GameplayNotifications,
    delivery: DamageDeliveryRecord,
    custody: PossessionCustody,
) -> Result<(Option<Released>, i32), PossessedContactBlock> {
    let mut released = None;
    let tasks = &mut *world.tasks;
    let retail_tick = world.retail_tick;
    let result = apply_live_actor_checked_damage(
        world.entities,
        world.world_fx,
        LiveActorDamageRequest {
            entity_id: custody.state.body,
            delivery,
            entry: LiveActorDamageEntry::Checked,
            ratio_numerator: 0,
            ratio_denominator: 0,
            feedback: Some(LiveActorDamageFeedback {
                notifications,
                retail_tick,
            }),
        },
        |entities, world_fx, _feedback| {
            let mut world = PossessionWorld {
                entities,
                tasks,
                world_fx,
                retail_tick,
            };
            released = Some(die(&mut world, resources, custody)?);
            Ok::<_, PossessedContactBlock>(LiveActorDeathResult {
                returned_nonzero: true,
                publication: None::<()>,
            })
        },
    )
    .map_err(|error| PossessedContactBlock::Damage(format!("{error:?}")))?;
    Ok((released, result.filtered_damage_raw))
}

/// What a shot did to the possessed body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PossessedHitOutcome {
    /// True when the hit killed and so released the body.
    Applied(bool),
    Blocked(PossessedContactBlock),
}

/// [`hit_by_particle`] from the Playing particle visit; a release goes to
/// the scheduler for the session's MindBall.
pub(crate) fn possessed_particle_hit(
    frame: crate::shared_actor_impact::PlayingActorImpactFrame<'_>,
    impact: crate::world_fx::ParticleEntityImpact,
    custody: super::PossessionCustody,
) -> PossessedHitOutcome {
    let result = hit_by_particle(
        &mut PossessionWorld {
            entities: frame.entities,
            tasks: frame.scheduler,
            world_fx: frame.world_fx,
            retail_tick: frame.retail_tick,
        },
        frame.resources,
        frame.notifications,
        impact,
        custody,
    );
    match result {
        Ok(released) => {
            let died = released.is_some();
            if released.is_some() {
                frame.scheduler.possession = None;
                frame.scheduler.possession_released = released;
            }
            PossessedHitOutcome::Applied(died)
        }
        Err(block) => PossessedHitOutcome::Blocked(block),
    }
}

/// `FUN_00414AE0`'s visit of the possessed body: the blast's admission,
/// impulse and checked damage are the common ones (no behavior hook), and a
/// lethal hit runs its death, which releases it inside the damage call.
pub(crate) struct PossessedRadialCallbacks<'a> {
    pub scheduler: &'a mut SpecializedActorTaskScheduler,
    pub resources: &'a ResourceCache,
    pub custody: super::PossessionCustody,
    pub released: Option<Released>,
}

impl crate::entity::DynamicRadialLiveCallbacks for PossessedRadialCallbacks<'_> {
    fn before_native_actor_mutation(&mut self, _: &EntityManager, _: u32) -> bool {
        true
    }

    fn standard_death(
        &mut self,
        manager: &mut EntityManager,
        _id: u32,
        _kind: u32,
        world_fx: &mut WorldFx,
        retail_tick: u32,
        _notifications: &mut GameplayNotifications,
    ) -> Option<
        Result<
            LiveActorDeathResult<crate::entity::DynamicRadialDeathPublication>,
            crate::entity::DynamicRadialLiveBlockReason,
        >,
    > {
        let result = die(
            &mut PossessionWorld {
                entities: manager,
                tasks: self.scheduler,
                world_fx,
                retail_tick,
            },
            self.resources,
            self.custody,
        );
        Some(match result {
            Ok(released) => {
                self.released = Some(released);
                Ok(LiveActorDeathResult {
                    returned_nonzero: true,
                    publication: None,
                })
            }
            Err(block) => Err(crate::entity::DynamicRadialLiveBlockReason::Possessed(
                block,
            )),
        })
    }
}
