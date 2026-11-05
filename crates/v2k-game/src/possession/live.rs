//! The possessed body's live hooks: the release (`FUN_00446D70`), its static
//! contact's style hook (`FUN_00447C90`) and `FUN_00411760` deliveries, and
//! its death, whose `FUN_00447280` initializer releases it.

use super::Possession;
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
    state: Possession,
    native_owner: bool,
) -> Released {
    let Possession { body, craft, .. } = state;
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
    state: Possession,
    native_owner: bool,
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
    let body = state.body;
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
    let mut released = None;
    let tasks = &mut *world.tasks;
    let retail_tick = world.retail_tick;
    let resources = &*resources;
    let result = apply_live_actor_checked_damage(
        world.entities,
        world.world_fx,
        LiveActorDamageRequest {
            entity_id: body,
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
            released = Some(die(&mut world, resources, state, native_owner)?);
            Ok::<_, PossessedContactBlock>(LiveActorDeathResult {
                returned_nonzero: true,
                publication: None::<()>,
            })
        },
    );
    result.map_err(|error| PossessedContactBlock::Damage(format!("{error:?}")))?;
    Ok(released)
}

/// `FUN_00410C10` on the possessed body: health 0, the dying bit and the
/// type's death sound; `FUN_0040DB80` finds no cleanup in the Player Control
/// style and installs the body's Type46 source while dying, whose alternate
/// initializer `FUN_00447280` drops the body's cargo (`FUN_00447590`), bursts
/// it (`FUN_004475F0`) and, the craft waiting at `+6C`, releases it.
pub fn die(
    world: &mut PossessionWorld<'_>,
    resources: &ResourceCache,
    state: Possession,
    native_owner: bool,
) -> Result<Released, PossessedContactBlock> {
    let body = state.body;
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
    Ok(release(world, resources, state, native_owner))
}
