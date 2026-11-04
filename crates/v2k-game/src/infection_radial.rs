//! `FUN_00456710`: the Virus and Antidote Bombs' radius-500 infect or cure,
//! run by `FUN_00441850` where a bomb particle hits a static.
//!
//! `FUN_00437630` writes the terrain cells first, then `FUN_00414C50` gives
//! each eligible live body the no-packet `FUN_00411250` (infect) or
//! `FUN_00411320` (cure).

use crate::{
    entity_collision_state::RetailRuntimeValue,
    infection_evolution::InfectionCellWrite,
    shared_actor_impact::{PlayingActorImpactFrame, SharedActorImpactOutcome},
    world_fx::{ParticleEntityImpact, ParticleInfectionRadial},
};

/// `FUN_00441850` passes 500 to `FUN_00456710`.
pub const BOMB_INFECTION_RADIUS: i32 = 500;

/// Entity `+08` bits `FUN_00414C50` tests: a clean body (`0x8800`) is
/// infected, an infected one (`0xA800`) cured.
const BODY_STATE_MASK: u32 = 0xB800;
const CLEAN_BODY_STATE: u32 = 0x8800;
const INFECTED_BODY_STATE: u32 = 0xA800;

/// `FUN_00437630`'s writes in order: X offsets outer and Z offsets inner,
/// each from `-radius` to `radius` by 256. A sample point is written when its
/// horizontal offset and its height above its cell's ground (the cell's
/// signed height byte `<< 5`) both lie inside the sphere. `height_at` reads
/// the height byte of a cell (`[x, z]`); the writes change no height, so the
/// list matches retail's interleaved reads.
pub fn infection_radial_cells(
    origin_raw: [i16; 3],
    radius: i32,
    infected: bool,
    height_at: impl Fn([u8; 2]) -> i8,
) -> Vec<InfectionCellWrite> {
    let radius_squared = (radius * radius) as u32;
    let mut writes = Vec::new();
    let mut dx = -radius;
    while dx <= radius {
        let x = (origin_raw[0] as u16).wrapping_add(dx as u16);
        let mut dz = -radius;
        while dz <= radius {
            let horizontal = (dx * dx + dz * dz) as u32;
            let z = (origin_raw[2] as u16).wrapping_add(dz as u16);
            if horizontal < radius_squared {
                let ground = i16::from(height_at([(x >> 8) as u8, (z >> 8) as u8])) << 5;
                let dy = i32::from(origin_raw[1]) - i32::from(ground);
                if ((dy * dy) as u32).wrapping_add(horizontal) < radius_squared {
                    writes.push(InfectionCellWrite::at([x as i16, z as i16], infected));
                }
            }
            dz += 0x100;
        }
        dx += 0x100;
    }
    writes
}

/// Whether `FUN_00414C50` reaches a body: within the radius by the wrapped
/// word differences, and in the state the walk looks for (`state` is the
/// word masked by `0xB800`).
fn body_eligible(
    position_raw: [i16; 3],
    state: RetailRuntimeValue<u32>,
    request: ParticleInfectionRadial,
) -> Result<bool, ()> {
    let distance_squared = (0..3)
        .map(|axis| {
            let delta = i32::from(position_raw[axis].wrapping_sub(request.origin_raw[axis]));
            (delta * delta) as u32
        })
        .fold(0u32, u32::wrapping_add);
    let radius_squared = (BOMB_INFECTION_RADIUS * BOMB_INFECTION_RADIUS) as u32;
    if distance_squared >= radius_squared {
        return Ok(false);
    }
    let RetailRuntimeValue::Known(state) = state else {
        return Err(());
    };
    let wanted = if request.infected {
        CLEAN_BODY_STATE
    } else {
        INFECTED_BODY_STATE
    };
    Ok(state & BODY_STATE_MASK == wanted)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InfectionRadialBlock {
    /// A cell listener could not run; the write itself was made.
    Cell(&'static str),
    /// A body in range whose state word is unresolved.
    BodyState { id: u32, entity_type: u32 },
    /// A peer's body: its owner's machine holds it.
    NetworkBody { id: u32, entity_type: u32 },
    /// A body the port has no owner for, so its `+18`/`+1C` hook cannot run.
    UnownedBody { id: u32, entity_type: u32 },
    /// The body's owner stopped; its outcome names why.
    Owner { id: u32, outcome: String },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InfectionRadialOutcome {
    pub cells_written: usize,
    pub cells_changed: usize,
    pub bodies: Vec<u32>,
    /// The walk stopped here; the cells and bodies before it are done.
    pub blocked: Option<InfectionRadialBlock>,
}

/// Playing's `FUN_00456710`: the cells, then the live list in order, each
/// body through the owner its packet hits take.
pub fn apply_playing_infection_radial(
    frame: PlayingActorImpactFrame<'_>,
    request: ParticleInfectionRadial,
) -> InfectionRadialOutcome {
    let PlayingActorImpactFrame {
        resources,
        entities,
        world_fx,
        scheduler,
        notifications,
        static_damage,
        player_hull,
        extra_lives,
        retail_tick,
    } = frame;
    let mut outcome = InfectionRadialOutcome::default();
    // A world without terrain has no cell table: 437630 tests it first.
    let writes = resources.level_terrain().map_or_else(Vec::new, |terrain| {
        infection_radial_cells(
            request.origin_raw,
            BOMB_INFECTION_RADIUS,
            request.infected,
            |[x, z]| {
                terrain
                    .cell(usize::from(x), usize::from(z))
                    .map_or(0, |cell| cell.height as i8)
            },
        )
    });
    for write in writes {
        outcome.cells_written += 1;
        match crate::infection_listeners::write_infection(resources, world_fx, write) {
            Ok(true) => {
                outcome.cells_changed += 1;
                resources.mark_level_terrain_presentation_dirty();
            }
            Ok(false) => {}
            Err(listener) => {
                outcome.blocked = Some(InfectionRadialBlock::Cell(listener));
                return outcome;
            }
        }
    }

    let mut current = entities.retail_live_order_ids().next();
    while let Some(id) = current {
        let entity = entities.iter_all().find(|entity| entity.id == id).unwrap();
        let entity_type = entity.entity_type;
        let position_raw = entity.position_raw();
        match body_eligible(
            position_raw,
            entity.collision.state_flags_at_0x08.masked(BODY_STATE_MASK),
            request,
        ) {
            Ok(false) => {}
            Err(()) => {
                outcome.blocked = Some(InfectionRadialBlock::BodyState { id, entity_type });
                return outcome;
            }
            Ok(true) => {
                if entity.is_network_remote() {
                    outcome.blocked = Some(InfectionRadialBlock::NetworkBody { id, entity_type });
                    return outcome;
                }
                let impact = ParticleEntityImpact::no_packet_hit(
                    id,
                    request.infected,
                    position_raw,
                    entity.velocity_raw(),
                );
                let cleansing =
                    crate::cleansing_vehicle::impact::apply_cleansing_vehicle_particle_hit(
                        crate::cleansing_vehicle::impact::CleansingVehicleImpactFrame {
                            entities: &mut *entities,
                            resources: &mut *resources,
                            static_damage: &mut *static_damage,
                            notifications: &mut *notifications,
                            world_fx: &mut *world_fx,
                            scheduler: &mut *scheduler,
                            player_hull: &mut *player_hull,
                            extra_lives,
                            retail_tick,
                        },
                        impact,
                    );
                use crate::cleansing_vehicle::impact::CleansingVehicleImpactOutcome as Cleansing;
                let applied = match cleansing {
                    Cleansing::Applied(_) => Ok(()),
                    Cleansing::Blocked { .. } => Err(format!("{cleansing:?}")),
                    Cleansing::NotApplicable => player_or_actor(
                        PlayingActorImpactFrame {
                            resources: &mut *resources,
                            entities: &mut *entities,
                            world_fx: &mut *world_fx,
                            scheduler: &mut *scheduler,
                            notifications: &mut *notifications,
                            static_damage: &mut *static_damage,
                            player_hull: &mut *player_hull,
                            extra_lives,
                            retail_tick,
                        },
                        impact,
                    ),
                };
                match applied {
                    Ok(()) => outcome.bodies.push(id),
                    Err(reason) if reason.is_empty() => {
                        outcome.blocked =
                            Some(InfectionRadialBlock::UnownedBody { id, entity_type });
                        return outcome;
                    }
                    Err(reason) => {
                        outcome.blocked = Some(InfectionRadialBlock::Owner {
                            id,
                            outcome: reason,
                        });
                        return outcome;
                    }
                }
            }
        }
        // 14C50 reads the successor after the call.
        current = entities
            .retail_live_order_ids()
            .skip_while(|candidate| *candidate != id)
            .nth(1);
    }
    outcome
}

/// The player's model-switch owner, else the shared actor owners. An empty
/// error is a body without an owner.
fn player_or_actor(
    frame: PlayingActorImpactFrame<'_>,
    impact: ParticleEntityImpact,
) -> Result<(), String> {
    use crate::entity::PlayerModelSwitchHitOutcome as Player;
    let is_player = frame
        .entities
        .player()
        .is_some_and(|player| player.id == impact.target_entity_id);
    let extra_lives = match frame.extra_lives {
        RetailRuntimeValue::Known(lives) => lives,
        RetailRuntimeValue::Unresolved if is_player => return Err("extra lives unresolved".into()),
        RetailRuntimeValue::Unresolved => 0,
    };
    let player = frame.entities.apply_player_model_switch_particle_hit(
        impact,
        crate::entity::PlayerCheckedDamageFrame {
            hull: &mut *frame.player_hull,
            resources: &*frame.resources,
            world_fx: &mut *frame.world_fx,
            retail_tick: frame.retail_tick,
            notifications: &mut *frame.notifications,
            extra_lives,
        },
    );
    match player {
        Player::NotApplicable => {}
        Player::Applied { .. } | Player::NoPacket { .. } => return Ok(()),
        other => return Err(format!("{other:?}")),
    }
    match crate::shared_actor_impact::apply_playing_actor_particle_hit(frame, impact) {
        None => Err(String::new()),
        Some(outcome) if outcome.applied() => Ok(()),
        Some(
            SharedActorImpactOutcome::UnsupportedCuredTarget { .. }
            | SharedActorImpactOutcome::UnsupportedStaticRouteTarget { .. },
        ) => Err(String::new()),
        Some(outcome) => Err(format!("{outcome:?}")),
    }
}

#[cfg(test)]
mod tests;
