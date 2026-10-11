//! Native living Type13/10/57 11AD0 -> D7F0/D860 surface admission.
//! Search/Move/fallback solid and water hooks are null in retail's style table.
//! A ground-induced Type10/57 death can change the subsequent water hook to
//! Tumble C750; that synchronous terminal callback retains its own adapter.

use crate::native_actor_capture::pair::PlayingPlayerContact;
use crate::{
    entity_collision_state::{active_model_slot_from_state_flags, RetailRuntimeValue},
    flying_surface_contact::{
        resolve_null_hook_flying_surface, surface_bits, FlyingSurfaceContactBlock,
        FlyingSurfaceContactOutcome,
    },
    intro2_contacts::Intro2ContactFrame,
    intro2_type10::{
        death::{Intro2Type10DeathBlock, Intro2Type10TumbleContact},
        Intro2Type10TumbleOwner,
    },
    intro2_type57::{Intro2Type57DeathBlock, Intro2Type57TumbleContact, Intro2Type57TumbleOwner},
    live_actor_checked_damage::LiveActorDeathResult,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeFlyingSurfaceDeathBlock {
    Class1(Box<crate::class49_terminal::Class49TerminalBlock>),
    RuntimeClass1ContextUnavailable,
    /// Class63 carriers finish BAF0/BC90 through the lent radial owner.
    AutoPilot(Box<crate::class49_terminal::Class49TerminalBlock>),
    RuntimeAutoPilotContextUnavailable,
    Type10(Intro2Type10DeathBlock),
    Type57(Intro2Type57DeathBlock),
    Type10Water(crate::intro2_type10::contact::Intro2Type10ContactBlock),
    Type57Water(crate::intro2_type57::contact::Intro2Type57ContactBlock),
    Runtime(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeFlyingSurfaceDeathPublication {
    Type10(Intro2Type10TumbleOwner),
    Type57(Intro2Type57TumbleOwner),
    QuietDeath(crate::main_base_abort::MainBaseAbortActorLease),
}

pub type NativeFlyingSurfaceContactBlock =
    FlyingSurfaceContactBlock<NativeFlyingSurfaceDeathBlock, NativeFlyingSurfaceDeathPublication>;
pub type NativeFlyingSurfaceContactOutcome =
    FlyingSurfaceContactOutcome<NativeFlyingSurfaceDeathBlock, NativeFlyingSurfaceDeathPublication>;

pub(crate) fn publish_native_flying_standard_death(
    manager: &mut crate::entity::EntityManager,
    id: u32,
    world_fx: &mut crate::world_fx::WorldFx,
) -> Result<LiveActorDeathResult<NativeFlyingSurfaceDeathPublication>, NativeFlyingSurfaceDeathBlock>
{
    let entity_type = manager
        .iter_all()
        .find(|e| e.id == id)
        .ok_or(NativeFlyingSurfaceDeathBlock::Runtime("death allocation"))?
        .entity_type;
    let publication = match entity_type {
        10 | 5 => {
            crate::intro2_type10::death::publish_intro2_type10_standard_death(manager, id, world_fx)
                .map(|owner| owner.map(NativeFlyingSurfaceDeathPublication::Type10))
                .map_err(NativeFlyingSurfaceDeathBlock::Type10)?
        }
        57 => crate::intro2_type57::publish_intro2_type57_standard_death(manager, id, world_fx)
            .map(|owner| owner.map(NativeFlyingSurfaceDeathPublication::Type57))
            .map_err(NativeFlyingSurfaceDeathBlock::Type57)?,
        15 | 87 => publish_native_flyer_quiet_death(manager, id, world_fx)?
            .map(NativeFlyingSurfaceDeathPublication::QuietDeath),
        13 => return Err(NativeFlyingSurfaceDeathBlock::RuntimeClass1ContextUnavailable),
        80 | 126 => return Err(NativeFlyingSurfaceDeathBlock::RuntimeAutoPilotContextUnavailable),
        _ => return Err(NativeFlyingSurfaceDeathBlock::Runtime("death type")),
    };
    Ok(LiveActorDeathResult {
        returned_nonzero: publication.is_some(),
        publication,
    })
}

/// The call site lends the complete synchronous radial context. Type13
/// completes BAC0 before checked damage returns; no synthetic dying owner is
/// published. Type10/57 and quiet15/87 retain their distinct native receipts.
pub(crate) fn run_native_flying_standard_death(
    frame: crate::class49_terminal::Class49TerminalFrame<'_>,
    id: u32,
) -> Result<LiveActorDeathResult<NativeFlyingSurfaceDeathPublication>, NativeFlyingSurfaceDeathBlock>
{
    let auto_pilot = frame.entities.iter_all().any(|entity| {
        entity.id == id && crate::intro2_type10::type10_auto_pilot_profile(entity).is_some()
    });
    if auto_pilot
        || frame
            .entities
            .iter_all()
            .any(|entity| entity.id == id && entity.entity_type == 13)
    {
        crate::class49_terminal::run_class49_standard_death(frame, id)
            .map(|result| LiveActorDeathResult {
                returned_nonzero: result.returned_nonzero,
                publication: None,
            })
            .map_err(|error| {
                if auto_pilot {
                    NativeFlyingSurfaceDeathBlock::AutoPilot(Box::new(error))
                } else {
                    NativeFlyingSurfaceDeathBlock::Class1(Box::new(error))
                }
            })
    } else {
        publish_native_flying_standard_death(frame.entities, id, frame.world_fx)
    }
}

pub(crate) fn register_native_flying_death(
    scheduler: &mut crate::specialized_actor_task_production::SpecializedActorTaskScheduler,
    publication: NativeFlyingSurfaceDeathPublication,
) {
    match publication {
        NativeFlyingSurfaceDeathPublication::Type10(owner) => {
            scheduler.register_intro2_type10_tumble(owner)
        }
        NativeFlyingSurfaceDeathPublication::Type57(owner) => {
            scheduler.register_intro2_type57_tumble(owner)
        }
        NativeFlyingSurfaceDeathPublication::QuietDeath(allocation) => {
            scheduler.retire_native_flyer_quiet_death(allocation)
        }
    }
}

/// Native15/87 authors alternate2, not a falling corpse task. 10C10 stops
/// sound11 before DB80/AC60 enters the zero-policy C470 terminal initializer.
fn publish_native_flyer_quiet_death(
    manager: &mut crate::entity::EntityManager,
    id: u32,
    world_fx: &mut crate::world_fx::WorldFx,
) -> Result<Option<crate::main_base_abort::MainBaseAbortActorLease>, NativeFlyingSurfaceDeathBlock>
{
    use NativeFlyingSurfaceDeathBlock as Block;
    let entity = manager
        .iter_all()
        .find(|e| e.id == id)
        .ok_or(Block::Runtime("quiet death allocation"))?;
    if !crate::intro2_flyers_live::flyer_manager_identity_authenticates(manager, id) {
        return Err(Block::Runtime("quiet death native flyer allocation"));
    }
    let flags = match entity.collision.state_flags_at_0x08.masked(0x8000_4000) {
        RetailRuntimeValue::Known(flags) => flags,
        _ => return Err(Block::Runtime("quiet death state")),
    };
    if flags != 0 {
        return Ok(None);
    }
    crate::intro2_flyers_live::Intro2FlyerSchedulerOwner::adopt_published(entity)
        .map_err(|_| Block::Runtime("quiet death native graph"))?;
    let metadata = manager
        .type_runtime_metadata(entity.entity_type)
        .ok_or(Block::Runtime("quiet death metadata"))?;
    crate::intro2_flyers_live::authenticate_flyer_mover_metadata(entity.entity_type, metadata)
        .map_err(|_| Block::Runtime("quiet death B/D/E/G descriptors"))?;
    if metadata
        .initializer
        .as_ref()
        .map(|init| init.alternate_behavior_class_ref)
        != Some(2)
        || metadata.mass_raw != 100
        || metadata.capability_flags != 8
        || metadata.model_slots.map(usize::from).map(Some) != entity.model_slots
        || metadata.constructor_sound_attachment_id != RetailRuntimeValue::Known(Some(11))
        || entity.collision.constructor_sound_attachment_id_at_0x8c
            != RetailRuntimeValue::Known(Some(11))
    {
        return Err(Block::Runtime("quiet death authored profile"));
    }
    let RetailRuntimeValue::Known(death_sound) = metadata.death_sound_id else {
        return Err(Block::Runtime("quiet death sound"));
    };
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(Block::Runtime("quiet death context"));
    };
    if context.active_style().death_callback_policy()
        != crate::entity_behavior::DeathCallbackPolicy::None
    {
        return Err(Block::Runtime("quiet death hook"));
    }
    let selected = context
        .reselect_audited_type_default(
            &crate::entity_behavior::QUIET_DEATH_BEHAVIOR_PROGRAM,
            0,
            crate::entity_behavior::QUIET_DEATH_STYLE,
        )
        .ok_or(Block::Runtime("quiet death alternate context"))?;
    let allocation = manager
        .main_base_abort_actor_observation(id)
        .ok_or(Block::Runtime("quiet death lease"))?
        .lease;
    let entity = manager.entity_mut(id).unwrap();
    entity.collision.health_raw = RetailRuntimeValue::Known(0);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(0x4000, 0x4000);
    if let Some(sound) = death_sound {
        world_fx.queue_fixed_positional_sound_raw(sound, entity.position_raw());
    }
    entity.collision.constructor_sound_attachment_id_at_0x8c = RetailRuntimeValue::Known(None);
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(selected));
    for slot in crate::actor_task_owner::ActorTaskSlot::IN_RETAIL_TICK_ORDER {
        entity.actor_tasks.clear_slot(slot);
    }
    entity.mark_actor_deferred_destroy_pending();
    manager.queue_actor_deferred_destroy(id);
    Ok(Some(allocation))
}

/// Completed native C470 source whose handle still belongs to the current
/// deferred live-list walk. This does not manufacture a replacement task.
pub(crate) fn native_flyer_quiet_death_authenticates(
    manager: &crate::entity::EntityManager,
    id: u32,
) -> bool {
    let Some(entity) = manager.iter_all().find(|e| e.id == id) else {
        return false;
    };
    crate::intro2_flyers_live::flyer_manager_identity_authenticates(manager, id)
        && manager.main_base_abort_actor_observation(id).is_some()
        && manager.pending_actor_deferred_destroy_ids().contains(&id)
        && entity.collision.health_raw == RetailRuntimeValue::Known(0)
        && entity.collision.state_flags_at_0x08.masked(0x0010_4000)
            == RetailRuntimeValue::Known(0x0010_4000)
        && entity.collision.constructor_sound_attachment_id_at_0x8c
            == RetailRuntimeValue::Known(None)
        && matches!(entity.current_behavior_context, RetailRuntimeValue::Known(Some(context))
            if context.descriptor() == crate::entity_behavior::BehaviorDescriptorIdentity::Named(
                &crate::entity_behavior::QUIET_DEATH_BEHAVIOR_PROGRAM)
                && context.active_style().style_address() == 0x004c7420)
        && crate::actor_task_owner::ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .all(|slot| entity.actor_tasks.task_in_slot(slot).is_none())
}

pub fn resolve_native_flying_surface_contact(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
) -> NativeFlyingSurfaceContactOutcome {
    resolve_native_flying_surface_contact_with_playing(frame, id, None)
}

/// Playing's late walk lends its player to a Class1 contact death's radial.
pub fn resolve_native_flying_surface_contact_with_playing(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
    playing: Option<PlayingPlayerContact<'_>>,
) -> NativeFlyingSurfaceContactOutcome {
    let mut committed = false;
    match resolve(frame, id, playing, &mut committed) {
        Ok(outcome) => outcome,
        Err(reason) => {
            if committed {
                frame
                    .actor_tasks
                    .park_native_contact_prefix(frame.entities, id);
            }
            NativeFlyingSurfaceContactOutcome::Blocked {
                reason,
                committed_prefix: committed,
            }
        }
    }
}

fn resolve(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
    playing: Option<PlayingPlayerContact<'_>>,
    committed: &mut bool,
) -> Result<NativeFlyingSurfaceContactOutcome, NativeFlyingSurfaceContactBlock> {
    use FlyingSurfaceContactBlock as Block;
    let entity = frame
        .entities
        .iter_all()
        .find(|e| e.id == id)
        .ok_or(Block::Runtime("allocation"))?;
    if !entity.active || !matches!(entity.entity_type, 13 | 10 | 5 | 80 | 126 | 57) {
        return Ok(FlyingSurfaceContactOutcome::Ineligible);
    }
    let entity_type = entity.entity_type;
    // Entry gates precede graph custody and preserve native policy exclusions.
    if surface_bits(entity, 0x8000)? == 0 || surface_bits(entity, 0x1000)? != 0 {
        return Ok(FlyingSurfaceContactOutcome::Ineligible);
    }
    match entity.collision.subject_scan_gate_at_0x70 {
        RetailRuntimeValue::Known(0) => {}
        RetailRuntimeValue::Known(_) => return Ok(FlyingSurfaceContactOutcome::Ineligible),
        _ => return Err(Block::Runtime("subject +70")),
    }
    let model_id = entity
        .model_in_slot(active_model_slot_from_state_flags(surface_bits(
            entity, 0x6000,
        )?))
        .ok_or(Block::Runtime("entry model"))?;
    let model = frame
        .resources
        .global_model(model_id)
        .ok_or(Block::Runtime("entry model"))?;
    if model.collision_radius_raw == 0
        || surface_bits(entity, 0x8800_0000)? != 0
        || surface_bits(entity, 0x10000)? == 0
    {
        return Ok(FlyingSurfaceContactOutcome::Ineligible);
    }
    let RetailRuntimeValue::Known(Some(style)) = entity.current_behavior_context else {
        return Err(Block::Runtime("current style"));
    };
    let style = style.active_style().style_address();
    if matches!(style, 0x004c7f60 | 0x004c7fa8) {
        // The existing Tumble resolver owns its C750 solid/water/static hooks.
        return Ok(FlyingSurfaceContactOutcome::Ineligible);
    }
    // 11AD0 has no dying test. A finished BAC0/BC90 corpse keeps class1's or
    // class63's style, whose solid/water hooks are null, until 14990.
    let finished = crate::class49_death::finished_terminal_hit_authenticates(frame.entities, id);
    let null_hooks = matches!(
        style,
        0x004c7a50 | 0x004c7a98 | 0x004c7ae0 | 0x004c7930 | 0x004c7978 | 0x004c74f8
    ) || (finished && matches!(style, 0x004c7150 | 0x004c7198));
    if !null_hooks {
        return Err(Block::Runtime("unaudited solid/water style hooks"));
    }
    let metadata = frame
        .entities
        .type_runtime_metadata(entity_type)
        .ok_or(Block::Runtime("metadata"))?;
    let (allocation, pending, completed) = match entity_type {
        10 | 5 | 80 | 126 => {
            let profile = crate::intro2_type10::Type10Profile::from_entity_type(entity_type)
                .expect("matched Type10-family row");
            crate::intro2_type10::authenticate_metadata(profile, metadata)
                .map_err(|_| Block::Runtime("Type10 metadata"))?;
            (
                crate::intro2_type10::intro2_type10_allocation_authenticates(entity),
                frame.actor_tasks.intro2_type10_has_pending_prefix(id),
                frame
                    .actor_tasks
                    .intro2_type10_completed_owner(frame.entities, id),
            )
        }
        57 => {
            crate::intro2_type57::authenticate_metadata(metadata)
                .map_err(|_| Block::Runtime("Type57 metadata"))?;
            (
                crate::intro2_type57::intro2_type57_allocation_authenticates(entity),
                frame.actor_tasks.intro2_type57_has_pending_prefix(id),
                frame
                    .actor_tasks
                    .intro2_type57_completed_owner(frame.entities, id),
            )
        }
        13 => (
            crate::intro2_type13_live::authenticate_intro2_type13(entity).is_ok(),
            frame.actor_tasks.intro2_type13_has_pending_prefix(id),
            frame
                .actor_tasks
                .intro2_type13_completed_owner(frame.entities, id),
        ),
        _ => unreachable!(),
    };
    if !allocation {
        return Err(Block::Runtime("native surface allocation"));
    }
    if pending || !(completed || finished) {
        return Err(Block::Runtime("current completed contact owner"));
    }
    let record = frame
        .resources
        .global_entity_type(entity_type as usize)
        .ok_or(Block::Runtime("type record"))?;
    let solid_sound = u16::from_le_bytes([record.raw_header[0x86], record.raw_header[0x87]]);
    let water_sound = u16::from_le_bytes([record.raw_header[0x88], record.raw_header[0x89]]);
    resolve_null_hook_flying_surface(
        frame,
        id,
        model_id,
        solid_sound,
        water_sound,
        playing,
        committed,
        |death_frame| run_native_flying_standard_death(death_frame, id),
        register_native_flying_death,
        |frame, id, playing, committed| {
            let entity = frame.entities.iter_all().find(|e| e.id == id).ok_or(
                NativeFlyingSurfaceDeathBlock::Runtime("water callback allocation"),
            )?;
            let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
                return Err(NativeFlyingSurfaceDeathBlock::Runtime(
                    "water callback style",
                ));
            };
            if context.active_style().style_address() != 0x004c7f60 {
                return Ok(());
            }
            let mut playing = playing;
            match entity_type {
                10 | 5 | 80 | 126 => {
                    let owner = Intro2Type10TumbleOwner::adopt(frame.entities, id)
                        .map_err(NativeFlyingSurfaceDeathBlock::Type10)?;
                    crate::intro2_type10::contact::terminal_callback(
                        frame,
                        id,
                        owner,
                        Intro2Type10TumbleContact::Water,
                        &mut Default::default(),
                        &mut playing,
                        committed,
                    )
                    .map_err(NativeFlyingSurfaceDeathBlock::Type10Water)
                }
                // Intro2-only Type57's C750 radial owns only its cinematic walk.
                57 if playing.is_some() => Err(NativeFlyingSurfaceDeathBlock::Runtime(
                    "Playing Type57 Tumble water radial",
                )),
                57 => {
                    let owner = Intro2Type57TumbleOwner::adopt(frame.entities, id)
                        .map_err(NativeFlyingSurfaceDeathBlock::Type57)?;
                    crate::intro2_type57::contact::terminal_callback(
                        frame,
                        id,
                        owner,
                        Intro2Type57TumbleContact::Water,
                        &mut Default::default(),
                        committed,
                    )
                    .map_err(NativeFlyingSurfaceDeathBlock::Type57Water)
                }
                _ => Ok(()),
            }
        },
    )
}

#[cfg(test)]
mod tests;
