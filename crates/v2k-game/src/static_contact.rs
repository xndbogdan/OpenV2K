//! Retail actor contact with Section-10 static terrain objects.
//!
//! This is the solid-model path used by `FUN_00411AD0`, `FUN_00427100`,
//! `FUN_00412CF0`, and `FUN_00411760`. It is intentionally separate from the
//! earlier bare-heightfield pass: terrain uses `FUN_004141D0` and can also
//! feed a hull-damage packet, while the water-plane branch has distinct
//! penetration/entry behavior.

use crate::damage::DamagePacket;
use crate::entity_collision_state::{
    EntityCollisionRuntimeState, RetailRuntimeValue, PAIR_COLLISION_ENABLED_STATE_BIT,
    PAIR_COLLISION_FIXED_STATE_BIT, PAIR_COLLISION_INELIGIBLE_STATE_BIT, REMOTE_OWNED_STATE_BIT,
};
use crate::player::{FuelPickupOutcome, PlayerCraft};
use crate::player_contact_style::{
    player_contact_haptic, player_surface_style, PlayerContactStyleBlock,
    PlayerContactStyleRequest, PlayerSurfaceStyle,
};
use crate::player_hull::{HullDamageOutcome, PlayerHull};
use crate::static_damage::{StaticCraterOutcome, StaticDamageScheduler};
use crate::static_kind_catalog::static_kind_descriptor;
use crate::terrain_contact::PlayerTerrainUnsupportedHaptic;
use v2k_formats::anim_frames::TerrainObjectTable;
use v2k_formats::models::{
    AnimVars, CollisionModelPool, ModelCollisionError, ModelCollisionHit, ModelEntry,
};
use v2k_formats::terrain::{TerrainGrid, GRID_SIZE};

const STATIC_SCAN_MARGIN_RAW: u16 = 0x0200;
const STATIC_CELL_STEP_RAW: u16 = 0x0100;
const STATIC_CELL_CENTER_LOW_RAW: u16 = 0x007f;

pub const HULL_REPAIR_STATIC_KIND_INDEX: u32 = 2;
pub const FUEL_STATIC_KIND_INDEX: u32 = 4;
pub const SHIELD_STATIC_KIND_INDEX: u32 = 7;
pub const HULL_REPAIR_AMOUNT_RAW: i32 = 20_000;
/// Case `0x37` accepts a repair only while health is strictly below 38001.
pub const HULL_REPAIR_ACCEPT_BELOW_RAW: i32 = 38_001;
pub const FUEL_PICKUP_AMOUNT_RAW: i32 = 100_000;
pub const SHIELD_PICKUP_AMOUNT_RAW: i32 = 100;
pub const SHIELD_BUFFER_MAX_RAW: i32 = 100_000;

/// One exact authored static-model contact selected by deepest penetration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticModelContact {
    pub cell: [u8; 2],
    pub attribute: u8,
    pub terrain_type: u8,
    pub model_id: u16,
    pub kind_index: u32,
    pub normal_q12: [i16; 3],
    pub penetration_raw: i32,
    pub response_raw: i16,
}

/// Result of the complete, representable mode-1 static pickup callbacks.
///
/// Kinds 2, 4, and 7 pack controller operations `0x37`, `0x33`, and `0x35`
/// into the executable's 44-byte object-kind table. The caller owns the actual
/// Section-10 mutation so rendering and collision consume one shared removal
/// event. Other mode-1/mode-2 kinds need controller inventory or entity-action
/// state which the port does not yet represent and remain ordinary contacts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaticPickupOutcome {
    None,
    /// `FUN_00427E20` runs mode-1 packed callbacks only for state zero
    /// (terrain-type model-selection bits 3-4 both clear).
    InactiveState {
        kind_index: u32,
        terrain_type: u8,
    },
    FuelAccepted {
        fuel_before_raw: i32,
        fuel_after_raw: i32,
        /// Clear only this cell's Section-10 attribute byte.
        clear_attribute_at: [u8; 2],
    },
    FuelRejectedAtCapacity {
        fuel_raw: i32,
    },
    HullRepairAccepted {
        health_before_raw: i32,
        health_after_raw: i32,
        clear_attribute_at: [u8; 2],
    },
    HullRepairRejectedAtCapacity {
        health_raw: i32,
    },
    ShieldAccepted {
        buffer_before_raw: i32,
        buffer_after_raw: i32,
        clear_attribute_at: [u8; 2],
    },
}

/// All player-visible state changes produced by one static contact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerStaticContactOutcome {
    pub contact: StaticModelContact,
    pub position_before_raw: [i16; 3],
    pub position_after_raw: [i16; 3],
    pub velocity_before_raw: [i16; 3],
    pub velocity_after_raw: [i16; 3],
    pub collision_impact_raw: i32,
    pub collision_packet: DamagePacket,
    pub collision_damage_raw: i32,
    pub hull_damage: HullDamageOutcome,
    pub pickup: StaticPickupOutcome,
    pub style_callback: PlayerStaticStyleCallbackOutcome,
}

pub struct PlayerStaticStyleCallbackFrame<'a> {
    pub request: PlayerContactStyleRequest,
    pub static_damage: &'a mut StaticDamageScheduler,
    pub world_fx: &'a mut crate::world_fx::WorldFx,
    /// Retained type-record +8A cue, requested by 12CF0 before pickup/style.
    pub contact_sound_id: RetailRuntimeValue<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerStaticDestructionOutcome {
    Started,
    Duplicate,
    BurnCell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerStaticStyleCallbackOutcome {
    Absent,
    ControllerMismatch,
    Completed {
        normal_velocity_raw: i32,
        unsupported_haptic: Option<PlayerTerrainUnsupportedHaptic>,
        kind9_destruction: Option<PlayerStaticDestructionOutcome>,
    },
}

/// A broad candidate could not be resolved faithfully. This remains distinct
/// from `Ok(None)`: treating missing resources/interpreter coverage as a miss
/// could let the player pass through an authored solid object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaticContactError {
    StyleCallback(PlayerContactStyleBlock),
    UnresolvedContactCue,
    InvalidActiveRadius {
        active_radius_raw: u16,
    },
    MissingDescriptor {
        cell: [u8; 2],
        attribute: u8,
    },
    MissingModel {
        cell: [u8; 2],
        model_id: u16,
    },
    UnsupportedKind {
        cell: [u8; 2],
        kind_index: u32,
    },
    CollisionProgram {
        cell: [u8; 2],
        model_id: u16,
        source: ModelCollisionError,
    },
}

/// Inputs to the solid Section-10 model scan in `FUN_00411AD0`.
///
/// The caller owns subject eligibility and any callback between selecting a
/// contact and applying its response. Positions use wrapping signed 8.8 words;
/// the basis maps the active model's raw-local coordinates into world axes.
pub struct StaticContactQuery<'a, P: CollisionModelPool + ?Sized> {
    pub terrain: &'a TerrainGrid,
    pub terrain_objects: &'a TerrainObjectTable,
    pub model_pool: &'a P,
    pub tick: u32,
    pub active_model: &'a ModelEntry,
    pub active_model_to_world_basis: [[f64; 3]; 3],
    pub active_anim_vars: &'a AnimVars,
    pub position_raw: [i16; 3],
}

/// Exact subject gate for the static-model branch of `FUN_00411AD0`.
///
/// Static objects do not require the separate terrain/water bit `0x10000`.
/// Unresolved surface classification and other unrelated state bits therefore
/// do not prevent this decision; unresolved inputs read by this gate do.
pub const fn static_contact_subject_eligible(
    collision: &EntityCollisionRuntimeState,
    active_model_collision_radius_raw: u16,
) -> RetailRuntimeValue<bool> {
    if active_model_collision_radius_raw == 0 {
        return RetailRuntimeValue::Known(false);
    }
    const REQUIRED_STATE_MASK: u32 = PAIR_COLLISION_ENABLED_STATE_BIT
        | PAIR_COLLISION_INELIGIBLE_STATE_BIT
        | PAIR_COLLISION_FIXED_STATE_BIT
        | REMOTE_OWNED_STATE_BIT;
    match (
        collision.state_flags_at_0x08.masked(REQUIRED_STATE_MASK),
        collision.subject_scan_gate_at_0x70,
    ) {
        (RetailRuntimeValue::Known(state), RetailRuntimeValue::Known(scan_gate)) => {
            RetailRuntimeValue::Known(
                state & PAIR_COLLISION_ENABLED_STATE_BIT != 0
                    && state
                        & (PAIR_COLLISION_INELIGIBLE_STATE_BIT
                            | PAIR_COLLISION_FIXED_STATE_BIT
                            | REMOTE_OWNED_STATE_BIT)
                        == 0
                    && scan_gate == 0,
            )
        }
        _ => RetailRuntimeValue::Unresolved,
    }
}

/// Scan and resolve at most one retail player/static-object contact.
///
/// The scan uses the active player model's collision radius plus `0x200`,
/// retaining the deepest authored contact exactly as `FUN_00411AD0` does.
/// X/Z subtraction and lattice traversal wrap as signed 8.8 words. On a kind
/// 4 contact, accepted fuel requests attribute removal, while a full-tank
/// rejection leaves the object in place. The physical response and damage
/// packet are applied in both cases.
#[allow(clippy::too_many_arguments)]
pub fn resolve_player_static_contact<P: CollisionModelPool + ?Sized>(
    terrain: &TerrainGrid,
    terrain_objects: &TerrainObjectTable,
    model_pool: &P,
    tick: u32,
    active_model: &ModelEntry,
    active_model_to_world_basis: [[f64; 3]; 3],
    active_anim_vars: &AnimVars,
    style_frame: PlayerStaticStyleCallbackFrame<'_>,
    position_raw: &mut [i16; 3],
    velocity_raw: &mut [i16; 3],
    hull: &mut PlayerHull,
    player_craft: &mut PlayerCraft,
) -> Result<Option<PlayerStaticContactOutcome>, StaticContactError> {
    let Some(contact) = scan_deepest_static_contact(StaticContactQuery {
        terrain,
        terrain_objects,
        model_pool,
        tick,
        active_model,
        active_model_to_world_basis,
        active_anim_vars,
        position_raw: *position_raw,
    })?
    else {
        return Ok(None);
    };

    let position_before_raw = *position_raw;
    let velocity_before_raw = *velocity_raw;
    let RetailRuntimeValue::Known(contact_sound_id) = style_frame.contact_sound_id else {
        return Err(StaticContactError::UnresolvedContactCue);
    };
    if contact_sound_id != 0 {
        style_frame
            .world_fx
            .queue_fixed_positional_sound_raw(contact_sound_id, position_before_raw);
    }
    // FUN_00412CF0 invokes FUN_00427E20 before the physical response and its
    // resulting damage packet. A repair or shield can therefore affect the
    // same contact which collected it.
    let pickup = apply_static_pickup(contact, hull, player_craft);
    let style_callback =
        apply_player_static_style_callback(contact, velocity_before_raw, style_frame)?;
    apply_contact_response_raw(position_raw, velocity_raw, contact);

    let collision_impact_raw = hull
        .profile()
        .impact_raw(velocity_before_raw, *velocity_raw);
    let collision_packet = DamagePacket::collision(collision_impact_raw);
    let collision_damage_raw = collision_packet.filtered_raw(Some(&hull.profile().damage));
    let hull_damage = hull.apply_damage_raw(collision_damage_raw);

    Ok(Some(PlayerStaticContactOutcome {
        contact,
        position_before_raw,
        position_after_raw: *position_raw,
        velocity_before_raw,
        velocity_after_raw: *velocity_raw,
        collision_impact_raw,
        collision_packet,
        collision_damage_raw,
        hull_damage,
        pickup,
        style_callback,
    }))
}

fn apply_player_static_style_callback(
    contact: StaticModelContact,
    velocity_raw: [i16; 3],
    frame: PlayerStaticStyleCallbackFrame<'_>,
) -> Result<PlayerStaticStyleCallbackOutcome, StaticContactError> {
    match frame.request.behavior_context {
        RetailRuntimeValue::Known(None) => return Ok(PlayerStaticStyleCallbackOutcome::Absent),
        RetailRuntimeValue::Known(Some(context))
            if player_surface_style(context) == Some(PlayerSurfaceStyle::Null) =>
        {
            return Ok(PlayerStaticStyleCallbackOutcome::Absent)
        }
        RetailRuntimeValue::Known(Some(context)) if player_surface_style(context).is_some() => {}
        _ => {
            return Err(StaticContactError::StyleCallback(
                PlayerContactStyleBlock::UnauthenticatedStyle,
            ))
        }
    }
    if frame.request.controlled_entity_handle != Some(frame.request.entity_handle) {
        return Ok(PlayerStaticStyleCallbackOutcome::ControllerMismatch);
    }
    let (normal_velocity_raw, unsupported_haptic) = player_contact_haptic(
        contact.normal_q12,
        velocity_raw,
        frame.request.haptic_scale_raw,
    );
    // 447C90 invokes 427B20 only for descriptor kind 9. It does not use the
    // dying +208 burst latch or change VY. Pickup/task work precedes this hook;
    // the shared 11760 physical response follows it.
    let kind9_destruction = if contact.kind_index == 9 {
        Some(
            match frame
                .static_damage
                .submit_crater_destruction(contact.cell, 9)
            {
                StaticCraterOutcome::Started => PlayerStaticDestructionOutcome::Started,
                StaticCraterOutcome::Duplicate => PlayerStaticDestructionOutcome::Duplicate,
                StaticCraterOutcome::BurnCell => PlayerStaticDestructionOutcome::BurnCell,
                StaticCraterOutcome::UnsupportedKind { .. } => {
                    return Err(StaticContactError::UnsupportedKind {
                        cell: contact.cell,
                        kind_index: 9,
                    })
                }
            },
        )
    } else {
        None
    };
    Ok(PlayerStaticStyleCallbackOutcome::Completed {
        normal_velocity_raw,
        unsupported_haptic,
        kind9_destruction,
    })
}

/// Select the deepest authored contact without callbacks or motion changes.
///
/// Equal penetrations retain the first tile in retail's X-then-Z traversal.
/// Resource and collision-program failures remain errors, never empty space.
pub fn scan_deepest_static_contact<P: CollisionModelPool + ?Sized>(
    query: StaticContactQuery<'_, P>,
) -> Result<Option<StaticModelContact>, StaticContactError> {
    let StaticContactQuery {
        terrain,
        terrain_objects,
        model_pool,
        tick,
        active_model,
        active_model_to_world_basis,
        active_anim_vars,
        position_raw,
    } = query;
    let active_collision_radius_raw = active_model.collision_radius_raw;
    let scan_radius_raw = active_collision_radius_raw.wrapping_add(STATIC_SCAN_MARGIN_RAW) as i16;
    if scan_radius_raw <= 0 {
        return Err(StaticContactError::InvalidActiveRadius {
            active_radius_raw: active_collision_radius_raw,
        });
    }

    // FUN_00411AD0 rounds the full diameter up to a whole-cell traversal by
    // adding 0x1FF and masking to 0xFF00. This differs subtly from iterating
    // offsets only while `offset <= radius`: it can include the cell whose
    // centre lies just beyond the positive edge.
    let span_raw = ((i32::from(scan_radius_raw) * 2 + 0x01ff) as u32 & 0xff00) as u16;
    let candidate_count = span_raw / STATIC_CELL_STEP_RAW;
    let start_x_raw = position_raw[0].wrapping_sub(scan_radius_raw);
    let start_z_raw = position_raw[2].wrapping_sub(scan_radius_raw);
    let anim_vars = static_collision_anim_vars(tick);
    let mut deepest: Option<(StaticModelContact, f64)> = None;

    // Retail visits X outside Z. Equal penetration therefore retains the
    // first encountered tile.
    for x_step in 0..candidate_count {
        let candidate_x_raw = start_x_raw.wrapping_add((x_step * STATIC_CELL_STEP_RAW) as i16);
        for z_step in 0..candidate_count {
            let candidate_z_raw = start_z_raw.wrapping_add((z_step * STATIC_CELL_STEP_RAW) as i16);
            let cell = [
                ((candidate_x_raw as u16) >> 8) as u8,
                ((candidate_z_raw as u16) >> 8) as u8,
            ];
            let terrain_cell = *terrain
                .cell(usize::from(cell[0]), usize::from(cell[1]))
                .expect("wrapped static-object cell");
            if terrain_cell.attribute == 0 {
                continue;
            }

            let Some(descriptor) = terrain_objects
                .records
                .get(usize::from(terrain_cell.attribute))
            else {
                return Err(StaticContactError::MissingDescriptor {
                    cell,
                    attribute: terrain_cell.attribute,
                });
            };
            let model_id = descriptor.model_id_for(terrain_cell.terrain_type);
            let Some(model) = model_pool.collision_model(usize::from(model_id)) else {
                return Err(StaticContactError::MissingModel { cell, model_id });
            };
            if model.collision_radius_raw == 0 {
                continue;
            }

            let center_raw = static_cell_center_raw(terrain, cell);
            let broad_radius_raw =
                u32::from(active_collision_radius_raw) + u32::from(model.collision_radius_raw);
            if !raw_spheres_overlap(position_raw, center_raw, broad_radius_raw) {
                continue;
            }

            let query_delta_raw = std::array::from_fn(|axis| {
                f64::from(position_raw[axis].wrapping_sub(center_raw[axis]))
            });
            let hit = active_model
                .collide_model_raw_oriented(
                    model,
                    query_delta_raw,
                    active_model_to_world_basis,
                    active_anim_vars,
                    &anim_vars,
                    model_pool,
                )
                .map_err(|source| StaticContactError::CollisionProgram {
                    cell,
                    model_id,
                    source,
                })?;
            let Some(hit) = hit else {
                continue;
            };
            let Some(response_raw) = static_kind_response_raw(descriptor.kind_index) else {
                return Err(StaticContactError::UnsupportedKind {
                    cell,
                    kind_index: descriptor.kind_index,
                });
            };
            let contact = contact_from_model_hit(
                cell,
                terrain_cell.attribute,
                terrain_cell.terrain_type,
                model_id,
                descriptor.kind_index,
                response_raw,
                hit,
            );
            if deepest.is_none_or(|(_, penetration)| hit.penetration_raw > penetration) {
                deepest = Some((contact, hit.penetration_raw));
            }
        }
    }

    Ok(deepest.map(|(contact, _)| contact))
}

fn contact_from_model_hit(
    cell: [u8; 2],
    attribute: u8,
    terrain_type: u8,
    model_id: u16,
    kind_index: u32,
    response_raw: i16,
    hit: ModelCollisionHit,
) -> StaticModelContact {
    let normal_q12 = hit
        .normal
        .map(|component| (component * 4096.0).round() as i16);
    StaticModelContact {
        cell,
        attribute,
        terrain_type,
        model_id,
        kind_index,
        normal_q12,
        penetration_raw: hit.penetration_raw as i32,
        response_raw,
    }
}

fn static_kind_response_raw(kind_index: u32) -> Option<i16> {
    static_kind_descriptor(kind_index).map(|descriptor| descriptor.contact_response_raw())
}

fn apply_static_pickup(
    contact: StaticModelContact,
    hull: &mut PlayerHull,
    player_craft: &mut PlayerCraft,
) -> StaticPickupOutcome {
    if !matches!(
        contact.kind_index,
        HULL_REPAIR_STATIC_KIND_INDEX | FUEL_STATIC_KIND_INDEX | SHIELD_STATIC_KIND_INDEX
    ) {
        return StaticPickupOutcome::None;
    }
    if contact.terrain_type & 0x18 != 0 {
        return StaticPickupOutcome::InactiveState {
            kind_index: contact.kind_index,
            terrain_type: contact.terrain_type,
        };
    }

    match contact.kind_index {
        HULL_REPAIR_STATIC_KIND_INDEX => {
            if hull.health_raw < HULL_REPAIR_ACCEPT_BELOW_RAW {
                let health_before_raw = hull.health_raw;
                hull.health_raw = hull
                    .health_raw
                    .wrapping_add(HULL_REPAIR_AMOUNT_RAW)
                    .min(hull.profile().max_health_raw);
                StaticPickupOutcome::HullRepairAccepted {
                    health_before_raw,
                    health_after_raw: hull.health_raw,
                    clear_attribute_at: contact.cell,
                }
            } else {
                StaticPickupOutcome::HullRepairRejectedAtCapacity {
                    health_raw: hull.health_raw,
                }
            }
        }
        FUEL_STATIC_KIND_INDEX => {
            let fuel_before_raw = player_craft.fuel_raw;
            match player_craft.collect_fuel_pickup_raw(FUEL_PICKUP_AMOUNT_RAW) {
                FuelPickupOutcome::Collected { fuel_raw, .. } => {
                    StaticPickupOutcome::FuelAccepted {
                        fuel_before_raw,
                        fuel_after_raw: fuel_raw,
                        clear_attribute_at: contact.cell,
                    }
                }
                FuelPickupOutcome::TankFull { fuel_raw } => {
                    StaticPickupOutcome::FuelRejectedAtCapacity { fuel_raw }
                }
            }
        }
        SHIELD_STATIC_KIND_INDEX => {
            let buffer_before_raw = hull.pre_health_damage_buffer_raw;
            hull.pre_health_damage_buffer_raw = hull
                .pre_health_damage_buffer_raw
                .wrapping_add(SHIELD_PICKUP_AMOUNT_RAW)
                .min(SHIELD_BUFFER_MAX_RAW);
            StaticPickupOutcome::ShieldAccepted {
                buffer_before_raw,
                buffer_after_raw: hull.pre_health_damage_buffer_raw,
                clear_attribute_at: contact.cell,
            }
        }
        _ => unreachable!("representable pickup kind was prefiltered"),
    }
}

/// Apply `FUN_00411760`'s signed-word contact response in place.
///
/// This applies only motion. The owner must dispatch the static callback first
/// and then derive and dispatch damage from the old/new velocity, as appropriate
/// for that actor. An outward or tangential negative-response contact does not
/// correct position; nonnegative responses scale velocity without separation.
pub fn apply_contact_response_raw(
    position_raw: &mut [i16; 3],
    velocity_raw: &mut [i16; 3],
    contact: StaticModelContact,
) {
    if contact.response_raw < 0 {
        let inward_raw = contact.normal_q12.iter().zip(velocity_raw.iter()).fold(
            0i32,
            |sum, (&normal, &velocity)| {
                sum.wrapping_add(i32::from(normal).wrapping_mul(i32::from(velocity)))
            },
        ) >> 12;
        if inward_raw >= 0 {
            return;
        }

        for (velocity, &normal) in velocity_raw.iter_mut().zip(&contact.normal_q12) {
            let projection_raw = (i32::from(normal).wrapping_mul(inward_raw) >> 12) as i16;
            *velocity = velocity.wrapping_sub(projection_raw);
        }

        let rebound_raw = inward_raw
            .wrapping_mul(i32::from(contact.response_raw))
            .wrapping_mul(0x10)
            >> 15;
        let rebound_raw = rebound_raw as i16;
        for (velocity, &normal) in velocity_raw.iter_mut().zip(&contact.normal_q12) {
            let axis_rebound_raw =
                (i32::from(normal).wrapping_mul(i32::from(rebound_raw)) >> 12) as i16;
            *velocity = velocity.wrapping_add(axis_rebound_raw);
        }
        for (position, &normal) in position_raw.iter_mut().zip(&contact.normal_q12) {
            let correction_raw =
                (i32::from(normal).wrapping_mul(contact.penetration_raw) >> 12) as i16;
            *position = position.wrapping_add(correction_raw);
        }
    } else {
        for velocity in velocity_raw {
            *velocity = (i32::from(*velocity)
                .wrapping_mul(i32::from(contact.response_raw))
                .wrapping_mul(0x10)
                >> 15) as i16;
        }
    }
}

fn static_collision_anim_vars(tick: u32) -> AnimVars {
    let mut vars = AnimVars::default();
    vars.dynamic[0] = (tick & 0xffff) as i32;
    vars
}

fn static_cell_center_raw(terrain: &TerrainGrid, cell: [u8; 2]) -> [i16; 3] {
    let x = usize::from(cell[0]);
    let z = usize::from(cell[1]);
    let next_x = (x + 1) % GRID_SIZE;
    let next_z = (z + 1) % GRID_SIZE;
    let height_sum_raw = [(x, z), (next_x, z), (x, next_z), (next_x, next_z)]
        .map(|(corner_x, corner_z)| {
            let height = terrain
                .cell(corner_x, corner_z)
                .expect("wrapped static-object terrain corner")
                .height as i8;
            i32::from(height) << 5
        })
        .into_iter()
        .sum::<i32>();

    [
        (((u16::from(cell[0])) << 8) | STATIC_CELL_CENTER_LOW_RAW) as i16,
        (height_sum_raw / 4) as i16,
        (((u16::from(cell[1])) << 8) | STATIC_CELL_CENTER_LOW_RAW) as i16,
    ]
}

fn raw_spheres_overlap(center_a: [i16; 3], center_b: [i16; 3], radius_raw: u32) -> bool {
    let delta = std::array::from_fn::<_, 3, _>(|axis| {
        i32::from(center_a[axis].wrapping_sub(center_b[axis]))
    });
    let radius = i64::from(radius_raw);
    if delta
        .iter()
        .any(|component| i64::from(*component).abs() > radius)
    {
        return false;
    }
    delta
        .iter()
        .map(|component| i64::from(*component) * i64::from(*component))
        .sum::<i64>()
        <= radius * radius
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity_collision_state::{RetailStateWord, SURFACE_STATE_MASK};
    use v2k_formats::anim_frames::{ModelSlotPattern, TerrainObjectDescriptor};
    use v2k_formats::models::{Billboard, ModelEntry, ModelFaceShading, ModelInstance};
    use v2k_formats::terrain::TerrainCell;

    struct TestPool(Vec<ModelEntry>);

    impl CollisionModelPool for TestPool {
        fn collision_model(&self, global_id: usize) -> Option<&ModelEntry> {
            self.0.get(global_id)
        }
    }

    fn sphere_model(radius_raw: u32) -> ModelEntry {
        let collision_program = vec![
            0x8e,
            0,
            0,
            0,
            radius_raw as u8,
            (radius_raw >> 8) as u8,
            (radius_raw >> 16) as u8,
            (radius_raw >> 24) as u8,
            0,
            0,
            0x88,
            0,
        ];
        ModelEntry {
            index: 0,
            cmd_word_count: 0,
            extra_count: (collision_program.len() / 4) as u8,
            flags: 0x40,
            slot_count: 2,
            face_val: 2,
            radius: 0,
            collision_radius_raw: radius_raw as u16,
            collision_program,
            records: vec![[0, 0, 0, 0]],
            normal_pool: Vec::new(),
            cmd_words: Vec::new(),
            has_view_commands: false,
            vertices: Vec::new(),
            vertex_type_flags: Vec::new(),
            vertex_projection: Vec::new(),
            vertex_clip: Vec::new(),
            vertex_surface_origin: Vec::new(),
            triangles: Vec::new(),
            face_vertices: Vec::new(),
            normals: Vec::new(),
            face_cull: Vec::new(),
            face_materials: Vec::new(),
            face_uvs: Vec::new(),
            face_corner_normals: Vec::new(),
            face_shading: Vec::<ModelFaceShading>::new(),
            shadow_triangles: Vec::new(),
            edges: Vec::new(),
            billboards: Vec::<Billboard>::new(),
            instances: Vec::<ModelInstance>::new(),
            name: None,
        }
    }

    fn flat_terrain() -> TerrainGrid {
        TerrainGrid {
            header: [0; 5],
            cells: vec![
                TerrainCell {
                    height: 0,
                    attribute: 0,
                    terrain_type: 0,
                };
                GRID_SIZE * GRID_SIZE
            ],
        }
    }

    fn cell_mut(terrain: &mut TerrainGrid, x: usize, z: usize) -> &mut TerrainCell {
        &mut terrain.cells[x * GRID_SIZE + z]
    }

    fn object_table(descriptors: &[(u8, u16, u32)]) -> TerrainObjectTable {
        let empty = TerrainObjectDescriptor {
            model_ids: [0; 4],
            kind_index: 0,
            pattern: ModelSlotPattern::Static,
        };
        let mut records = vec![empty; 256];
        for &(attribute, model_id, kind_index) in descriptors {
            records[usize::from(attribute)] = TerrainObjectDescriptor {
                model_ids: [model_id; 4],
                kind_index,
                pattern: ModelSlotPattern::Static,
            };
        }
        TerrainObjectTable { records }
    }

    fn contact(
        normal_q12: [i16; 3],
        penetration_raw: i32,
        response_raw: i16,
    ) -> StaticModelContact {
        StaticModelContact {
            cell: [0, 0],
            attribute: 1,
            terrain_type: 0,
            model_id: 0,
            kind_index: 0,
            normal_q12,
            penetration_raw,
            response_raw,
        }
    }

    #[test]
    fn static_subject_gate_does_not_require_terrain_water_or_surface_bits() {
        let mut collision = EntityCollisionRuntimeState::unresolved_port_entity(0);
        collision.state_flags_at_0x08 = RetailStateWord::exact(0x8000);
        collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
        assert_eq!(
            static_contact_subject_eligible(&collision, 100),
            RetailRuntimeValue::Known(true)
        );
        collision
            .state_flags_at_0x08
            .invalidate(SURFACE_STATE_MASK | 0x10000);
        assert_eq!(
            static_contact_subject_eligible(&collision, 100),
            RetailRuntimeValue::Known(true)
        );

        for state in [0, 0x9000, 0x0800_8000, 0x8000_8000] {
            collision.state_flags_at_0x08 = RetailStateWord::exact(state);
            assert_eq!(
                static_contact_subject_eligible(&collision, 100),
                RetailRuntimeValue::Known(false),
                "state {state:#010x}"
            );
        }
        collision.state_flags_at_0x08 = RetailStateWord::exact(0x8000);
        collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(1);
        assert_eq!(
            static_contact_subject_eligible(&collision, 100),
            RetailRuntimeValue::Known(false)
        );
    }

    #[test]
    fn static_subject_gate_preserves_unknown_required_state_but_zero_radius_cannot_contact() {
        let mut collision = EntityCollisionRuntimeState::unresolved_port_entity(0);
        assert_eq!(
            static_contact_subject_eligible(&collision, 0),
            RetailRuntimeValue::Known(false)
        );
        assert_eq!(
            static_contact_subject_eligible(&collision, 100),
            RetailRuntimeValue::Unresolved
        );
        collision.state_flags_at_0x08 = RetailStateWord::exact(0x8000);
        assert_eq!(
            static_contact_subject_eligible(&collision, 100),
            RetailRuntimeValue::Unresolved
        );
        collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
        collision.state_flags_at_0x08.invalidate(0x1000);
        assert_eq!(
            static_contact_subject_eligible(&collision, 100),
            RetailRuntimeValue::Unresolved
        );
    }

    #[test]
    fn static_center_averages_signed_sloping_corners_across_both_wrapped_edges() {
        let mut terrain = flat_terrain();
        for (x, z, height) in [(255, 255, -5_i8), (0, 255, -2), (255, 0, 1), (0, 0, 3)] {
            cell_mut(&mut terrain, x, z).height = height as u8;
        }
        // The four signed heights total -3: -3 * 32 / 4 = -24.
        // The static model uses their average, not a bilinear sample at 0x7f.
        assert_eq!(
            static_cell_center_raw(&terrain, [255, 255]),
            [-129, -24, -129]
        );
    }

    #[test]
    fn static_scan_ties_keep_first_tile_and_current_model_state_controls_solidity() {
        let mut terrain = flat_terrain();
        cell_mut(&mut terrain, 10, 10).attribute = 1;
        cell_mut(&mut terrain, 11, 10).attribute = 1;
        let mut objects = object_table(&[(1, 0, 0)]);
        objects.records[1].model_ids = [0, 1, 0, 1];
        let active_model = sphere_model(100);
        let pool = TestPool(vec![sphere_model(100), sphere_model(0)]);
        let vars = AnimVars::default();
        let scan = |terrain: &TerrainGrid| {
            scan_deepest_static_contact(StaticContactQuery {
                terrain,
                terrain_objects: &objects,
                model_pool: &pool,
                tick: 0,
                active_model: &active_model,
                active_model_to_world_basis: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                active_anim_vars: &vars,
                position_raw: [2815, 0, 2687],
            })
            .unwrap()
        };
        let first = scan(&terrain).expect("equidistant solid spheres");
        assert_eq!(first.cell, [10, 10]);
        assert_eq!(first.normal_q12, [4096, 0, 0]);
        assert_eq!(first.penetration_raw, 72);

        cell_mut(&mut terrain, 10, 10).terrain_type = 0x08;
        let second = scan(&terrain).expect("the second sphere remains intact");
        assert_eq!(second.cell, [11, 10]);
        assert_eq!(second.normal_q12, [-4096, 0, 0]);
        assert_eq!(second.penetration_raw, 72);
        cell_mut(&mut terrain, 11, 10).terrain_type = 0x08;
        assert_eq!(scan(&terrain), None);
    }

    #[test]
    fn negative_response_removes_inward_velocity_adds_rebound_and_corrects_position() {
        let mut position = [100, 0, 0];
        let mut velocity = [-4096, 0, 0];
        apply_contact_response_raw(
            &mut position,
            &mut velocity,
            contact([4096, 0, 0], 64, -256),
        );
        assert_eq!(velocity, [512, 0, 0]);
        assert_eq!(position, [164, 0, 0]);
    }

    #[test]
    fn negative_response_preserves_outward_and_tangential_motion_without_separation() {
        for initial_velocity in [[1, -7, 11], [0, -7, 11]] {
            let mut position = [32760, 200, -32760];
            let mut velocity = initial_velocity;
            apply_contact_response_raw(
                &mut position,
                &mut velocity,
                contact([4096, 0, 0], 64, -256),
            );
            assert_eq!(position, [32760, 200, -32760]);
            assert_eq!(velocity, initial_velocity);
        }
    }

    #[test]
    fn inward_response_wraps_position_and_preserves_tangential_velocity() {
        let mut position = [32760, 200, -32760];
        let mut velocity = [-4096, -7, 11];
        apply_contact_response_raw(
            &mut position,
            &mut velocity,
            contact([4096, 0, 0], 64, -256),
        );
        assert_eq!(position, [-32712, 200, -32760]);
        assert_eq!(velocity, [512, -7, 11]);
    }

    #[test]
    fn nonnegative_response_scales_all_axes_without_position_correction() {
        let mut position = [100, 200, 300];
        let mut velocity = [2048, -1024, 512];
        apply_contact_response_raw(
            &mut position,
            &mut velocity,
            contact([4096, 0, 0], 64, 1792),
        );
        assert_eq!(velocity, [1792, -896, 448]);
        assert_eq!(position, [100, 200, 300]);
    }

    #[test]
    fn mode_one_pickups_preserve_retail_thresholds_amounts_and_state_gate() {
        let mut hull = PlayerHull::default();
        let mut craft = PlayerCraft::default();
        craft.fuel_raw = 100_000;
        let mut fuel_contact = contact([4096, 0, 0], 1, -256);
        fuel_contact.kind_index = FUEL_STATIC_KIND_INDEX;
        fuel_contact.cell = [12, 34];
        assert_eq!(
            apply_static_pickup(fuel_contact, &mut hull, &mut craft),
            StaticPickupOutcome::FuelAccepted {
                fuel_before_raw: 100_000,
                fuel_after_raw: 200_000,
                clear_attribute_at: [12, 34],
            }
        );
        assert_eq!(craft.fuel_raw, 200_000);

        craft.fuel_raw = crate::player::FUEL_PICKUP_ACCEPT_BELOW_RAW;
        assert_eq!(
            apply_static_pickup(fuel_contact, &mut hull, &mut craft),
            StaticPickupOutcome::FuelRejectedAtCapacity {
                fuel_raw: crate::player::FUEL_PICKUP_ACCEPT_BELOW_RAW,
            }
        );

        fuel_contact.terrain_type = 0x08;
        assert_eq!(
            apply_static_pickup(fuel_contact, &mut hull, &mut craft),
            StaticPickupOutcome::InactiveState {
                kind_index: FUEL_STATIC_KIND_INDEX,
                terrain_type: 0x08,
            }
        );

        let mut repair_contact = fuel_contact;
        repair_contact.kind_index = HULL_REPAIR_STATIC_KIND_INDEX;
        repair_contact.terrain_type = 0;
        hull.health_raw = HULL_REPAIR_ACCEPT_BELOW_RAW;
        assert_eq!(
            apply_static_pickup(repair_contact, &mut hull, &mut craft),
            StaticPickupOutcome::HullRepairRejectedAtCapacity {
                health_raw: HULL_REPAIR_ACCEPT_BELOW_RAW,
            }
        );
        hull.health_raw = HULL_REPAIR_ACCEPT_BELOW_RAW - 1;
        assert_eq!(
            apply_static_pickup(repair_contact, &mut hull, &mut craft),
            StaticPickupOutcome::HullRepairAccepted {
                health_before_raw: HULL_REPAIR_ACCEPT_BELOW_RAW - 1,
                health_after_raw: 40_000,
                clear_attribute_at: [12, 34],
            }
        );

        let mut shield_contact = repair_contact;
        shield_contact.kind_index = SHIELD_STATIC_KIND_INDEX;
        hull.pre_health_damage_buffer_raw = SHIELD_BUFFER_MAX_RAW - 50;
        assert_eq!(
            apply_static_pickup(shield_contact, &mut hull, &mut craft),
            StaticPickupOutcome::ShieldAccepted {
                buffer_before_raw: SHIELD_BUFFER_MAX_RAW - 50,
                buffer_after_raw: SHIELD_BUFFER_MAX_RAW,
                clear_attribute_at: [12, 34],
            }
        );
    }

    #[test]
    fn scan_retains_deepest_contact_and_emits_fuel_removal_after_physics() {
        let mut terrain = flat_terrain();
        cell_mut(&mut terrain, 10, 10).attribute = 1;
        cell_mut(&mut terrain, 11, 10).attribute = 2;
        let objects = object_table(&[(1, 0, FUEL_STATIC_KIND_INDEX), (2, 1, 0)]);
        let active_model = sphere_model(100);
        let pool = TestPool(vec![sphere_model(100), sphere_model(100)]);
        let mut position = [2800, 0, 2687];
        let mut velocity = [-1000, 0, 0];
        let mut hull = PlayerHull::default();
        let mut craft = PlayerCraft::default();

        let outcome = resolve_player_static_contact(
            &terrain,
            &objects,
            &pool,
            0,
            &active_model,
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            &AnimVars::default(),
            PlayerStaticStyleCallbackFrame {
                request: PlayerContactStyleRequest {
                    behavior_context: RetailRuntimeValue::Known(None),
                    entity_handle: 46,
                    controlled_entity_handle: Some(46),
                    haptic_scale_raw: 15,
                },
                static_damage: &mut StaticDamageScheduler::new(),
                world_fx: &mut crate::world_fx::WorldFx::new(),
                contact_sound_id: RetailRuntimeValue::Known(0),
            },
            &mut position,
            &mut velocity,
            &mut hull,
            &mut craft,
        )
        .unwrap()
        .expect("player overlaps both static spheres");

        assert_eq!(outcome.contact.cell, [10, 10]);
        assert_eq!(outcome.contact.penetration_raw, 87);
        assert_eq!(
            outcome.pickup,
            StaticPickupOutcome::FuelAccepted {
                fuel_before_raw: 0,
                fuel_after_raw: 100_000,
                clear_attribute_at: [10, 10],
            }
        );
        assert_eq!(craft.fuel_raw, 100_000);
        assert!(outcome.velocity_after_raw[0] > 0);
        assert_eq!(
            outcome.collision_impact_raw,
            hull.profile()
                .impact_raw(outcome.velocity_before_raw, outcome.velocity_after_raw)
        );
        assert_eq!(
            outcome.collision_packet,
            DamagePacket::collision(outcome.collision_impact_raw)
        );
        assert_eq!(
            outcome.collision_damage_raw,
            outcome
                .collision_packet
                .filtered_raw(Some(&hull.profile().damage))
        );
    }

    #[test]
    fn scan_wraps_xz_across_the_signed_word_seam() {
        let mut terrain = flat_terrain();
        cell_mut(&mut terrain, 255, 0).attribute = 1;
        let objects = object_table(&[(1, 0, 0)]);
        let active_model = sphere_model(100);
        let pool = TestPool(vec![sphere_model(100)]);
        let mut position = [10, 0, 127];
        let mut velocity = [-500, 0, 0];
        let mut hull = PlayerHull::default();
        let mut craft = PlayerCraft::default();

        let outcome = resolve_player_static_contact(
            &terrain,
            &objects,
            &pool,
            0,
            &active_model,
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            &AnimVars::default(),
            PlayerStaticStyleCallbackFrame {
                request: PlayerContactStyleRequest {
                    behavior_context: RetailRuntimeValue::Known(None),
                    entity_handle: 46,
                    controlled_entity_handle: Some(46),
                    haptic_scale_raw: 15,
                },
                static_damage: &mut StaticDamageScheduler::new(),
                world_fx: &mut crate::world_fx::WorldFx::new(),
                contact_sound_id: RetailRuntimeValue::Known(0),
            },
            &mut position,
            &mut velocity,
            &mut hull,
            &mut craft,
        )
        .unwrap()
        .expect("cell 255 is adjacent to raw X zero");
        assert_eq!(outcome.contact.cell, [255, 0]);
    }

    #[test]
    fn living_and_dying_static_c90_haptic_and_kind9_program_precede_the_actual_response() {
        for class in [24, 25] {
            let mut terrain = flat_terrain();
            cell_mut(&mut terrain, 10, 10).attribute = 1;
            let objects = object_table(&[(1, 0, 9)]);
            let active = sphere_model(100);
            let pool = TestPool(vec![sphere_model(100)]);
            let mut position = [2687 + 113, 0, 2687];
            let mut velocity = [-1000, 77, 0];
            let mut hull = PlayerHull::default();
            hull.dying = class == 25;
            let mut craft = PlayerCraft::default();
            let mut scheduler = StaticDamageScheduler::new();
            let mut fx = crate::world_fx::WorldFx::new();
            let rng_before = *fx.entity_construction_state().0;
            let program = crate::entity_behavior::audited_behavior_program(class).unwrap();
            let context = crate::entity_behavior::BehaviorContextRuntime::named_audited(
                program,
                0,
                RetailRuntimeValue::Known(
                    crate::entity_behavior::BehaviorChoiceListSource::TypeDefault,
                ),
                RetailRuntimeValue::Known(None),
                RetailRuntimeValue::Known(0),
                program.initial_style,
            )
            .unwrap();
            let outcome = resolve_player_static_contact(
                &terrain,
                &objects,
                &pool,
                0,
                &active,
                [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
                &AnimVars::default(),
                PlayerStaticStyleCallbackFrame {
                    request: PlayerContactStyleRequest {
                        behavior_context: RetailRuntimeValue::Known(Some(context)),
                        entity_handle: 46,
                        controlled_entity_handle: Some(46),
                        haptic_scale_raw: 15,
                    },
                    static_damage: &mut scheduler,
                    world_fx: &mut fx,
                    contact_sound_id: RetailRuntimeValue::Known(0),
                },
                &mut position,
                &mut velocity,
                &mut hull,
                &mut craft,
            )
            .unwrap()
            .unwrap();
            let PlayerStaticStyleCallbackOutcome::Completed {
                normal_velocity_raw,
                unsupported_haptic,
                kind9_destruction,
            } = outcome.style_callback
            else {
                panic!("authenticated C90")
            };
            assert_eq!(normal_velocity_raw, -1000);
            assert_eq!(unsupported_haptic.unwrap().strength_raw, 255);
            assert_eq!(
                kind9_destruction,
                Some(PlayerStaticDestructionOutcome::Started)
            );
            assert_eq!(scheduler.active_program_count(), 1);
            assert_eq!(
                velocity[1], 77,
                "static +1C does not perform AF0's VY=1000 write"
            );
            assert!(velocity[0] > 0, "11760 follows C90");
            assert_eq!(
                *fx.entity_construction_state().0,
                rng_before,
                "C90 has no 475F0/reseed branch"
            );
            assert_eq!(fx.particle_count(), 0);
            let mut p = [2687 + 113, 0, 2687];
            let mut v = [-512, 0, 0];
            let second = resolve_player_static_contact(
                &terrain,
                &objects,
                &pool,
                0,
                &active,
                [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
                &AnimVars::default(),
                PlayerStaticStyleCallbackFrame {
                    request: PlayerContactStyleRequest {
                        behavior_context: RetailRuntimeValue::Known(Some(context)),
                        entity_handle: 46,
                        controlled_entity_handle: Some(46),
                        haptic_scale_raw: 15,
                    },
                    static_damage: &mut scheduler,
                    world_fx: &mut fx,
                    contact_sound_id: RetailRuntimeValue::Known(0),
                },
                &mut p,
                &mut v,
                &mut hull,
                &mut craft,
            )
            .unwrap()
            .unwrap();
            assert!(matches!(
                second.style_callback,
                PlayerStaticStyleCallbackOutcome::Completed {
                    unsupported_haptic: None,
                    kind9_destruction: Some(PlayerStaticDestructionOutcome::Duplicate),
                    ..
                }
            ));
        }
    }
}
