//! Bodies of a network game ([MULTIPLAYER.md](../../../../docs/re/MULTIPLAYER.md),
//! "Bodies across machines").
//!
//! Every machine keeps every body. A body another machine owns carries
//! state bit 31: `FUN_00412DA0` runs none of its callbacks, only its sync
//! timeout and the integration of its replicated velocity, and its fields
//! come from the owner's sync records (`FUN_00417680`). Damage to it goes to
//! its owner instead (`FUN_00414E90`).

use super::*;
use crate::network::protocol::BodySync;

/// State bit 31: another machine owns the body.
pub const REMOTE_STATE_BIT: u32 = 0x8000_0000;
/// `+08` bits a sync record replaces: its low word (`FUN_00417680`).
const SYNCED_STATE_BITS: u32 = 0xFFFF;
/// `FUN_00417680`'s sync timeouts: 1.25 s for a body that was drawn
/// (`0x800`) and moving (no `0x8000000`) before the record, else 3.5 s.
pub const REMOTE_TIMEOUT_AT_REST_US: u32 = 3_500_000;
pub const REMOTE_TIMEOUT_US: u32 = 0x0013_12D0;
/// `FUN_00438080`: a player's craft travels as a Type51.
pub const PLAYER_CRAFT_TYPE: u32 = 46;
pub const REMOTE_CRAFT_TYPE: u32 = 51;

/// What a peer's sync records publish beyond the shared body fields.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NetworkBody {
    /// `+74`: time before the body lapses without a sync record.
    pub sync_timeout_us: u32,
    /// `+7C`: the owner's send count of the last record applied.
    pub resend: u32,
    /// The Sub-K model variables.
    pub model_variables: Vec<u16>,
    /// `+50`.
    pub shield: u32,
    /// `+88`.
    pub payload: u32,
    /// `+60`.
    pub owner: u32,
    /// `+A8..+AE`.
    pub words: [u16; 4],
    /// `FUN_00418F60`'s sequence word.
    pub sequence: u16,
    /// `+9C..+A0` as signed velocity words, integrated by `FUN_00412DA0`.
    pub velocity_raw: [i16; 3],
    /// `+96..+9A`, the integrated position words.
    pub position_raw: [i16; 3],
}

/// One damage the local player took in a network game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayerHitReport {
    pub lethal: bool,
    pub attacker_type: u32,
    /// A peer's slot for its damage, `None` for this machine's.
    pub attacker_slot: Option<u8>,
}

impl Entity {
    /// State bit 31.
    pub fn is_network_remote(&self) -> bool {
        self.network_body.is_some()
    }

    pub fn network_body(&self) -> Option<&NetworkBody> {
        self.network_body.as_deref()
    }
}

/// `FUN_00447280`'s network inputs: the player's weapon slots as (selector,
/// count) and its special-weapon count (controller `+0x195`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NetworkDeathDrops {
    pub slots: Vec<(u32, u32)>,
    pub special: u8,
    /// Births the constructor refused, for the host's typed stop.
    pub refused: Vec<&'static str>,
}

impl EntityManager {
    /// `FUN_00438080`'s network birth of a peer's body of `entity_type`:
    /// an allocation with the type's models and no callbacks. Returns its id.
    pub fn append_network_body(&mut self, entity_type: u32) -> Option<u32> {
        let metadata = self.type_metadata.get(entity_type as usize)?.clone();
        let id = self.allocate_live_entity_id();
        let mut entity =
            Entity::unresolved_port_entity(id, EntityKind::from_type(entity_type), entity_type);
        if entity.kind == EntityKind::Player {
            entity.kind = EntityKind::Unknown(entity_type);
        }
        entity.model_slots = metadata.model_slots.map(|slot| Some(usize::from(slot)));
        entity.model_index = Some(usize::from(metadata.model_slots[0]));
        entity.mass_raw = metadata.mass_raw;
        entity.capability_flags = metadata.capability_flags;
        entity.collision = EntityCollisionRuntimeState::from_constructor(
            Some(&metadata),
            0,
            RetailStateWord::exact(REMOTE_STATE_BIT),
        );
        entity.network_body = Some(Box::new(NetworkBody {
            sync_timeout_us: REMOTE_TIMEOUT_US,
            ..NetworkBody::default()
        }));
        self.append_live_entity(entity);
        Some(id)
    }

    /// `FUN_00417680`'s writes to a peer's body from one sync record.
    /// `parent` is the parent's local id when the record names one.
    pub fn apply_network_sync(&mut self, id: u32, sync: &BodySync, parent: Option<u32>) {
        let Some(entity) = self.entities.iter_mut().find(|entity| entity.id == id) else {
            return;
        };
        let Some(body) = entity.network_body.as_deref_mut() else {
            return;
        };
        let f = sync.flags;
        let previous_state = entity.collision.state_flags_at_0x08.known_value_bits();
        let state = (previous_state & !SYNCED_STATE_BITS) | u32::from(f) | REMOTE_STATE_BIT;
        entity.collision.state_flags_at_0x08 = RetailStateWord::exact(state);
        body.resend = sync.resend;
        if f & 0x20 != 0 {
            body.position_raw = sync.position;
            entity.position = raw_position_world(sync.position);
        }
        body.velocity_raw = if f & 0x10 != 0 { sync.velocity } else { [0; 3] };
        entity.velocity = raw_position_world(body.velocity_raw);
        let euler = f & 4 != 0;
        if f & 8 != 0 {
            entity.physical_body_basis_q31 = RetailRuntimeValue::Known(basis_from_rows(sync.basis));
        }
        if f & 0x100 != 0 {
            entity.capability_flags = sync.capability;
        }
        if sync.extras & 2 != 0 {
            body.owner = sync.owner;
        }
        if sync.extras & 1 != 0 {
            body.words = sync.words;
        }
        if sync.extras & 4 != 0 {
            body.sequence = sync.sequence;
        }
        body.shield = if f & 0x200 != 0 { sync.shield } else { 0 };
        if f & 0x400 != 0 {
            body.payload = sync.payload;
            if entity.entity_type == 61 {
                entity.power_up_payload_packed = Some(sync.payload);
            }
        }
        if f & 0x80 != 0 {
            body.model_variables = sync.model_variables.clone();
        }
        entity.attached_to = if f & 0x1000 != 0 { parent } else { None };
        body.sync_timeout_us = if previous_state & 0x0800_0000 == 0 && previous_state & 0x800 != 0 {
            REMOTE_TIMEOUT_US
        } else {
            REMOTE_TIMEOUT_AT_REST_US
        };
        if euler {
            entity.set_rotation_heading_pitch_roll_raw(sync.euler);
            entity.apply_d720_euler_body_basis();
        }
    }

    /// `FUN_00412DA0`'s branch for a peer's body: no callbacks; its sync
    /// timeout runs down and a lapsed body is staged for removal; it moves by
    /// its replicated velocity (`(elapsed >> 5) * v >> 15` a word). Returns
    /// the lapsed bodies.
    pub fn advance_network_bodies(&mut self, elapsed_us: u32) -> Vec<u32> {
        let step = elapsed_us.min(0x1E848) >> 5;
        let mut lapsed = Vec::new();
        for entity in &mut self.entities {
            let Some(body) = entity.network_body.as_deref_mut() else {
                continue;
            };
            if body.sync_timeout_us > elapsed_us {
                body.sync_timeout_us -= elapsed_us;
            } else {
                body.sync_timeout_us = 0;
                lapsed.push(entity.id);
            }
            if entity.attached_to.is_some() || body.velocity_raw == [0; 3] {
                continue;
            }
            for axis in 0..3 {
                let delta = (step as i32 * i32::from(body.velocity_raw[axis])) >> 15;
                body.position_raw[axis] = body.position_raw[axis].wrapping_add(delta as i16);
            }
            entity.position = raw_position_world(body.position_raw);
        }
        lapsed
    }

    /// Removes a peer's body (`FUN_00410B70` then the sweep): a sync timeout,
    /// a removal record, or its slot's departure.
    pub fn remove_network_body(&mut self, id: u32) -> bool {
        if !self
            .entities
            .iter()
            .any(|entity| entity.id == id && entity.network_body.is_some())
        {
            return false;
        }
        for entity in &mut self.entities {
            if entity.attached_to == Some(id) && entity.network_body.is_some() {
                entity.attached_to = None;
            }
        }
        self.remove_live_entity_ids(&[id]);
        true
    }

    /// The local bodies a machine syncs: every live allocation it owns.
    pub fn network_local_ids(&self) -> Vec<u32> {
        self.entities
            .iter()
            .filter(|entity| entity.network_body.is_none())
            .map(|entity| entity.id)
            .collect()
    }

    pub fn network_entity(&self, id: u32) -> Option<&Entity> {
        self.entities.iter().find(|entity| entity.id == id)
    }

    /// Start collecting the local player's damage reports.
    /// The player's weapon slots (selector, count) and special-weapon count
    /// for `FUN_00447280`'s drops should it die this frame.
    pub fn set_network_death_drops(&mut self, drops: NetworkDeathDrops) {
        self.network_death_drops = drops;
    }

    /// `FUN_00447280`'s network branch: each step builds a Type61 through
    /// `FUN_00438080`, a placed one around the player first
    /// (`FUN_00447750`); a repeat builds the request as it stands.
    pub(crate) fn leave_network_power_ups(
        &mut self,
        player_raw: [i16; 3],
        terrain: &TerrainGrid,
        fx: &mut WorldFx,
        retail_tick: u32,
    ) {
        use crate::network::power_ups::{drop_position, drop_steps, DropStep};
        let steps = drop_steps(
            &self.network_death_drops.slots,
            self.player_cargo_unlock_raw(),
            self.network_death_drops.special,
        );
        let sea_raw = terrain.sea_level_raw();
        let mut request = ([0i16; 3], 0u32);
        for step in steps {
            if let DropStep::Place(payload) = step {
                let position = drop_position(
                    &mut || fx.next_shared_retail_random_u16(),
                    player_raw,
                    |x, z| terrain.bilinear_height_raw(x, z),
                    sea_raw,
                );
                request = (position, payload);
            }
            if let Err(reason) =
                self.append_network_power_up(request.0, request.1, terrain, retail_tick, fx)
            {
                self.network_death_drops.refused.push(reason);
            }
        }
    }

    /// The drops the constructor refused since the last call.
    pub fn take_refused_network_drops(&mut self) -> Vec<&'static str> {
        std::mem::take(&mut self.network_death_drops.refused)
    }

    pub fn set_network_player_hits(&mut self, enabled: bool) {
        self.network_player_hits = enabled.then(Vec::new);
    }

    pub fn take_network_player_hits(&mut self) -> Vec<PlayerHitReport> {
        self.network_player_hits
            .as_mut()
            .map(std::mem::take)
            .unwrap_or_default()
    }
}

/// The forward (`+24`) and lateral (`+0C`) rows each word back to Q31,
/// normalized (`FUN_00457960`), with up as their cross product.
fn basis_from_rows(rows: [i16; 6]) -> Type9BodyBasis {
    let widen = |word: i16| -> f64 {
        let value = (i32::from(word)) << 19;
        f64::from(value)
    };
    let normalize = |v: [f64; 3]| -> [f64; 3] {
        let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        if length == 0.0 {
            return v;
        }
        v.map(|c| c / length)
    };
    let forward = normalize([widen(rows[0]), widen(rows[1]), widen(rows[2])]);
    let lateral = normalize([widen(rows[3]), widen(rows[4]), widen(rows[5])]);
    let up = [
        forward[1] * lateral[2] - forward[2] * lateral[1],
        forward[2] * lateral[0] - forward[0] * lateral[2],
        forward[0] * lateral[1] - forward[1] * lateral[0],
    ];
    let q31 = |v: [f64; 3]| v.map(|c| (c * 2_147_483_647.0).round() as i32);
    Type9BodyBasis {
        lateral: q31(lateral),
        up: q31(up),
        forward: q31(forward),
    }
}

/// The two rows a sync record carries: each Q31 word shifted right 19.
pub fn basis_rows(basis: &Type9BodyBasis) -> [i16; 6] {
    let narrow = |word: i32| (word >> 19) as i16;
    [
        narrow(basis.forward[0]),
        narrow(basis.forward[1]),
        narrow(basis.forward[2]),
        narrow(basis.lateral[0]),
        narrow(basis.lateral[1]),
        narrow(basis.lateral[2]),
    ]
}

#[cfg(test)]
#[path = "network_body_tests.rs"]
mod tests;
