//! Shared 11AD0 solid/water phases for authenticated null-hook flying styles.
//! Family adapters retain birth/task custody and provide their own 10C10 death.

use crate::{
    damage::{DamageDeliveryRecord, DamagePacket},
    entity::Entity,
    entity_collision_state::RetailRuntimeValue,
    intro2_contacts::Intro2ContactFrame,
    intro2_meteors::plan_meteor_terrain_response,
    live_actor_checked_damage::{
        apply_live_actor_checked_damage, LiveActorDamageEntry, LiveActorDamageError,
        LiveActorDamageRequest, LiveActorDeathResult,
    },
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    terrain_contact::TerrainModelContact,
    type60_exploding_ring::{
        HardWaterType60ConstructionRequest, HardWaterType60Severity, Type60ConstructionOutcome,
    },
    whole_body_surface::{
        classify_whole_body_surface, nearest_cell_material_code,
        plan_fallback_whole_body_surface_response, WholeBodySurfaceClassificationRequest,
        WholeBodySurfaceFallbackResponseRequest, WholeBodySurfaceResponse,
    },
    world_fx::{ParticleEnvironment, TerrainCollisionContext, WholeBodyContactScatter},
};
use v2k_formats::models::ModelCollisionError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlyingSurfaceContactBlock<E, D> {
    Runtime(&'static str),
    Collision(ModelCollisionError),
    SurfaceCallback(E),
    Damage(LiveActorDamageError<E, D>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlyingSurfaceContactOutcome<E, D> {
    Ineligible,
    Applied {
        solid_contact: bool,
        collision_damage_raw: i32,
        water_entry: bool,
        response: Option<WholeBodySurfaceResponse>,
        ring: Option<Type60ConstructionOutcome>,
    },
    Blocked {
        reason: FlyingSurfaceContactBlock<E, D>,
        committed_prefix: bool,
    },
}

pub(crate) fn surface_bits<E, D>(
    entity: &Entity,
    mask: u32,
) -> Result<u32, FlyingSurfaceContactBlock<E, D>> {
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(value) => Ok(value),
        RetailRuntimeValue::Unresolved => Err(FlyingSurfaceContactBlock::Runtime("contact state")),
    }
}

pub(crate) fn resolve_null_hook_flying_surface<E, D: Copy>(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
    model_id: usize,
    solid_sound: u16,
    water_sound: u16,
    committed: &mut bool,
    mut standard_death: impl FnMut(
        crate::class49_terminal::Class49TerminalFrame<'_>,
    ) -> Result<LiveActorDeathResult<D>, E>,
    mut register_death: impl FnMut(&mut SpecializedActorTaskScheduler, D),
    mut water_style_callback: impl FnMut(&mut Intro2ContactFrame<'_>, u32, &mut bool) -> Result<(), E>,
) -> Result<FlyingSurfaceContactOutcome<E, D>, FlyingSurfaceContactBlock<E, D>> {
    use FlyingSurfaceContactBlock as Block;
    let model = frame
        .resources
        .global_model(model_id)
        .ok_or(Block::Runtime("entry model"))?;
    let radius = model.collision_radius_raw;
    let entity = frame
        .entities
        .iter_all()
        .find(|e| e.id == id)
        .ok_or(Block::Runtime("allocation"))?;
    let entity_type = entity.entity_type;
    let context = TerrainCollisionContext::from_current_level_cache(frame.resources)
        .ok_or(Block::Runtime("terrain contact environment"))?;
    let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
        return Err(Block::Runtime("physical basis"));
    };
    let hit = model
        .collide_terrain_raw_oriented(
            context.terrain,
            entity.position_raw(),
            basis
                .orientation_world_from_model()
                .map(|row| row.map(f64::from)),
            &entity.presentation_anim_vars(frame.retail_tick),
        )
        .map_err(Block::Collision)?;
    let solid_contact = hit.is_some();
    let mut collision_damage_raw = 0;
    if let Some(hit) = hit {
        let original = entity.position_raw();
        let material = nearest_cell_material_code(context.terrain, original[0], original[2]);
        // This pure planner implements the same 141D0 solid mode used by the
        // meteor; it carries no meteor-specific identity or death policy.
        let response = plan_meteor_terrain_response(
            original,
            entity.velocity_raw(),
            entity.mass_raw,
            TerrainModelContact {
                normal_q12: hit.normal.map(|n| (n * 4096.0).round() as i16),
                penetration_raw: hit.penetration_raw as i32,
            },
        );
        let entity = frame
            .entities
            .entity_mut(id)
            .ok_or(Block::Runtime("solid survivor"))?;
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(0x800000, 0x800000);
        entity.set_position_raw(response.position_raw);
        *committed = true;
        if response.particle_scale_raw != 0 && solid_sound != 0 {
            return Err(Block::Runtime("native solid gain cue"));
        }
        if response.particle_scale_raw != 0 {
            let sign = surface_bits(entity, 0x8000_0000)? != 0;
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
                    owner_entity_type: entity_type as u8,
                    owner_state_sign: sign,
                    environment: ParticleEnvironment::Terrain(context),
                    retail_tick: frame.retail_tick,
                });
        }
        entity.set_velocity_raw(response.velocity_raw);
        collision_damage_raw = response.collision_damage_raw;
        if collision_damage_raw != 0 {
            let checked = apply_live_actor_checked_damage(
                frame.entities,
                frame.world_fx,
                LiveActorDamageRequest {
                    ratio_numerator: 0,
                    ratio_denominator: 0,
                    feedback: None,
                    entity_id: id,
                    delivery: DamageDeliveryRecord {
                        packet: DamagePacket::collision(collision_damage_raw),
                        source_entity_type_raw: (-2i32) as u32,
                        owner_handle: 0,
                    },
                    entry: LiveActorDamageEntry::Checked,
                },
                |manager, world_fx, _feedback| {
                    standard_death(crate::class49_terminal::Class49TerminalFrame {
                        entities: manager,
                        resources: frame.resources,
                        world_fx,
                        static_damage: frame.static_damage,
                        notifications: frame.notifications,
                        retail_tick: frame.retail_tick,
                        world: crate::class49_terminal::Class49WorldContext::Cinematic {
                            actor_tasks: frame.actor_tasks,
                            active_terminal_calls: Vec::new(),
                        },
                    })
                },
            );
            match checked {
                Ok(outcome) => {
                    if let Some(owner) = outcome.death_publication {
                        register_death(frame.actor_tasks, owner);
                    }
                }
                Err(error) => {
                    *committed |= error.committed_prefix;
                    if let Some(owner) = error.death_publication {
                        register_death(frame.actor_tasks, owner);
                    }
                    return Err(Block::Damage(error));
                }
            }
        }
    }
    // The synchronous death radial may mutate terrain; reacquire its current
    // environment before the retained-model water classifier.
    let context = TerrainCollisionContext::from_current_level_cache(frame.resources)
        .ok_or(Block::Runtime("terrain contact environment"))?;
    // 11AD0 passes its retained entry model into 129B0, even if the solid
    // callback changed actor state. XYZ and crossing bits are read anew.
    let entity = frame
        .entities
        .entity_mut(id)
        .ok_or(Block::Runtime("water survivor"))?;
    let position = entity.position_raw();
    let ground = context
        .terrain
        .cell(
            usize::from((position[0] as u16) >> 8),
            usize::from((position[2] as u16) >> 8),
        )
        .ok_or(Block::Runtime("complete terrain grid"))?;
    if context.terrain.sea_level_raw() > i16::from(ground.height as i8) * 32 {
        // A wet classification consumes the old above bit to decide whether
        // to call D860. An unresolved bit cannot silently become a missed edge.
        surface_bits(entity, 0x400000)?;
    }
    let classification = classify_whole_body_surface(WholeBodySurfaceClassificationRequest {
        state_before: entity.collision.state_flags_at_0x08,
        terrain: context.terrain,
        position_raw: position,
        collision_radius_raw: radius as u16,
        retail_tick: frame.retail_tick,
        static_sea_level_raw: Some(context.terrain.sea_level_raw()),
        waves_enabled: frame
            .resources
            .level_desc()
            .and_then(|level| level.raw_u32(0x84))
            .ok_or(Block::Runtime("current wave policy"))?
            != 0,
    });
    *committed |= classification.state_after != entity.collision.state_flags_at_0x08;
    entity.collision.state_flags_at_0x08 = classification.state_after;
    let Some(contact) = classification.entry_contact else {
        return Ok(FlyingSurfaceContactOutcome::Applied {
            solid_contact,
            collision_damage_raw,
            water_entry: false,
            response: None,
            ring: None,
        });
    };
    *committed = true;
    frame.world_fx.note_water_entry();
    // 11AD0 requests authored type+88 before D860's null style+14 hook.
    if water_sound != 0 {
        frame
            .world_fx
            .queue_fixed_positional_sound_raw(water_sound, contact.position_raw);
    }
    water_style_callback(frame, id, committed).map_err(Block::SurfaceCallback)?;
    // A solid hit can publish another style before this water callback. Both
    // D860 and141D0 reread the surviving body after the synchronous hook.
    let context = TerrainCollisionContext::from_current_level_cache(frame.resources)
        .ok_or(Block::Runtime("terrain contact environment"))?;
    let entity = frame
        .entities
        .entity_mut(id)
        .ok_or(Block::Runtime("water callback survivor"))?;
    let plan = plan_fallback_whole_body_surface_response(WholeBodySurfaceFallbackResponseRequest {
        contact,
        vertical_velocity_raw: entity.velocity_raw()[1],
        water_response_selectors: context.water_response_selectors,
    });
    let mut ring = None;
    match plan.response {
        None => {}
        Some(WholeBodySurfaceResponse::SurfaceBurst { response_selector }) => {
            let class = *[7, 8, 9, 10, 7, 11, 13]
                .get(usize::from(response_selector))
                .ok_or(Block::Runtime("water response selector"))?;
            let sign = surface_bits(entity, 0x8000_0000)? != 0;
            frame
                .world_fx
                .emit_whole_body_contact_scatter_raw(WholeBodyContactScatter {
                    position_raw: [
                        contact.position_raw[0],
                        contact.surface_y_raw,
                        contact.position_raw[2],
                    ],
                    particle_class: class,
                    scale_raw: 0x1000,
                    owner_id: id,
                    owner_entity_type: entity_type as u8,
                    owner_state_sign: sign,
                    environment: ParticleEnvironment::Terrain(context),
                    retail_tick: frame.retail_tick,
                });
        }
        Some(WholeBodySurfaceResponse::HardImpact { severe }) => {
            frame.world_fx.note_hard_entry();
            let construction = frame.entities.construct_hard_water_type60_ring(
                HardWaterType60ConstructionRequest::new(
                    [
                        contact.position_raw[0],
                        contact.surface_y_raw,
                        contact.position_raw[2],
                    ],
                    if severe {
                        HardWaterType60Severity::Severe
                    } else {
                        HardWaterType60Severity::Moderate
                    },
                ),
                context.terrain,
                frame.world_fx,
            );
            let registration_failed = if let Some(lease) = construction.primary_task_lease() {
                frame
                    .actor_tasks
                    .register_type60_exploding_ring(lease)
                    .is_err()
            } else {
                frame.world_fx.note_ring_rejection();
                false
            };
            // Constructor failure does not skip the source caller's suffix.
            frame
                .world_fx
                .queue_fixed_positional_sound_raw(17, contact.position_raw);
            let entity = frame
                .entities
                .entity_mut(id)
                .ok_or(Block::Runtime("hard-entry survivor"))?;
            let mut velocity = entity.velocity_raw();
            velocity[1] = plan.vertical_velocity_after_raw;
            entity.set_velocity_raw(velocity);
            ring = Some(construction);
            if registration_failed {
                return Err(Block::Runtime("Type60 scheduler publication"));
            }
        }
    }
    Ok(FlyingSurfaceContactOutcome::Applied {
        solid_contact,
        collision_damage_raw,
        water_entry: true,
        response: plan.response,
        ring,
    })
}
