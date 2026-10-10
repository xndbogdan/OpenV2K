//! Class11's late `11AD0` terrain -> water -> static contact and physical tails.
//!
//! C750/BAC0 destroys tasks and schedules the actor's later splice. It does
//! not invalidate its handle: D7F0/D860 still run141D0, and D920 runs11760.
//! The entry model and outer scan gates survive those synchronous callbacks.

use super::{death::*, intro2_type10_allocation_authenticates};
use crate::{
    class49_terminal::Class49RadialReport,
    damage::{velocity_delta_impact_raw, DamageDeliveryRecord, DamagePacket},
    entity::Entity,
    entity_collision_state::{active_model_slot_from_state_flags, RetailRuntimeValue},
    intro2_contacts::Intro2ContactFrame,
    intro2_meteors::plan_meteor_terrain_response,
    intro2_radial::{
        apply_intro2_radial_damage, apply_static_radial_damage, Intro2RadialFrame,
        Intro2RadialTerminalCall,
    },
    live_actor_checked_damage::{
        apply_live_actor_checked_damage, LiveActorDamageEntry, LiveActorDamageError,
        LiveActorDamageRequest,
    },
    native_actor_capture::pair::PlayingPlayerContact,
    specialized_actor_task_production::playing_radial::PlayingRadialFrame,
    static_contact::{
        apply_contact_response_raw, scan_deepest_static_contact, StaticContactError,
        StaticContactQuery, StaticModelContact,
    },
    static_damage::StaticDamageOutcome,
    static_damage_live::{resolve_current_static_damage_target, CurrentStaticDamageLookupError},
    terrain_contact::TerrainModelContact,
    type60_exploding_ring::{HardWaterType60ConstructionRequest, HardWaterType60Severity},
    whole_body_surface::{
        classify_whole_body_surface, nearest_cell_material_code,
        plan_fallback_whole_body_surface_response, WholeBodySurfaceClassificationRequest,
        WholeBodySurfaceFallbackResponseRequest, WholeBodySurfaceResponse,
    },
    world_fx::{ParticleEnvironment, TerrainCollisionContext, WholeBodyContactScatter},
};
use v2k_formats::models::ModelCollisionError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2Type10ContactBlock {
    Runtime(&'static str),
    Geometry(ModelCollisionError),
    StaticScan(StaticContactError),
    StaticLookup(CurrentStaticDamageLookupError),
    UnsupportedStaticKind(u32),
    Death(Intro2Type10DeathBlock),
    Damage(LiveActorDamageError<&'static str, ()>),
    RadialIncomplete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Intro2Type10TerminalContactReport {
    pub contact: Intro2Type10TumbleContact,
    /// Cinematic for Intro2's walk; Playing when the walk lends its player.
    pub radial: Class49RadialReport,
    pub finalized: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Intro2Type10ContactReport {
    pub terrain_contact: bool,
    pub water_entry: bool,
    pub static_contact: Option<StaticModelContact>,
    pub terminal: Option<Intro2Type10TerminalContactReport>,
    pub collision_damage_raw: i32,
    pub collision_static_damage: Option<StaticDamageOutcome>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2Type10ContactOutcome {
    Ineligible,
    Applied(Intro2Type10ContactReport),
    Blocked {
        reason: Intro2Type10ContactBlock,
        committed_prefix: bool,
        report: Intro2Type10ContactReport,
    },
}

pub fn resolve_intro2_type10_tumble_contact(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
) -> Intro2Type10ContactOutcome {
    resolve_intro2_type10_tumble_contact_with_playing(frame, id, None)
}

/// Playing's walk lends its player to C750's terminal radial.
pub fn resolve_intro2_type10_tumble_contact_with_playing(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
    mut playing: Option<PlayingPlayerContact<'_>>,
) -> Intro2Type10ContactOutcome {
    let mut report = Intro2Type10ContactReport::default();
    let mut committed = false;
    match resolve(frame, id, &mut playing, &mut report, &mut committed) {
        Ok(false) => Intro2Type10ContactOutcome::Ineligible,
        Ok(true) => Intro2Type10ContactOutcome::Applied(report),
        Err(reason) => {
            if committed
                && !report
                    .terminal
                    .as_ref()
                    .is_some_and(|terminal| terminal.finalized)
            {
                frame
                    .actor_tasks
                    .park_intro2_type10_tumble_contact_prefix(id);
            }
            Intro2Type10ContactOutcome::Blocked {
                reason,
                committed_prefix: committed,
                report,
            }
        }
    }
}

fn bits(entity: &Entity, mask: u32) -> Result<u32, Intro2Type10ContactBlock> {
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(bits) => Ok(bits),
        _ => Err(Intro2Type10ContactBlock::Runtime("contact state")),
    }
}

fn resolve(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
    playing: &mut Option<PlayingPlayerContact<'_>>,
    report: &mut Intro2Type10ContactReport,
    committed: &mut bool,
) -> Result<bool, Intro2Type10ContactBlock> {
    use Intro2Type10ContactBlock as Block;
    let entity = frame
        .entities
        .iter_all()
        .find(|e| e.id == id)
        .ok_or(Block::Runtime("allocation"))?;
    if !intro2_type10_allocation_authenticates(entity) {
        return Ok(false);
    }
    if bits(entity, 0x8000)? == 0 || bits(entity, 0x1000)? != 0 {
        return Ok(false);
    }
    match entity.collision.subject_scan_gate_at_0x70 {
        RetailRuntimeValue::Known(0) => {}
        RetailRuntimeValue::Known(_) => return Ok(false),
        _ => return Err(Block::Runtime("subject +70")),
    }
    let model_id = entity
        .model_in_slot(active_model_slot_from_state_flags(bits(entity, 0x6000)?))
        .ok_or(Block::Runtime("entry model"))?;
    let model = frame
        .resources
        .global_model(model_id)
        .ok_or(Block::Runtime("entry model"))?;
    let radius = model.collision_radius_raw;
    if radius == 0 || bits(entity, 0x8800_0000)? != 0 {
        return Ok(false);
    }
    // The central walker also encounters native living Search/fallback actors;
    // those require their own contact policy, not a class11 task receipt.
    if matches!(
        style(entity)?,
        0x004c7a50 | 0x004c7a98 | 0x004c7ae0 | 0x004c74f8
    ) {
        return Ok(false);
    }
    // Search's pair hook and Tumble's null +18 do not authorize a pair explosion.
    let owner = Intro2Type10TumbleOwner::adopt(frame.entities, id).map_err(Block::Death)?;
    if !frame
        .actor_tasks
        .intro2_type10_tumble_completed_owner(frame.entities, id)
    {
        return Err(Block::Runtime("current completed Tumble owner"));
    }
    if style(entity)? != 0x004c7f60 {
        return Err(Block::Runtime("falling Tumble style"));
    }
    let terrain_water = bits(entity, 0x10000)? != 0;
    if terrain_water {
        let terrain = frame
            .resources
            .level_terrain()
            .ok_or(Block::Runtime("terrain"))?;
        let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
            return Err(Block::Runtime("physical basis"));
        };
        let hit = model
            .collide_terrain_raw_oriented(
                terrain,
                entity.position_raw(),
                basis
                    .orientation_world_from_model()
                    .map(|row| row.map(f64::from)),
                &entity.presentation_anim_vars(frame.retail_tick),
            )
            .map_err(Block::Geometry)?;
        if let Some(hit) = hit {
            let p = entity.position_raw();
            let material = nearest_cell_material_code(terrain, p[0], p[2]);
            let contact = TerrainModelContact {
                normal_q12: hit.normal.map(|value| (value * 4096.0).round() as i16),
                penetration_raw: hit.penetration_raw as i32,
            };
            report.terrain_contact = true;
            terminal_callback(
                frame,
                id,
                owner,
                Intro2Type10TumbleContact::Terrain,
                report,
                playing,
                committed,
            )?;
            apply_terrain_tail(frame, id, contact, material, report, committed)?;
        }
        classify_and_respond_water(frame, id, owner, radius, report, playing, committed)?;
    }
    resolve_static_phase(frame, id, Some(owner), model_id, report, playing, committed)?;
    Ok(true)
}

/// Continue the static suffix of a living11AD0 visit after its terrain/water
/// phases, retaining the original selected model. This does not replay solid
/// contact, water classification, or the source entry gates after callbacks.
pub(crate) fn resolve_intro2_type10_tumble_static_continuation(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
    entry_model_id: usize,
) -> Intro2Type10ContactOutcome {
    resolve_intro2_type10_tumble_static_continuation_with_playing(frame, id, entry_model_id, None)
}

pub(crate) fn resolve_intro2_type10_tumble_static_continuation_with_playing(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
    entry_model_id: usize,
    mut playing: Option<PlayingPlayerContact<'_>>,
) -> Intro2Type10ContactOutcome {
    let mut report = Intro2Type10ContactReport::default();
    let mut committed = false;
    let result = (|| {
        use Intro2Type10ContactBlock as Block;
        let entity = frame
            .entities
            .iter_all()
            .find(|entity| entity.id == id)
            .ok_or(Block::Runtime("static continuation allocation"))?;
        if !intro2_type10_allocation_authenticates(entity) {
            return Err(Block::Runtime("static continuation native allocation"));
        }
        let owner = match style(entity)? {
            0x004c7f60 => {
                let owner =
                    Intro2Type10TumbleOwner::adopt(frame.entities, id).map_err(Block::Death)?;
                if !frame
                    .actor_tasks
                    .intro2_type10_tumble_completed_owner(frame.entities, id)
                {
                    return Err(Block::Runtime("static continuation completed Tumble owner"));
                }
                Some(owner)
            }
            0x004c7fa8 => None, // BAC0 completed: style hooks are now null.
            _ => return Ok(false),
        };
        resolve_static_phase(
            frame,
            id,
            owner,
            entry_model_id,
            &mut report,
            &mut playing,
            &mut committed,
        )?;
        Ok(true)
    })();
    match result {
        Ok(false) => Intro2Type10ContactOutcome::Ineligible,
        Ok(true) => Intro2Type10ContactOutcome::Applied(report),
        Err(reason) => {
            if committed
                && !report
                    .terminal
                    .as_ref()
                    .is_some_and(|terminal| terminal.finalized)
            {
                frame
                    .actor_tasks
                    .park_intro2_type10_tumble_contact_prefix(id);
            }
            Intro2Type10ContactOutcome::Blocked {
                reason,
                committed_prefix: committed,
                report,
            }
        }
    }
}

fn resolve_static_phase(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
    owner: Option<Intro2Type10TumbleOwner>,
    model_id: usize,
    report: &mut Intro2Type10ContactReport,
    playing: &mut Option<PlayingPlayerContact<'_>>,
    committed: &mut bool,
) -> Result<(), Intro2Type10ContactBlock> {
    use Intro2Type10ContactBlock as Block;
    // Static eligibility uses the ENTRY flags. BAC0's pending-splice write
    // cannot cancel this scan in the same11AD0 invocation.
    let entity = frame
        .entities
        .iter_all()
        .find(|e| e.id == id)
        .ok_or(Block::Runtime("static survivor"))?;
    let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
        return Err(Block::Runtime("static basis"));
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
        active_model: frame
            .resources
            .global_model(model_id)
            .ok_or(Block::Runtime("entry model"))?,
        active_model_to_world_basis: basis
            .orientation_world_from_model()
            .map(|row| row.map(f64::from)),
        active_anim_vars: &entity.presentation_anim_vars(frame.retail_tick),
        position_raw: entity.position_raw(),
    })
    .map_err(Block::StaticScan)?;
    if let Some(contact) = contact {
        report.static_contact = Some(contact);
        //12CF0: own +8A,27E20(no player capability),A8B0(Tumble task+20
        // remains405FF0-null),D920(no effective400),then own style+1C.
        queue_type_cue(frame, id, 0x8a)?;
        let entity = frame
            .entities
            .entity_mut(id)
            .ok_or(Block::Runtime("static survivor"))?;
        if entity.capability_flags != 8 {
            return Err(Block::Runtime("static pickup policy"));
        }
        if entity.collision.default_state_flags_at_0xc8 != RetailRuntimeValue::Known(8) {
            return Err(Block::Runtime("static crush policy"));
        }
        if owner.is_some() {
            terminal_callback(
                frame,
                id,
                owner.expect("falling Tumble completed owner"),
                Intro2Type10TumbleContact::Static,
                report,
                playing,
                committed,
            )?;
        }
        apply_static_tail(frame, id, contact, report, committed)?;
    }
    Ok(())
}

fn style(entity: &Entity) -> Result<u32, Intro2Type10ContactBlock> {
    match entity.current_behavior_context {
        RetailRuntimeValue::Known(Some(context)) => Ok(context.active_style().style_address()),
        _ => Err(Intro2Type10ContactBlock::Runtime("current style")),
    }
}

pub(crate) fn terminal_callback(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
    owner: Intro2Type10TumbleOwner,
    contact: Intro2Type10TumbleContact,
    report: &mut Intro2Type10ContactReport,
    playing: &mut Option<PlayingPlayerContact<'_>>,
    committed: &mut bool,
) -> Result<(), Intro2Type10ContactBlock> {
    use Intro2Type10ContactBlock as Block;
    let entity = frame
        .entities
        .iter_all()
        .find(|e| e.id == id)
        .ok_or(Block::Runtime("callback survivor"))?;
    match style(entity)? {
        0x004c7fa8 => return Ok(()), // completion style's three hooks are null
        0x004c7f60 => {}
        _ => return Err(Block::Runtime("Tumble contact style")),
    }
    let receipt = begin_intro2_type10_tumble_contact(
        frame.entities,
        id,
        frame.resources,
        frame.world_fx,
        contact,
    )
    .map_err(Block::Death)?
    .ok_or(Block::Runtime("pending terminal receipt"))?;
    *committed = true;
    if !claim_intro2_type10_terminal(frame.entities, &receipt) {
        return Err(Block::Runtime("terminal claim"));
    }
    let radial = match playing.as_mut() {
        None => Class49RadialReport::Cinematic(apply_intro2_radial_damage(
            &mut Intro2RadialFrame {
                active_terminal_calls: vec![Intro2RadialTerminalCall::Type10(receipt)],
                entities: frame.entities,
                resources: frame.resources,
                world_fx: frame.world_fx,
                static_damage: frame.static_damage,
                notifications: frame.notifications,
                retail_tick: frame.retail_tick,
                actor_tasks: frame.actor_tasks,
            },
            receipt.position_raw,
            receipt.radial_damage,
        )),
        // Playing's same static prefix, then its dynamic owner with the hull.
        Some(player) => {
            let static_deliveries = apply_static_radial_damage(
                frame.resources,
                frame.static_damage,
                frame.world_fx,
                receipt.position_raw,
                receipt.radial_damage,
            )
            .map_err(Block::StaticLookup)?;
            let dynamic = frame
                .actor_tasks
                .apply_playing_radial_damage(PlayingRadialFrame {
                    entities: frame.entities,
                    player_hull: player.hull,
                    extra_lives: player.extra_lives,
                    origin_raw: receipt.position_raw,
                    template: receipt.radial_damage,
                    world_fx: frame.world_fx,
                    notifications: frame.notifications,
                    retail_tick: frame.retail_tick,
                    resources: frame.resources,
                    static_damage: frame.static_damage,
                    active_terminal_calls: vec![Intro2RadialTerminalCall::Type10(receipt)],
                });
            Class49RadialReport::Playing {
                static_deliveries,
                dynamic,
            }
        }
    };
    let completed = radial.completed();
    let finalized = completed && finish_intro2_type10_terminal(frame.entities, receipt);
    report.terminal = Some(Intro2Type10TerminalContactReport {
        contact,
        radial,
        finalized,
    });
    if !finalized {
        return Err(Block::RadialIncomplete);
    }
    frame.actor_tasks.retire_intro2_type10_tumble(owner);
    Ok(())
}

fn queue_type_cue(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
    offset: usize,
) -> Result<(), Intro2Type10ContactBlock> {
    use Intro2Type10ContactBlock as Block;
    let row = frame
        .entities
        .iter_all()
        .find(|entity| entity.id == id)
        .map(|entity| entity.entity_type as usize)
        .ok_or(Block::Runtime("type record"))?;
    let header = &frame
        .resources
        .global_entity_type(row)
        .ok_or(Block::Runtime("type record"))?
        .raw_header;
    let cue = u16::from_le_bytes(
        header
            .get(offset..offset + 2)
            .ok_or(Block::Runtime("contact cue"))?
            .try_into()
            .unwrap(),
    );
    if cue != 0 {
        let p = frame
            .entities
            .entity_mut(id)
            .ok_or(Block::Runtime("cue survivor"))?
            .position_raw();
        frame.world_fx.queue_fixed_positional_sound_raw(cue, p);
    }
    Ok(())
}

fn checked_tail_damage(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
    amount: i32,
    source: u32,
) -> Result<(), Intro2Type10ContactBlock> {
    if amount == 0 {
        return Ok(());
    }
    let own_type = frame
        .entities
        .iter_all()
        .find(|entity| entity.id == id)
        .map(|entity| entity.entity_type)
        .ok_or(Intro2Type10ContactBlock::Runtime("tail damage survivor"))?;
    apply_live_actor_checked_damage::<(), &'static str>(
        frame.entities,
        frame.world_fx,
        LiveActorDamageRequest {
            ratio_numerator: 0,
            ratio_denominator: 0,
            feedback: None,
            entity_id: id,
            delivery: DamageDeliveryRecord {
                packet: DamagePacket::collision(amount),
                source_entity_type_raw: source,
                owner_handle: if source == own_type { id } else { 0 },
            },
            entry: LiveActorDamageEntry::Checked,
        },
        |_, _, _feedback| Err("already-dying Tumble actor death"),
    )
    .map(|_| ())
    .map_err(Intro2Type10ContactBlock::Damage)
}

fn apply_terrain_tail(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
    contact: TerrainModelContact,
    material: u8,
    report: &mut Intro2Type10ContactReport,
    committed: &mut bool,
) -> Result<(), Intro2Type10ContactBlock> {
    use Intro2Type10ContactBlock as Block;
    let context = TerrainCollisionContext::from_current_level_cache(frame.resources)
        .ok_or(Block::Runtime("terrain response environment"))?;
    let entity = frame
        .entities
        .entity_mut(id)
        .ok_or(Block::Runtime("terrain-tail survivor"))?;
    let own_type = entity.entity_type;
    let response = plan_meteor_terrain_response(
        entity.position_raw(),
        entity.velocity_raw(),
        entity.mass_raw,
        contact,
    );
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(0x800000, 0x800000);
    entity.set_position_raw(response.position_raw);
    *committed = true;
    if response.particle_scale_raw != 0 {
        // Type10 +86 is null; unlike +88/+8A, a nonzero solid cue needs the
        // gain derived from inward velocity and is not a fixed full-gain call.
        if frame
            .resources
            .global_entity_type(own_type as usize)
            .ok_or(Block::Runtime("type record"))?
            .raw_header[0x86..0x88]
            != [0, 0]
        {
            return Err(Block::Runtime("solid gain cue"));
        }
        let selector = context.ground_response_selectors[usize::from(material)];
        let class = *[7, 8, 9, 10, 7, 11, 13, 12, 59, 10, 10, 10, 7]
            .get(usize::from(selector))
            .ok_or(Block::Runtime("ground response selector"))?;
        let [x, _, z] = response.position_raw;
        frame
            .world_fx
            .emit_whole_body_contact_scatter_raw(WholeBodyContactScatter {
                position_raw: [x, context.terrain.bilinear_height_raw(x, z), z],
                particle_class: class,
                scale_raw: response.particle_scale_raw,
                owner_id: id,
                owner_entity_type: own_type as u8,
                owner_state_sign: bits(entity, 0x8000_0000)? != 0,
                environment: ParticleEnvironment::Terrain(context),
                retail_tick: frame.retail_tick,
            });
    }
    entity.set_velocity_raw(response.velocity_raw);
    report.collision_damage_raw = report
        .collision_damage_raw
        .wrapping_add(response.collision_damage_raw);
    checked_tail_damage(frame, id, response.collision_damage_raw, (-2i32) as u32)
}

fn classify_and_respond_water(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
    owner: Intro2Type10TumbleOwner,
    radius: u16,
    report: &mut Intro2Type10ContactReport,
    playing: &mut Option<PlayingPlayerContact<'_>>,
    committed: &mut bool,
) -> Result<(), Intro2Type10ContactBlock> {
    use Intro2Type10ContactBlock as Block;
    let terrain = frame
        .resources
        .level_terrain()
        .ok_or(Block::Runtime("terrain"))?;
    let entity = frame
        .entities
        .entity_mut(id)
        .ok_or(Block::Runtime("water survivor"))?;
    let p = entity.position_raw();
    let ground = terrain
        .cell(
            usize::from((p[0] as u16) >> 8),
            usize::from((p[2] as u16) >> 8),
        )
        .ok_or(Block::Runtime("complete terrain grid"))?;
    if terrain.sea_level_raw() > i16::from(ground.height as i8) * 32 {
        bits(entity, 0x400000)?;
    }
    let classified = classify_whole_body_surface(WholeBodySurfaceClassificationRequest {
        state_before: entity.collision.state_flags_at_0x08,
        terrain,
        position_raw: p,
        collision_radius_raw: radius,
        retail_tick: frame.retail_tick,
        static_sea_level_raw: Some(terrain.sea_level_raw()),
        waves_enabled: frame
            .resources
            .level_desc()
            .and_then(|level| level.raw_u32(0x84))
            .ok_or(Block::Runtime("current wave policy"))?
            != 0,
    });
    *committed |= classified.state_after != entity.collision.state_flags_at_0x08;
    entity.collision.state_flags_at_0x08 = classified.state_after;
    let Some(mut contact) = classified.entry_contact else {
        return Ok(());
    };
    report.water_entry = true;
    *committed = true;
    frame.world_fx.note_water_entry();
    queue_type_cue(frame, id, 0x88)?;
    terminal_callback(
        frame,
        id,
        owner,
        Intro2Type10TumbleContact::Water,
        report,
        playing,
        committed,
    )?;
    let context = TerrainCollisionContext::from_current_level_cache(frame.resources)
        .ok_or(Block::Runtime("water response environment"))?;
    if context.water_response_selectors[usize::from(contact.material_code & 7)] == 7 {
        return Ok(());
    }
    let entity = frame
        .entities
        .entity_mut(id)
        .ok_or(Block::Runtime("water-tail survivor"))?;
    let own_type = entity.entity_type;
    // 141D0 samples again AFTER C750's radial/static mutations.
    let p = entity.position_raw();
    let ground = context
        .terrain
        .cell(
            usize::from((p[0] as u16) >> 8),
            usize::from((p[2] as u16) >> 8),
        )
        .ok_or(Block::Runtime("complete terrain grid"))?;
    contact.position_raw = p;
    contact.surface_y_raw = if frame
        .resources
        .level_desc()
        .and_then(|level| level.raw_u32(0x84))
        .ok_or(Block::Runtime("current wave policy"))?
        != 0
    {
        v2k_formats::terrain::wave_surface_raw(
            p[0],
            p[2],
            frame.retail_tick as i32,
            context.terrain.sea_level_raw(),
            i16::from(ground.height as i8) * 32,
        )
    } else {
        context.terrain.sea_level_raw()
    };
    let plan = plan_fallback_whole_body_surface_response(WholeBodySurfaceFallbackResponseRequest {
        contact,
        vertical_velocity_raw: entity.velocity_raw()[1],
        water_response_selectors: context.water_response_selectors,
    });
    match plan.response {
        None => {}
        Some(WholeBodySurfaceResponse::SurfaceBurst { response_selector }) => {
            let class = *[7, 8, 9, 10, 7, 11, 13]
                .get(usize::from(response_selector))
                .ok_or(Block::Runtime("water response selector"))?;
            frame
                .world_fx
                .emit_whole_body_contact_scatter_raw(WholeBodyContactScatter {
                    position_raw: [p[0], contact.surface_y_raw, p[2]],
                    particle_class: class,
                    scale_raw: 0x1000,
                    owner_id: id,
                    owner_entity_type: own_type as u8,
                    owner_state_sign: bits(entity, 0x8000_0000)? != 0,
                    environment: ParticleEnvironment::Terrain(context),
                    retail_tick: frame.retail_tick,
                });
        }
        Some(WholeBodySurfaceResponse::HardImpact { severe }) => {
            frame.world_fx.note_hard_entry();
            let ring = frame.entities.construct_hard_water_type60_ring(
                HardWaterType60ConstructionRequest::new(
                    [p[0], contact.surface_y_raw, p[2]],
                    if severe {
                        HardWaterType60Severity::Severe
                    } else {
                        HardWaterType60Severity::Moderate
                    },
                ),
                context.terrain,
                frame.world_fx,
            );
            let failed = if let Some(lease) = ring.primary_task_lease() {
                frame
                    .actor_tasks
                    .register_type60_exploding_ring(lease)
                    .is_err()
            } else {
                frame.world_fx.note_ring_rejection();
                false
            };
            frame.world_fx.queue_fixed_positional_sound_raw(17, p);
            let entity = frame
                .entities
                .entity_mut(id)
                .ok_or(Block::Runtime("hard-water survivor"))?;
            let mut velocity = entity.velocity_raw();
            velocity[1] = plan.vertical_velocity_after_raw;
            entity.set_velocity_raw(velocity);
            if failed {
                return Err(Block::Runtime("Type60 scheduler publication"));
            }
        }
    }
    Ok(())
}

fn apply_static_tail(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
    contact: StaticModelContact,
    report: &mut Intro2Type10ContactReport,
    committed: &mut bool,
) -> Result<(), Intro2Type10ContactBlock> {
    use Intro2Type10ContactBlock as Block;
    let entity = frame
        .entities
        .entity_mut(id)
        .ok_or(Block::Runtime("static-tail survivor"))?;
    let own_type = entity.entity_type;
    let before = entity.velocity_raw();
    let mut velocity = before;
    let mut position = entity.position_raw();
    apply_contact_response_raw(&mut position, &mut velocity, contact);
    entity.set_motion_raw(position, velocity);
    *committed = true;
    let impact = if bits(entity, 0x8000_0000)? == 0 {
        velocity_delta_impact_raw(before, velocity, entity.mass_raw)
    } else {
        0
    };
    report.collision_damage_raw = report.collision_damage_raw.wrapping_add(impact);
    if impact != 0 {
        if frame
            .resources
            .global_entity_type(own_type as usize)
            .ok_or(Block::Runtime("type record"))?
            .raw_header[0x8a..0x8c]
            != [0, 0]
        {
            return Err(Block::Runtime("static gain cue"));
        }
        if let Some(target) = resolve_current_static_damage_target(frame.resources, contact.cell)
            .map_err(Block::StaticLookup)?
        {
            let outcome = frame.static_damage.submit_hit(
                target,
                DamagePacket::collision(impact),
                &mut || frame.world_fx.next_shared_retail_random_u16(),
            );
            report.collision_static_damage = Some(outcome.clone());
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
        }
        checked_tail_damage(frame, id, impact, own_type)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
