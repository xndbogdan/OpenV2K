//! Bodies of a network game ([MULTIPLAYER.md](../../../../docs/re/MULTIPLAYER.md),
//! "Bodies between machines").
//!
//! Every machine keeps every body. A body another machine owns carries
//! state bit 31: `FUN_00412DA0` runs none of its callbacks, only its sync
//! timeout and the integration of its replicated velocity, and its fields
//! come from the owner's sync records (`FUN_00417680`). Damage to it goes to
//! its owner instead (`FUN_00414E90`). Its birth is the full constructor, so
//! a hit on it runs the copy's own callbacks.

use super::*;
use crate::intro2_flyers_live::{
    authenticate_native_flyer_body, authenticate_native_flyer_metadata, publish_native_flyer,
    NativeFlyerConstructionRequest, TYPE51_ENTITY_TYPE,
};
use crate::network::protocol::BodySync;

/// What a peer's birth reads of this machine's world: `FUN_00438080`'s
/// constructor places the body on this terrain and draws the shared words.
pub struct PeerBirthWorld<'a> {
    pub terrain: &'a TerrainGrid,
    pub terrain_objects: Option<&'a TerrainObjectTable>,
    /// The models' Section-8 extents, for `FUN_0040D4A0`'s placement.
    pub model_extent_raw: Option<&'a dyn Fn(usize) -> Option<u16>>,
    pub world_fx: &'a mut WorldFx,
    pub retail_tick: u32,
}

/// The body a peer's first record built.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PeerBirth {
    /// `FUN_00438080`'s whole constructor ran: components, the behavior
    /// chosen at birth and its shared draws.
    Native(u32),
    /// The port does not build this type's copy: its models and no
    /// components, a typed stop.
    Hollow { id: u32, reason: String },
}

impl PeerBirth {
    pub fn id(&self) -> u32 {
        match *self {
            Self::Native(id) | Self::Hollow { id, .. } => id,
        }
    }
}

/// The first authored-record index a peer's copy carries: the port's
/// bookkeeping only (retail has no such index; no level has this many
/// records). Copies count up from it in birth order, which is their list
/// order, as an authored world's records are.
pub const PEER_COPY_RECORD_INDEX: usize = usize::MAX / 2;

/// `FUN_00418A90` without a template: the zeroed Sub-M's 0x58 bytes with
/// `+50` (the primary `FUN_0044C830` voice) set to 0x1F.
const NULL_SUB_M_TEMPLATE: [u8; 0x58] = {
    let mut template = [0; 0x58];
    template[0x50] = 0x1F;
    template
};

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

/// What `FUN_00417680`'s relation tail asks of the network layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetworkRelationRepair {
    /// Kind 15 to the owner: this machine's parent does not list the body.
    /// The parent's raw X and Z.
    Dropped { x_raw: i16, z_raw: i16 },
    /// This machine's parent's Sub-J is unresolved.
    Unknown,
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
    /// A jolt's velocity on a peer's copy: the words `FUN_00412DA0`
    /// integrates are the body's own.
    pub(crate) fn set_peer_velocity_raw(&mut self, velocity_raw: [i16; 3]) {
        self.set_velocity_raw(velocity_raw);
        if let Some(body) = self.network_body.as_deref_mut() {
            body.velocity_raw = velocity_raw;
        }
    }

    /// `FUN_0040A950`'s model word `selector` (1-based) of a peer's copy,
    /// zeroed: the words its owner's records carry.
    pub(crate) fn zero_peer_model_word(&mut self, selector: usize) {
        if let Some(word) = self
            .network_body
            .as_deref_mut()
            .and_then(|body| body.model_variables.get_mut(selector - 1))
        {
            *word = 0;
        }
    }

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
    /// `FUN_00417680`'s birth of a peer's body: `FUN_00438080` with a zeroed
    /// record that carries the handle, the type and the record's position
    /// (`None` when the record has none, whose words retail reads from
    /// stale stack). Types whose copy the port builds natively go through
    /// their constructor; any other is a hollow body.
    pub fn append_network_body(
        &mut self,
        entity_type: u32,
        position_raw: Option<[i16; 3]>,
        world: Option<PeerBirthWorld<'_>>,
    ) -> Option<PeerBirth> {
        let reason = match (entity_type, position_raw, world) {
            (_, None, _) => "a record without its position".to_string(),
            (_, _, None) => "no terrain".to_string(),
            (TYPE51_ENTITY_TYPE, Some(position), Some(world)) => {
                match self.append_native_peer_craft(position, world) {
                    Ok(id) => return Some(PeerBirth::Native(id)),
                    Err(reason) => reason.to_string(),
                }
            }
            (_, Some(position), Some(world)) if native_authored_publication_type(entity_type) => {
                match self.append_native_peer_copy(entity_type, position, world) {
                    Ok(id) => return Some(PeerBirth::Native(id)),
                    Err(reason) => format!("its constructor stopped: {reason}"),
                }
            }
            _ => "its constructor is not ported".to_string(),
        };
        let id = self.append_hollow_network_body(entity_type)?;
        if let Some(position) = position_raw {
            let entity = self.entity_mut(id)?;
            entity.set_position_raw(position);
            entity.network_body.as_mut()?.position_raw = position;
        }
        Some(PeerBirth::Hollow { id, reason })
    }

    /// A peer's craft (type 51) through the class-7 flyer constructor, as
    /// `FUN_00438080` builds it: `FUN_00443110`'s controller first, whose
    /// zeroed profile writes nothing to a body not yet built; then
    /// `FUN_004104B0` with the handle of another slot (state bit 31), Sub-G's
    /// word (`FUN_0041B8C0`), Sub-D's allocation, the class selector and the
    /// Sub-G words of B6C0's two task constructors. Its owner is never
    /// scheduled: `FUN_00412DA0` runs none of a peer's callbacks.
    fn append_native_peer_craft(
        &mut self,
        position_raw: [i16; 3],
        world: PeerBirthWorld<'_>,
    ) -> Result<u32, &'static str> {
        let metadata = self
            .type_metadata
            .get(TYPE51_ENTITY_TYPE as usize)
            .ok_or("no Type51 record")?
            .clone();
        authenticate_native_flyer_metadata(TYPE51_ENTITY_TYPE, &metadata)
            .map_err(|_| "an unexpected Type51 record")?;
        if !matches!(
            self.next_common_body_ordinal(),
            RetailRuntimeValue::Known(_)
        ) {
            return Err("the body stamp lineage");
        }
        let terrain = world.terrain;
        if terrain.cells.len() != v2k_formats::terrain::GRID_SIZE * v2k_formats::terrain::GRID_SIZE
        {
            return Err("an incomplete terrain");
        }
        let surface = crate::entity_initializer::constructor_surface_bits_at_tick(
            position_raw,
            terrain,
            world.retail_tick,
            self.common_environment_physics().waves_enabled,
        )
        .ok_or("the constructor's surface")?;
        let id = self.next_entity_id;
        if id == 0 || self.entities.iter().any(|entity| entity.id == id) {
            return Err("the allocation handle");
        }
        let mut entity = native_instance_body(
            id,
            RetailRuntimeValue::Unresolved,
            &metadata,
            terrain,
            NativeInstanceBodyRequest {
                entity_type: TYPE51_ENTITY_TYPE,
                position_raw,
                objective: false,
            },
        );
        entity.collision.state_flags_at_0x08.overwrite(
            SURFACE_STATE_MASK | REMOTE_STATE_BIT,
            surface | REMOTE_STATE_BIT,
        );
        let radius = match entity.collision.active_model_slot() {
            RetailRuntimeValue::Known(slot) => entity
                .model_in_slot(slot)
                .and_then(|model| world.model_extent_raw.and_then(|lookup| lookup(model)))
                .map(|radius| radius as i16),
            RetailRuntimeValue::Unresolved => None,
        };
        let RetailRuntimeValue::Known(position) =
            crate::entity_initializer::constructor_position_raw(
                &metadata,
                position_raw,
                Some(terrain),
                radius,
            )
        else {
            return Err("FUN_0040D4A0's placement");
        };
        entity.set_position_raw(position);
        let allocation = observe_main_base_abort_actor(&entity, self.allocation_generation).lease;
        authenticate_native_flyer_body(&entity, allocation, &metadata, None)
            .map_err(|_| "an unexpected Type51 body")?;
        entity.construction_stamp_at_0xb4 = self.begin_common_body_attempt();
        publish_native_flyer(
            NativeFlyerConstructionRequest {
                entity: &mut entity,
                allocation,
                metadata: &metadata,
                authored: None,
            },
            world.world_fx,
        )
        .map_err(|_| "the class-7 publication")?;
        // FUN_004104B0's +44: type 51 has no damage modifier.
        entity.collision.pair_callbacks.damage_modifier_address = RetailRuntimeValue::Known(None);
        entity.network_body = Some(Box::new(NetworkBody {
            sync_timeout_us: REMOTE_TIMEOUT_US,
            position_raw: entity.position_raw(),
            ..NetworkBody::default()
        }));
        assert_eq!(self.allocate_live_entity_id(), id);
        self.append_live_entity(entity);
        Ok(id)
    }

    /// `FUN_00438080` with the zeroed record `FUN_00417680` builds: the
    /// handle, the type and the position, every other word zero. It is the
    /// constructor `FUN_0042E440` runs for an authored record (the same
    /// `FUN_0040D720`, components and type initializer, drawing the same
    /// shared words), so the port's authored path builds it from an empty
    /// record. `FUN_004104B0` sets state bit 31 for the handle's slot, and
    /// the copy is never scheduled.
    fn append_native_peer_copy(
        &mut self,
        entity_type: u32,
        position_raw: [i16; 3],
        world: PeerBirthWorld<'_>,
    ) -> Result<u32, String> {
        if !matches!(
            self.construction_provenance,
            LevelEntityConstructionProvenance::NativeOrdinary { .. }
        ) {
            return Err("a world built without its native constructors".into());
        }
        let [x, y, z] = position_raw.map(i16::to_le_bytes);
        // FUN_004381F0 hands FUN_00418A90 the record's template pointer
        // (+44), null here: the Sub-M FUN_00418A60 zeroed keeps its words
        // but for +50, which becomes 0x1F. That is the template below.
        let has_sub_m = matches!(
            self.type_metadata
                .get(entity_type as usize)
                .map(|metadata| metadata.status_component_descriptor),
            Some(RetailRuntimeValue::Known(Some(_)))
        );
        let record = v2k_formats::levels::EntitySpawn {
            index: self
                .entities
                .iter()
                .filter_map(|entity| entity.authored_spawn_index)
                .filter(|&index| index >= PEER_COPY_RECORD_INDEX)
                .max()
                .map_or(PEER_COPY_RECORD_INDEX, |index| index + 1),
            entity_type,
            pos_data_1: [x[0], x[1], y[0], y[1]],
            pos_data_2: [z[0], z[1], 0, 0],
            param: 0,
            rotation: [0; 3],
            extra: [0; 40],
            initial_damage_buffer_raw: 0,
            model_overrides: [0; 4],
            has_animation: false,
            anim_frames: 0,
            animation: None,
            has_config: false,
            config: has_sub_m.then_some(NULL_SUB_M_TEMPLATE),
        };
        let id = self.next_entity_id;
        let type_metadata = self.type_metadata.clone();
        let construction = LevelSpawnConstruction {
            type_metadata: &type_metadata,
            resources: EntityConstructionResources {
                terrain: Some(world.terrain),
                terrain_objects: world.terrain_objects,
                model_extent_raw: world.model_extent_raw,
            },
            construction_provenance: self.construction_provenance,
            retail_first_world: false,
            waves_enabled: self.common_environment_physics().waves_enabled,
            restored_world_flags: 0,
        };
        self.construct_level_spawn(
            &record,
            construction,
            &mut Some((world.retail_tick, world.world_fx)),
            &mut None,
        )
        .map_err(|error| match error {
            FreshNewGameEntityConstructionError::NativeActorConstruction { reason, .. } => reason,
            other => format!("{other:?}"),
        })?;
        let entity = self
            .entities
            .iter_mut()
            .find(|entity| entity.id == id)
            .ok_or("no body published")?;
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(REMOTE_STATE_BIT, REMOTE_STATE_BIT);
        entity.network_body = Some(Box::new(NetworkBody {
            sync_timeout_us: REMOTE_TIMEOUT_US,
            position_raw: entity.position_raw(),
            ..NetworkBody::default()
        }));
        Ok(id)
    }

    /// A body with the type's models and no components or callbacks.
    fn append_hollow_network_body(&mut self, entity_type: u32) -> Option<u32> {
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
        // A peer's body arrives with FUN_00418A90's null template: the zeroed
        // Sub-M keeps its words but for +50 (the 0x1F voice). Retail's full
        // constructor therefore always resolves the component's presence, so
        // a hollow copy carries the same zeroed state instead of an
        // unresolved one: searches (Job Nearby) and remote-owner damage
        // (kind 9) skip it the way they skip a zeroed live Sub-M, rather
        // than stopping the whole evaluation on unknown words.
        entity.base_factory_runtime = match &metadata.status_component_descriptor {
            RetailRuntimeValue::Known(Some(descriptor)) => RetailRuntimeValue::Known(Some(
                super::BaseFactoryRuntimeState::from_behavior_instance_template(
                    *descriptor,
                    None,
                    None,
                ),
            )),
            RetailRuntimeValue::Known(None) => RetailRuntimeValue::Known(None),
            RetailRuntimeValue::Unresolved => RetailRuntimeValue::Unresolved,
        };
        // FUN_004104B0's instance damage modifier `+44` by type.
        entity.collision.pair_callbacks.damage_modifier_address =
            RetailRuntimeValue::Known(match entity_type {
                0x2E => Some(0x0044_84A0),
                0x49 | 0x6D => Some(0x0041_6820),
                0x6E => Some(0x0042_1910),
                _ => None,
            });
        entity.network_body = Some(Box::new(NetworkBody {
            sync_timeout_us: REMOTE_TIMEOUT_US,
            ..NetworkBody::default()
        }));
        self.append_live_entity(entity);
        Some(id)
    }

    /// Whether a body this machine owns lists `child` in its Sub-J
    /// (`FUN_00418A00`, false without a Sub-J); `None` for a peer's or an
    /// unknown body.
    fn local_sub_j_holds(&self, parent: u32, child: u32) -> Option<RetailRuntimeValue<bool>> {
        let parent = self
            .entities
            .iter()
            .find(|entity| entity.id == parent && entity.network_body.is_none())?;
        Some(match &parent.sub_j_attachment_runtime {
            RetailRuntimeValue::Known(Some(runtime)) => {
                RetailRuntimeValue::Known(runtime.contains(child))
            }
            RetailRuntimeValue::Known(None) => RetailRuntimeValue::Known(false),
            RetailRuntimeValue::Unresolved => RetailRuntimeValue::Unresolved,
        })
    }

    /// `FUN_00417680`'s writes to a peer's body from one sync record.
    /// `parent` is the parent's local id when the record names one. Returns
    /// what its relation tail asks of the network layer.
    pub fn apply_network_sync(
        &mut self,
        id: u32,
        sync: &BodySync,
        parent: Option<u32>,
    ) -> Option<NetworkRelationRepair> {
        let attached = sync.flags & 0x1000 != 0;
        let old_parent = self
            .entities
            .iter()
            .find(|entity| entity.id == id)?
            .attached_to;
        // The tail reads the relation the body had before this record.
        let old_holds = (!attached)
            .then(|| old_parent.and_then(|old| self.local_sub_j_holds(old, id)))
            .flatten();
        let parent_state = parent.filter(|_| attached).and_then(|parent_id| {
            let parent = self.entities.iter().find(|entity| entity.id == parent_id)?;
            Some((
                parent.position_raw(),
                parent.velocity_raw(),
                self.local_sub_j_holds(parent_id, id),
            ))
        });
        let entity = self.entities.iter_mut().find(|entity| entity.id == id)?;
        let body = entity.network_body.as_deref_mut()?;
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
        let mut repair = None;
        if attached {
            entity.attached_to = parent;
            if let Some((position, velocity, holds)) = parent_state {
                // Without its own fields the body takes the parent's.
                if f & 0x20 == 0 {
                    body.position_raw = position;
                    entity.position = raw_position_world(position);
                }
                if f & 0x10 == 0 {
                    body.velocity_raw = velocity;
                    entity.velocity = raw_position_world(velocity);
                }
                // This machine's parent does not hold it: kind 15 tells the
                // owner to drop it at the parent's X and Z.
                repair = match holds {
                    Some(RetailRuntimeValue::Known(false)) => {
                        Some(NetworkRelationRepair::Dropped {
                            x_raw: position[0],
                            z_raw: position[2],
                        })
                    }
                    Some(RetailRuntimeValue::Unresolved) => Some(NetworkRelationRepair::Unknown),
                    _ => None,
                };
            }
        } else {
            // Released by its owner while this machine's body still lists it:
            // it is no longer drawn here.
            match old_holds {
                Some(RetailRuntimeValue::Known(true)) => {
                    entity.collision.state_flags_at_0x08.overwrite(0x800, 0)
                }
                Some(RetailRuntimeValue::Unresolved) => {
                    repair = Some(NetworkRelationRepair::Unknown)
                }
                _ => {}
            }
            entity.attached_to = None;
        }
        body.sync_timeout_us = if previous_state & 0x0800_0000 == 0 && previous_state & 0x800 != 0 {
            REMOTE_TIMEOUT_US
        } else {
            REMOTE_TIMEOUT_AT_REST_US
        };
        if euler {
            entity.set_rotation_heading_pitch_roll_raw(sync.euler);
            entity.apply_d720_euler_body_basis();
        }
        repair
    }

    /// `FUN_00418640`'s network branch over the player's Sub-J, for the rows
    /// that name peers' bodies: a body its owner has not attached (no
    /// relation) is asked for with kind 13, and loses its drawn bit `0x800`
    /// when its slot's first byte is zero; a body attached elsewhere, gone,
    /// dying or with a zero state word loses its row, the rows after it
    /// taking its slot. Returns each kind 13's body and flags (1 for the
    /// Sub-J's `+0C`, 2 for a slot drawn in place). Rows of this machine's
    /// bodies are left as they are.
    pub fn network_player_cargo_rows(&mut self) -> Vec<(u32, u32)> {
        let Some(player_id) = self.player_id else {
            return Vec::new();
        };
        let Some(player_type) = self.player().map(|player| player.entity_type) else {
            return Vec::new();
        };
        let Some(runtime) = self.player_sub_j_runtime() else {
            return Vec::new();
        };
        let rows = runtime.ordered_entity_ids().to_vec();
        let outer_policy = u32::from(runtime.policy_raw_at_0x0c() != 0);
        let slot_bytes: Vec<u8> = match self
            .type_metadata
            .get(player_type as usize)
            .map(|metadata| &metadata.sub_j_attachment_descriptor)
        {
            Some(RetailRuntimeValue::Known(Some(descriptor))) => descriptor
                .slots
                .iter()
                .map(|slot| slot.policy_word_raw as u8)
                .collect(),
            _ => return Vec::new(),
        };
        let mut kept = Vec::with_capacity(rows.len());
        let mut requests = Vec::new();
        for child in rows {
            let Some(entity) = self.entities.iter_mut().find(|entity| entity.id == child) else {
                continue;
            };
            if entity.network_body.is_none() {
                kept.push(child);
                continue;
            }
            match entity.attached_to {
                Some(parent) if parent == player_id => {}
                None => {
                    let drawn = slot_bytes.get(kept.len()).copied().unwrap_or(0) != 0;
                    requests.push((child, outer_policy | if drawn { 2 } else { 0 }));
                    if !drawn {
                        entity.collision.state_flags_at_0x08.overwrite(0x800, 0);
                    }
                }
                Some(_) => continue,
            }
            let state = entity.collision.state_flags_at_0x08.known_value_bits();
            if state & 0x4000 != 0 || state == 0 {
                continue;
            }
            kept.push(child);
        }
        if let Some(runtime) = self.player_sub_j_runtime_mut() {
            runtime.commit_stable_compaction(kept);
        }
        requests
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
