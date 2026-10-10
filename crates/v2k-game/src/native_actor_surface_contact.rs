//! Native Type16/17/26/58/122 `11AD0 -> D7F0/D860 -> 141D0` terrain/water contact.
//!
//! Living C8=39/439 clears10000; Class12's reverse mask2015 restores it and yields
//! effective policy28 (428 for Type122), with null solid/water style hooks.
//! Rolling Boulder Type3/27 style0 reverses 1005 into the same admission and
//! also carries null `+10`/`+14` hooks, so its contact is the bare 141D0 pair;
//! resting style1 clears the bit and never reaches this walk. The outer walk retains
//! its entry model across solid response, then classifies the current pose.
//! A late failure parks the actual scheduler owner: committed effects cannot
//! be replayed by another contact visit or overwritten by the next task tick.

use crate::{
    damage::{DamageDeliveryRecord, DamagePacket},
    entity::{Entity, EntityManager},
    entity_collision_state::{active_model_slot_from_state_flags, RetailRuntimeValue},
    intro2_common_dying::{Intro2CommonDyingBlock, Intro2CommonDyingOwner},
    intro2_contacts::Intro2ContactFrame,
    intro2_meteors::plan_meteor_terrain_response,
    intro2_radial::Intro2RadialTaskCustody,
    live_actor_checked_damage::{
        apply_live_actor_checked_damage, LiveActorDamageEntry, LiveActorDamageError,
        LiveActorDamageFeedback, LiveActorDamageRequest, LiveActorDeathResult,
    },
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

/// Shared source phases retain each allocation's constructor and death owner.
/// Values consumed by 141D0 come from its current type record/model/body:
/// Type17 has cues +86/+88=0/27 and model256 radius300; Type58 has 0/91 and
/// model273 radius250. Type122 has cues0/0, model274 radius250 and policy439.
/// All three authored masses are100; Type17/58 default policies are39.
#[derive(Debug, Clone, Copy)]
enum NativeSurfaceProfile {
    Type16,
    Type26,
    Type17,
    Type58,
    Type30,
    Type122,
    RollingBoulder(crate::rolling_boulder::RollingBoulderProfile),
}

impl NativeSurfaceProfile {
    fn for_entity_type(entity_type: u32) -> Option<Self> {
        match entity_type {
            16 => Some(Self::Type16),
            26 => Some(Self::Type26),
            17 => Some(Self::Type17),
            58 => Some(Self::Type58),
            30 => Some(Self::Type30),
            122 => Some(Self::Type122),
            3 | 27 => crate::rolling_boulder::RollingBoulderProfile::for_entity_type(entity_type)
                .map(Self::RollingBoulder),
            _ => None,
        }
    }

    fn entity_type(self) -> u32 {
        match self {
            Self::Type16 => 16,
            Self::Type26 => 26,
            Self::Type17 => 17,
            Self::Type58 => 58,
            Self::Type30 => 30,
            Self::Type122 => 122,
            Self::RollingBoulder(profile) => profile.entity_type(),
        }
    }

    fn authenticate(
        self,
        manager: &EntityManager,
        id: u32,
    ) -> Result<(), NativeActorSurfaceContactBlock> {
        use NativeActorSurfaceContactBlock as Block;
        let native_allocation = match self {
            Self::Type16 => manager
                .iter_all()
                .find(|entity| entity.id == id)
                .is_some_and(crate::intro2_type16::intro2_type16_allocation_authenticates),
            Self::Type26 => {
                crate::intro2_type26_defecate_virus::type26_manager_allocation_authenticates(
                    manager, id,
                )
            }
            Self::Type30 => crate::native_type30::manager_allocation_authenticates(manager, id),
            Self::Type122 => {
                crate::native_type122::type122_manager_allocation_authenticates(manager, id)
            }
            Self::Type17 => {
                crate::intro2_type17::type17_manager_allocation_authenticates(manager, id)
            }
            Self::Type58 => {
                crate::intro2_type58::type58_manager_allocation_authenticates(manager, id)
            }
            Self::RollingBoulder(_) => {
                crate::rolling_boulder::rolling_boulder_manager_allocation_authenticates(
                    manager, id,
                )
            }
        };
        if !native_allocation {
            return Err(Block::Runtime("native surface allocation"));
        }
        let metadata = manager
            .type_runtime_metadata(self.entity_type())
            .ok_or(Block::Runtime("native surface metadata"))?;
        let metadata_matches = match self {
            Self::Type16 => crate::intro2_type16::authenticate_metadata(metadata).is_ok(),
            Self::Type26 => {
                crate::intro2_type26_defecate_virus::authenticate_metadata(metadata).is_ok()
            }
            Self::Type30 => crate::native_type30::authenticate_metadata(metadata).is_ok(),
            Self::Type122 => crate::native_type122::authenticate_metadata(metadata).is_ok(),
            Self::Type17 => crate::intro2_type17::authenticate_metadata(metadata).is_ok(),
            Self::Type58 => crate::intro2_type58::authenticate_metadata(metadata).is_ok(),
            Self::RollingBoulder(profile) => {
                crate::rolling_boulder::authenticate_metadata(profile, metadata).is_ok()
            }
        };
        if !metadata_matches {
            return Err(Block::Runtime("native surface metadata"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeActorSurfaceContactBlock {
    Runtime(&'static str),
    Collision(ModelCollisionError),
    Damage(LiveActorDamageError<Intro2CommonDyingBlock, Intro2CommonDyingOwner>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeActorSurfaceContactOutcome {
    Ineligible,
    Applied {
        solid_contact: bool,
        collision_damage_raw: i32,
        water_entry: bool,
        response: Option<WholeBodySurfaceResponse>,
        ring: Option<Type60ConstructionOutcome>,
    },
    Blocked {
        reason: NativeActorSurfaceContactBlock,
        committed_prefix: bool,
    },
}

pub fn resolve_native_actor_surface_contact(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
) -> NativeActorSurfaceContactOutcome {
    let mut committed = false;
    match resolve(frame, id, &mut committed) {
        Ok(outcome) => outcome,
        Err(reason) => {
            if committed {
                frame
                    .actor_tasks
                    .park_native_contact_prefix(frame.entities, id);
            }
            NativeActorSurfaceContactOutcome::Blocked {
                reason,
                committed_prefix: committed,
            }
        }
    }
}

fn bits(entity: &Entity, mask: u32) -> Result<u32, NativeActorSurfaceContactBlock> {
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(value) => Ok(value),
        RetailRuntimeValue::Unresolved => {
            Err(NativeActorSurfaceContactBlock::Runtime("contact state"))
        }
    }
}

fn resolve(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
    committed: &mut bool,
) -> Result<NativeActorSurfaceContactOutcome, NativeActorSurfaceContactBlock> {
    use NativeActorSurfaceContactBlock as Block;
    let entity = frame
        .entities
        .iter_all()
        .find(|e| e.id == id)
        .ok_or(Block::Runtime("allocation"))?;
    if !entity.active {
        return Ok(NativeActorSurfaceContactOutcome::Ineligible);
    }
    let Some(profile) = NativeSurfaceProfile::for_entity_type(entity.entity_type) else {
        return Ok(NativeActorSurfaceContactOutcome::Ineligible);
    };
    // Source-ineligible actors never require callback/task custody or a basis.
    if bits(entity, 0x8000)? == 0 || bits(entity, 0x1000)? != 0 {
        return Ok(NativeActorSurfaceContactOutcome::Ineligible);
    }
    match entity.collision.subject_scan_gate_at_0x70 {
        RetailRuntimeValue::Known(0) => {}
        RetailRuntimeValue::Known(_) => return Ok(NativeActorSurfaceContactOutcome::Ineligible),
        RetailRuntimeValue::Unresolved => return Err(Block::Runtime("subject +70")),
    }
    let model_id = entity
        .model_in_slot(active_model_slot_from_state_flags(bits(entity, 0x6000)?))
        .ok_or(Block::Runtime("entry model"))?;
    let model = frame
        .resources
        .global_model(model_id)
        .ok_or(Block::Runtime("entry model"))?;
    let radius = model.collision_radius_raw;
    if radius == 0 || bits(entity, 0x8800_0000)? != 0 || bits(entity, 0x10000)? == 0 {
        return Ok(NativeActorSurfaceContactOutcome::Ineligible);
    }
    profile.authenticate(frame.entities, id)?;
    if !frame
        .actor_tasks
        .prepare_native_actor_mutation(frame.entities, id)
    {
        return Err(Block::Runtime("current completed contact owner"));
    }
    if let NativeSurfaceProfile::RollingBoulder(_) = profile {
        // Only style0 sets 10000; any other current style is not a source state.
        if crate::rolling_boulder::current_style(entity)
            != Ok(crate::rolling_boulder::RollingBoulderStyle::Rolling)
        {
            return Err(Block::Runtime("Rolling Boulder null solid/water hooks"));
        }
        crate::rolling_boulder::RollingBoulderOwner::adopt(frame.entities, id)
            .map_err(|_| Block::Runtime("Rolling Boulder current graph"))?;
    } else {
        let RetailRuntimeValue::Known(Some(style)) = entity.current_behavior_context else {
            return Err(Block::Runtime("current style"));
        };
        // No admitted living style enables this gate. Do not promote an externally
        // altered living body into a guessed terrain callback.
        if style.active_style().style_address() != 0x004c7ed0
            || entity.collision.state_flags_at_0x08.masked(0x4000)
                != RetailRuntimeValue::Known(0x4000)
        {
            return Err(Block::Runtime("Class12 null solid/water hooks"));
        }
        Intro2CommonDyingOwner::adopt(frame.entities, id)
            .map_err(|_| Block::Runtime("Class12 current graph"))?;
    }
    let record = frame
        .resources
        .global_entity_type(profile.entity_type() as usize)
        .ok_or(Block::Runtime("type record"))?;
    let solid_sound = u16::from_le_bytes([record.raw_header[0x86], record.raw_header[0x87]]);
    let water_sound = u16::from_le_bytes([record.raw_header[0x88], record.raw_header[0x89]]);
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
        if response.particle_scale_raw != 0 {
            // 141D0's optional +86 cue follows separation and precedes scatter.
            // A nonzero cue requires its inward-speed gain, not a full-gain call.
            if solid_sound != 0 {
                return Err(Block::Runtime("native solid gain cue"));
            }
            let sign = bits(entity, 0x8000_0000)? != 0;
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
                    owner_entity_type: profile.entity_type() as u8,
                    owner_state_sign: sign,
                    environment: ParticleEnvironment::Terrain(context),
                    retail_tick: frame.retail_tick,
                });
        }
        entity.set_velocity_raw(response.velocity_raw);
        collision_damage_raw = response.collision_damage_raw;
        if collision_damage_raw != 0 {
            let result = apply_live_actor_checked_damage(
                frame.entities,
                frame.world_fx,
                LiveActorDamageRequest {
                    ratio_numerator: 0,
                    ratio_denominator: 0,
                    feedback: Some(LiveActorDamageFeedback {
                        notifications: frame.notifications,
                        retail_tick: frame.retail_tick,
                    }),
                    entity_id: id,
                    delivery: DamageDeliveryRecord {
                        packet: DamagePacket::collision(collision_damage_raw),
                        source_entity_type_raw: (-2i32) as u32,
                        owner_handle: 0,
                    },
                    entry: LiveActorDamageEntry::Checked,
                },
                |manager, fx, feedback| {
                    let publication = match profile {
                        NativeSurfaceProfile::Type17 | NativeSurfaceProfile::Type122 => {
                            let feedback = feedback
                                .ok_or(Intro2CommonDyingBlock::Runtime("surface death feedback"))?;
                            crate::native_actor_capture::publish_native_captor_standard_death(
                                manager,
                                id,
                                &mut crate::intro2_type17::capture::CaptureContext {
                                    tasks: frame.actor_tasks,
                                    world_fx: fx,
                                    notifications: feedback.notifications,
                                    retail_tick: feedback.retail_tick,
                                    result_screen: crate::main_base_type9_abort::MainBaseType9ResultScreenState::NotShown,
                                    hive_dying: Default::default(),
                                },
                            )
                            .map_err(|block| Intro2CommonDyingBlock::Runtime(block.reason))?
                        }
                        NativeSurfaceProfile::Type16
                        | NativeSurfaceProfile::Type26
                        | NativeSurfaceProfile::Type58
                        | NativeSurfaceProfile::Type30 => {
                            crate::intro2_common_dying::publish_intro2_common_standard_death(
                                manager, id, fx,
                            )?
                        }
                        // Type3's class1 BAC0 and Type27's class18 split are
                        // not owned yet; hold the lethal hit at the boundary.
                        NativeSurfaceProfile::RollingBoulder(_) => {
                            return Err(Intro2CommonDyingBlock::Runtime(
                                "Rolling Boulder death program",
                            ));
                        }
                    };
                    Ok(LiveActorDeathResult {
                        returned_nonzero: publication.is_some(),
                        publication,
                    })
                },
            );
            match result {
                Ok(result) => {
                    if let Some(owner) = result.death_publication {
                        frame.actor_tasks.register_intro2_common_dying(owner);
                    }
                }
                Err(error) => {
                    if let Some(owner) = error.death_publication {
                        frame.actor_tasks.register_intro2_common_dying(owner);
                    }
                    return Err(Block::Damage(error));
                }
            }
        }
    }
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
        bits(entity, 0x400000)?;
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
        return Ok(NativeActorSurfaceContactOutcome::Applied {
            solid_contact,
            collision_damage_raw,
            water_entry: false,
            response: None,
            ring: None,
        });
    };
    *committed = true;
    frame.world_fx.note_water_entry();
    // 11AD0's type+88 cue precedes D860. Class12's style+14 is null, so
    // D860 then reaches141D0 without changing the surviving body or task.
    if water_sound != 0 {
        frame
            .world_fx
            .queue_fixed_positional_sound_raw(water_sound, position);
    }
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
            let sign = bits(entity, 0x8000_0000)? != 0;
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
                    owner_entity_type: profile.entity_type() as u8,
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
    Ok(NativeActorSurfaceContactOutcome::Applied {
        solid_contact,
        collision_damage_raw,
        water_entry: true,
        response: plan.response,
        ring,
    })
}

#[cfg(test)]
#[path = "native_actor_surface_contact/type17_tests.rs"]
mod type17_tests;

#[cfg(test)]
#[path = "native_actor_surface_contact/type58_tests.rs"]
pub(crate) mod type58_tests;
