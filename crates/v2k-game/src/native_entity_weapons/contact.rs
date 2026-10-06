//! Native post-mover Type42/59 model-terrain and whole-body water contact.
//!
//! 11AD0 retains the selected entry model across solid checked damage, then
//! reads the surviving body's pose/state again for129B0. This module accepts
//! constructor custody, uses the physical rolling matrix, and delegates the
//! synchronous410C10 owner. Static/active pair suffixes need their own current
//! opposite-body/static-cell callback context and are separate from this pass.

use super::{authenticate_weapon_metadata, grenade::*, EntityWeaponBlock, NativeEntityWeaponOwner};
use crate::{
    class49_terminal::{
        run_class49_standard_death, Class49TerminalBlock, Class49TerminalFrame, Class49WorldContext,
    },
    damage::{velocity_delta_impact_raw, DamageDeliveryRecord, DamagePacket},
    entity::{Entity, EntityManager},
    entity_collision_state::{active_model_slot_from_state_flags, RetailRuntimeValue},
    live_actor_checked_damage::{
        apply_live_actor_checked_damage, LiveActorDamageEntry, LiveActorDamageError,
        LiveActorDamageRequest, LiveActorDeathResult,
    },
    resource_cache::ResourceCache,
    type60_exploding_ring::{
        HardWaterType60ConstructionRequest, HardWaterType60Severity, Type60ConstructionOutcome,
    },
    whole_body_surface::{
        classify_whole_body_surface, nearest_cell_material_code,
        WholeBodySurfaceClassificationRequest, WholeBodySurfaceResponse,
    },
    world_fx::{ParticleEnvironment, TerrainCollisionContext, WholeBodyContactScatter, WorldFx},
};
use v2k_formats::models::ModelCollisionError;

mod pass;
pub mod static_objects;
pub use pass::{
    resolve_entity_weapon_contacts, EntityWeaponContactBlock, EntityWeaponContactError,
    EntityWeaponContactFrame, EntityWeaponContactOutcome,
};

pub(crate) fn reborrow_terminal_world<'a>(
    world: &'a mut Class49WorldContext<'_>,
) -> Class49WorldContext<'a> {
    match world {
        Class49WorldContext::Cinematic {
            actor_tasks,
            active_terminal_calls,
        } => Class49WorldContext::Cinematic {
            actor_tasks: &mut **actor_tasks,
            active_terminal_calls: active_terminal_calls.clone(),
        },
        Class49WorldContext::Playing {
            scheduler,
            player_hull,
            extra_lives,
            active_terminal_calls,
        } => Class49WorldContext::Playing {
            scheduler: &mut **scheduler,
            player_hull: &mut **player_hull,
            extra_lives: *extra_lives,
            active_terminal_calls: active_terminal_calls.clone(),
        },
    }
}

pub struct EntityWeaponSurfaceFrame<'a> {
    pub resources: &'a mut ResourceCache,
    pub world_fx: &'a mut WorldFx,
    pub retail_tick: u32,
}

/// Production dispatch borrows the current synchronous explosion context.
pub fn resolve_entity_weapon_surface_contact(
    frame: Class49TerminalFrame<'_>,
    owner: NativeEntityWeaponOwner,
) -> Result<EntityWeaponSurfaceOutcome, EntityWeaponSurfaceError<Class49TerminalBlock>> {
    let Class49TerminalFrame {
        entities,
        resources,
        world_fx,
        static_damage,
        notifications,
        retail_tick,
        mut world,
    } = frame;
    resolve_entity_weapon_surface_contact_with_terminal(
        entities,
        owner,
        EntityWeaponSurfaceFrame {
            resources,
            world_fx,
            retail_tick,
        },
        |entities, resources, world_fx, id| {
            let world = reborrow_terminal_world(&mut world);
            run_class49_standard_death(
                Class49TerminalFrame {
                    entities,
                    resources,
                    world_fx,
                    static_damage,
                    notifications,
                    retail_tick,
                    world,
                },
                id,
            )
            .map(|result| result.returned_nonzero)
        },
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntityWeaponSurfaceBlock<E> {
    Owner(EntityWeaponBlock),
    Collision(ModelCollisionError),
    Damage(LiveActorDamageError<E, ()>),
    Terminal(E),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityWeaponSurfaceError<E> {
    pub reason: EntityWeaponSurfaceBlock<E>,
    /// The caller must park this allocation instead of replaying contact.
    pub committed_prefix: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntityWeaponSurfaceOutcome {
    Ineligible,
    Applied {
        solid_contact: bool,
        collision_damage_raw: i32,
        water_entry: bool,
        response: Option<WholeBodySurfaceResponse>,
        ring: Option<Type60ConstructionOutcome>,
    },
}

/// Resolve Type42's terminal style+10 and both types' null+14 into common141D0.
/// The terminal closure runs synchronously after the lethal health write.
pub(crate) fn resolve_entity_weapon_surface_contact_with_terminal<E>(
    manager: &mut EntityManager,
    owner: NativeEntityWeaponOwner,
    mut frame: EntityWeaponSurfaceFrame<'_>,
    mut terminal: impl FnMut(
        &mut EntityManager,
        &mut ResourceCache,
        &mut WorldFx,
        u32,
    ) -> Result<bool, E>,
) -> Result<EntityWeaponSurfaceOutcome, EntityWeaponSurfaceError<E>> {
    let mut committed = false;
    run(manager, owner, &mut frame, &mut terminal, &mut committed).map_err(|reason| {
        EntityWeaponSurfaceError {
            reason,
            committed_prefix: committed,
        }
    })
}

fn state_bits<E>(entity: &Entity, mask: u32) -> Result<u32, EntityWeaponSurfaceBlock<E>> {
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(value) => Ok(value),
        RetailRuntimeValue::Unresolved => Err(EntityWeaponSurfaceBlock::Owner(
            EntityWeaponBlock::Runtime("surface contact state"),
        )),
    }
}

fn run<E>(
    manager: &mut EntityManager,
    owner: NativeEntityWeaponOwner,
    frame: &mut EntityWeaponSurfaceFrame<'_>,
    terminal: &mut impl FnMut(
        &mut EntityManager,
        &mut ResourceCache,
        &mut WorldFx,
        u32,
    ) -> Result<bool, E>,
    committed: &mut bool,
) -> Result<EntityWeaponSurfaceOutcome, EntityWeaponSurfaceBlock<E>> {
    use EntityWeaponSurfaceBlock::Owner as Owned;
    let id = owner.entity_id();
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(Owned(EntityWeaponBlock::Allocation))?;
    if !entity.active || state_bits(entity, 0x8000)? == 0 || state_bits(entity, 0x1000)? != 0 {
        return Ok(EntityWeaponSurfaceOutcome::Ineligible);
    }
    match entity.collision.subject_scan_gate_at_0x70 {
        RetailRuntimeValue::Known(0) => {}
        RetailRuntimeValue::Known(_) => return Ok(EntityWeaponSurfaceOutcome::Ineligible),
        RetailRuntimeValue::Unresolved => {
            return Err(Owned(EntityWeaponBlock::Runtime("surface subject +70")))
        }
    }
    let model_id = entity
        .model_in_slot(active_model_slot_from_state_flags(state_bits(
            entity, 0x6000,
        )?))
        .ok_or(Owned(EntityWeaponBlock::Metadata))?;
    let model = frame
        .resources
        .global_model(model_id)
        .ok_or(Owned(EntityWeaponBlock::Metadata))?;
    let retained_radius = model.collision_radius_raw;
    if retained_radius == 0
        || state_bits(entity, 0x8800_0000)? != 0
        || state_bits(entity, 0x10000)? == 0
    {
        return Ok(EntityWeaponSurfaceOutcome::Ineligible);
    }
    let entity_type = owner.kind().entity_type() as u32;
    let living_style = if entity_type == 42 {
        0x004c81a0
    } else {
        GRENADE_STYLE_ADDRESS
    };
    if entity.entity_type != entity_type || !owner.authenticates(manager) {
        return Err(Owned(EntityWeaponBlock::Allocation));
    }
    authenticate_weapon_metadata(
        owner.kind(),
        manager
            .type_runtime_metadata(entity_type)
            .ok_or(Owned(EntityWeaponBlock::Metadata))?,
    )
    .map_err(|_| Owned(EntityWeaponBlock::Metadata))?;
    if !matches!(entity.current_behavior_context, RetailRuntimeValue::Known(Some(context))
        if context.active_style().style_address() == living_style)
        || owner.task_ids.into_iter().flatten().any(|task| {
            entity
                .actor_tasks
                .wrapper_flags(task)
                .is_none_or(|flags| !flags.alive || flags.in_callback)
        })
    {
        return Err(Owned(EntityWeaponBlock::TaskChanged));
    }
    let record = frame
        .resources
        .global_entity_type(entity_type as usize)
        .ok_or(Owned(EntityWeaponBlock::Metadata))?;
    let solid_sound = u16::from_le_bytes(record.raw_header[0x86..0x88].try_into().unwrap());
    let water_sound = u16::from_le_bytes(record.raw_header[0x88..0x8a].try_into().unwrap());
    if solid_sound != 0 {
        return Err(Owned(EntityWeaponBlock::Runtime(
            "weapon solid impact cue must be null",
        )));
    }
    let context = TerrainCollisionContext::from_current_level_cache(frame.resources)
        .ok_or(Owned(EntityWeaponBlock::Terrain))?;
    let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
        return Err(Owned(EntityWeaponBlock::Runtime(
            "surface physical body matrix",
        )));
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
        .map_err(EntityWeaponSurfaceBlock::Collision)?;
    let solid_contact = hit.is_some();
    let mut collision_damage_raw = 0;
    if let Some(hit) = hit {
        if entity_type == 42 {
            // Class22 +10 runs40CEF0 before D7F0 re-resolves and invokes141D0.
            // Mark a begun terminal prefix before it can return a host stop.
            *committed = true;
            terminal(manager, frame.resources, frame.world_fx, id)
                .map_err(EntityWeaponSurfaceBlock::Terminal)?;
        }
        let context = TerrainCollisionContext::from_current_level_cache(frame.resources)
            .ok_or(Owned(EntityWeaponBlock::Terrain))?;
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == id)
            .ok_or(Owned(EntityWeaponBlock::Allocation))?;
        let original = entity.position_raw();
        let velocity_before = entity.velocity_raw();
        let material = nearest_cell_material_code(context.terrain, original[0], original[2]);
        let response = plan_grenade_terrain_contact(GrenadeTerrainContactRequest {
            position_raw: original,
            velocity_raw: velocity_before,
            contact: crate::terrain_contact::TerrainModelContact {
                normal_q12: hit.normal.map(|n| (n * 4096.0).round() as i16),
                penetration_raw: hit.penetration_raw as i32,
            },
        });
        collision_damage_raw = velocity_delta_impact_raw(
            velocity_before,
            response.velocity_after_raw,
            entity.mass_raw,
        );
        let entity = manager
            .entity_mut(id)
            .ok_or(Owned(EntityWeaponBlock::Allocation))?;
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(0x800000, 0x800000);
        entity.set_position_raw(response.position_after_raw);
        *committed = true;
        if let Some(combined) = response.motion.combined_response_raw {
            let selector = context.ground_response_selectors[usize::from(material)];
            let class = *[7, 8, 9, 10, 7, 11, 13, 12, 59, 10, 10, 10, 7]
                .get(usize::from(selector))
                .ok_or(Owned(EntityWeaponBlock::Runtime(
                    "ground response selector",
                )))?;
            let [x, _, z] = response.position_after_raw;
            frame
                .world_fx
                .emit_whole_body_contact_scatter_raw(WholeBodyContactScatter {
                    position_raw: [x, context.terrain.bilinear_height_raw(x, z), z],
                    particle_class: class,
                    scale_raw: combined.wrapping_mul(2) as u32,
                    owner_id: id,
                    owner_entity_type: entity_type as u8,
                    owner_state_sign: state_bits::<E>(entity, 0x8000_0000)? != 0,
                    environment: ParticleEnvironment::Terrain(context),
                    retail_tick: frame.retail_tick,
                });
        }
        entity.set_velocity_raw(response.velocity_after_raw);
        if collision_damage_raw != 0 {
            // The native negative environment source cannot enter selector4's
            // player-kill feedback, so no borrowed notification owner is used.
            apply_live_actor_checked_damage(
                manager,
                frame.world_fx,
                LiveActorDamageRequest {
                    entity_id: id,
                    delivery: DamageDeliveryRecord {
                        packet: DamagePacket::collision(collision_damage_raw),
                        source_entity_type_raw: (-2i32) as u32,
                        owner_handle: 0,
                    },
                    entry: LiveActorDamageEntry::Checked,
                    ratio_numerator: 0,
                    ratio_denominator: 0,
                    feedback: None,
                },
                |manager, fx, _| {
                    let returned_nonzero = terminal(manager, frame.resources, fx, id)?;
                    Ok(LiveActorDeathResult {
                        returned_nonzero,
                        publication: None::<()>,
                    })
                },
            )
            .map_err(EntityWeaponSurfaceBlock::Damage)?;
        }
    }
    // Reborrow both resources and the current survivor after synchronous death;
    // the entry model's radius remains the retained pre-callback value.
    let context = TerrainCollisionContext::from_current_level_cache(frame.resources)
        .ok_or(Owned(EntityWeaponBlock::Terrain))?;
    let entity = manager
        .entity_mut(id)
        .ok_or(Owned(EntityWeaponBlock::Allocation))?;
    let position = entity.position_raw();
    let ground = context
        .terrain
        .cell(
            usize::from((position[0] as u16) >> 8),
            usize::from((position[2] as u16) >> 8),
        )
        .ok_or(Owned(EntityWeaponBlock::Terrain))?;
    if context.terrain.sea_level_raw() > i16::from(ground.height as i8) * 32 {
        state_bits::<E>(entity, 0x400000)?;
    }
    let classification = classify_whole_body_surface(WholeBodySurfaceClassificationRequest {
        state_before: entity.collision.state_flags_at_0x08,
        terrain: context.terrain,
        position_raw: position,
        collision_radius_raw: retained_radius,
        retail_tick: frame.retail_tick,
        waves_enabled: frame
            .resources
            .level_desc()
            .and_then(|level| level.raw_u32(0x84))
            .ok_or(Owned(EntityWeaponBlock::Metadata))?
            != 0,
        static_sea_level_raw: Some(context.terrain.sea_level_raw()),
    });
    *committed |= classification.state_after != entity.collision.state_flags_at_0x08;
    entity.collision.state_flags_at_0x08 = classification.state_after;
    let Some(contact) = classification.entry_contact else {
        return Ok(EntityWeaponSurfaceOutcome::Applied {
            solid_contact,
            collision_damage_raw,
            water_entry: false,
            response: None,
            ring: None,
        });
    };
    *committed = true;
    frame.world_fx.note_water_entry();
    if water_sound != 0 {
        frame
            .world_fx
            .emit_fixed_positional_sound_raw(water_sound, position);
    }
    // Solid damage may have installed class49; its style+14 is also null.
    if !matches!(entity.current_behavior_context, RetailRuntimeValue::Known(Some(style))
        if matches!(style.active_style().style_address(), GRENADE_STYLE_ADDRESS | 0x004c81a0 | 0x004c7150 | 0x004c71e0))
    {
        return Err(Owned(EntityWeaponBlock::Runtime("water style callback")));
    }
    let plan = plan_grenade_water_contact(GrenadeWaterContactRequest {
        position_raw: position,
        surface_y_raw: contact.surface_y_raw,
        material_code: contact.material_code,
        vertical_velocity_raw: entity.velocity_raw()[1],
        water_response_selectors: context.water_response_selectors,
    });
    let mut ring = None;
    match plan.response {
        None => {}
        Some(WholeBodySurfaceResponse::SurfaceBurst { response_selector }) => {
            let class = *[7, 8, 9, 10, 7, 11, 13]
                .get(usize::from(response_selector))
                .ok_or(Owned(EntityWeaponBlock::Runtime("water response selector")))?;
            frame
                .world_fx
                .emit_whole_body_contact_scatter_raw(WholeBodyContactScatter {
                    position_raw: plan.response_origin_raw,
                    particle_class: class,
                    scale_raw: 0x1000,
                    owner_id: id,
                    owner_entity_type: entity_type as u8,
                    owner_state_sign: state_bits::<E>(entity, 0x8000_0000)? != 0,
                    environment: ParticleEnvironment::Terrain(context),
                    retail_tick: frame.retail_tick,
                });
        }
        Some(WholeBodySurfaceResponse::HardImpact { severe }) => {
            frame.world_fx.note_hard_entry();
            ring = Some(manager.construct_hard_water_type60_ring(
                HardWaterType60ConstructionRequest::new(
                    plan.response_origin_raw,
                    if severe {
                        HardWaterType60Severity::Severe
                    } else {
                        HardWaterType60Severity::Moderate
                    },
                ),
                context.terrain,
                frame.world_fx,
            ));
            frame
                .world_fx
                .emit_fixed_positional_sound_raw(17, plan.sound_origin_raw);
            let entity = manager
                .entity_mut(id)
                .ok_or(Owned(EntityWeaponBlock::Allocation))?;
            let mut velocity = entity.velocity_raw();
            velocity[1] = plan.vertical_velocity_after_raw;
            entity.set_velocity_raw(velocity);
        }
    }
    Ok(EntityWeaponSurfaceOutcome::Applied {
        solid_contact,
        collision_damage_raw,
        water_entry: true,
        response: plan.response,
        ring,
    })
}

#[cfg(test)]
mod tests;
