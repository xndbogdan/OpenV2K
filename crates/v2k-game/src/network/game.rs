//! A network game in progress ([MULTIPLAYER.md](../../../../docs/re/MULTIPLAYER.md),
//! "Session tick, handles and queues").
//!
//! [`NetworkGame`] holds what retail keeps in globals between
//! `FUN_0046BED0`'s start and `FUN_0046BFC0`'s stop: the open session, the
//! slot table, the handles, the scores, both channels' outgoing messages and
//! the game channel's inbound queue. Each frame the frontend or the world
//! drives it: the network timer (`FUN_0046BD80`), the in-game step
//! (`FUN_00450CC0`) with the session tick (`FUN_00438CB0`) and the drain of
//! received records (`FUN_004695E0`).

use std::collections::{HashMap, VecDeque};
use std::time::Instant;

use super::framing::{self, Channel, Outgoing, Record};
use super::handles::{self, HandleTable};
use super::protocol::{BodySync, GameRecord, Stamp, KINDS, PERIOD_US};
use super::scoring::{LineTables, Scoring, ScoringEffect};
use super::slots::{SlotRecord, SlotTable, SLOTS};
use super::transport::{NetSession, NetSessionError, Received, SessionInfo, SystemMessage};
use crate::entity::network_body::{basis_rows, PLAYER_CRAFT_TYPE, REMOTE_CRAFT_TYPE};
use crate::entity::EntityManager;
use crate::entity_collision_state::{RetailRuntimeValue, BODY_BASIS_REBUILT_STATE_BIT};

/// `FUN_00469550(..., 0x40, ...)`: the game channel's inbound ring holds 64
/// messages; one arriving at a full ring is dropped.
pub const INBOUND_MESSAGES: usize = 64;
/// `FUN_0046BED0` re-arms the network timer every second.
pub const TIMER_US: u32 = 1_000_000;

/// What the world must do with a record the game cannot apply itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NetworkEvent {
    /// Kinds 22..=25: a positional sound (`FUN_0044F450`).
    Sound {
        sound_id: u16,
        position_raw: [i16; 3],
        gain: u32,
        rate: u32,
    },
    /// Kind 1: a peer's particle, its damage suppressed (byte 17 = 2).
    Particle([u8; 20]),
    /// Kind 2: a peer's burst.
    Burst([u8; 12]),
    /// Kind 9 to a body this machine owns (`FUN_00417E10` then
    /// `FUN_00414E90`): the amount, the attacker's type and slot.
    Damage {
        entity_id: u32,
        amount: u16,
        attacker_type: u16,
        attacker_slot: u8,
    },
    /// Kind 10: velocity and attitude deltas for an owned body.
    Impact { entity_id: u32, deltas: [i16; 6] },
    /// Kinds 3..=6 and 13..=15: records whose world owners the port holds.
    Held { kind: u8 },
    /// `FUN_00438CB0` could not tell whether a body rests: its state word's
    /// bits `0x08000080` are unknown, so it resends on the moving period.
    UnknownRestState,
    /// `FUN_00456750`: a winner was named.
    WinnerFlash,
    /// `FUN_00450CC0`: the session was lost, or this machine became the host
    /// (retail aborts the game for it).
    Abort,
    /// Kind 8 for a body this machine owns (`FUN_00417D90` ->
    /// `FUN_00410B70`): a peer picked up one of its power-ups.
    Destroy { entity_id: u32 },
}

/// The local side of a body this machine owns.
#[derive(Clone, Copy, Debug, Default)]
struct LocalBody {
    handle: u32,
    /// `+74`: time to the next sync record.
    sync_timer_us: u32,
    /// `+7C`.
    resend: u32,
    /// State bit 0: the next record carries every field (a new body, or a
    /// peer asked with kind 11).
    full: bool,
    /// `+78`: a peer sees this body in detail (kind 12).
    detail_wait_us: u32,
}

/// What a local body publishes in its sync record, gathered by the caller.
#[derive(Clone, Debug, Default)]
pub struct LocalBodyState {
    pub model_variables: Vec<u16>,
    pub shield: u32,
}

/// The world a game frame runs on.
pub struct WorldFrame<'a> {
    pub entities: &'a mut EntityManager,
    /// The logical world number (`FUN_00450C90`) and whether it is loaded.
    pub world: u16,
    pub world_active: bool,
    pub elapsed_us: u32,
    /// `DAT_004FED60`, the 50 Hz tick count.
    pub ticks: u32,
    pub random: &'a mut dyn FnMut() -> u32,
    /// The profile's name and initials (`FUN_00449270`).
    pub profile: (&'a str, &'a str),
    /// The local player's extra sync fields.
    pub player: LocalBodyState,
}

pub struct NetworkGame {
    session: NetSession,
    pub slots: SlotTable,
    pub handles: HandleTable,
    pub scoring: Scoring,
    pub tables: LineTables,
    session_out: Outgoing,
    game_out: Outgoing,
    inbound: VecDeque<Vec<u8>>,
    stamp: Stamp,
    /// `004DC9D8`: the world the stamp last recorded.
    stamp_world: Option<u16>,
    /// `004DC758`: the slot table as the scores last saw it.
    expected: [u32; SLOTS],
    /// `004DC7B8`: slots whose stamp this world accepted since the last
    /// session tick, which then asks for each unknown name among them.
    pub ready: [bool; SLOTS],
    /// The newest stamp counter heard from each slot (its block's `+4`).
    counters: [u32; SLOTS],
    /// `004DC9B0`: the host's world, learnt from its stamp.
    pub host_world: u16,
    /// `004F73C0`.
    pub session_lost: bool,
    /// `004F73BC`.
    pub became_host: bool,
    timer_us: u32,
    /// `004DC7C4`.
    period_us: u32,
    local: HashMap<u32, LocalBody>,
    /// A peer's body's handle by its local id.
    peers: HashMap<u32, u32>,
    pub events: Vec<NetworkEvent>,
}

impl NetworkGame {
    /// `FUN_0043CD50` -> `FUN_0046CB50`: a session named after this machine,
    /// with the slot authority here.
    pub fn host(
        session_name: &str,
        player_name: &str,
        port: u16,
        tables: LineTables,
    ) -> Result<Self, NetSessionError> {
        let session = NetSession::host(session_name, player_name, port, Instant::now())?;
        Ok(Self::start(session, true, tables))
    }

    /// `FUN_0043CD00` -> `FUN_0046CAC0`: a searched session, joined.
    pub fn join(
        info: &SessionInfo,
        player_name: &str,
        tables: LineTables,
    ) -> Result<Self, NetSessionError> {
        let session = NetSession::join(info, player_name)?;
        Ok(Self::start(session, false, tables))
    }

    /// `FUN_0046BED0`: the slot table restarts with the local id first and
    /// the session id taken from it; the handle blocks open for nine slots
    /// (`FUN_00438880`); `FUN_00439290` resets the scores.
    fn start(session: NetSession, host: bool, tables: LineTables) -> Self {
        let local_id = session.local_id();
        Self {
            slots: SlotTable::start(local_id, host),
            handles: HandleTable::new(SLOTS, 0x100),
            scoring: Scoring::new(),
            tables,
            session_out: Outgoing::new(Channel::Session),
            game_out: Outgoing::new(Channel::Game),
            inbound: VecDeque::new(),
            stamp: Stamp {
                session: local_id,
                ..Stamp::default()
            },
            stamp_world: None,
            expected: [0; SLOTS],
            ready: [false; SLOTS],
            counters: [0; SLOTS],
            host_world: 0,
            session_lost: false,
            became_host: false,
            timer_us: TIMER_US,
            period_us: 0,
            local: HashMap::new(),
            peers: HashMap::new(),
            events: Vec::new(),
            session,
        }
    }

    pub fn is_host(&self) -> bool {
        self.session.is_host()
    }

    pub fn local_slot(&self) -> u8 {
        self.slots.local
    }

    pub fn session_name(&self) -> &str {
        self.session.session_name()
    }

    pub fn port(&self) -> Option<u16> {
        self.session.port().ok()
    }

    /// `FUN_0046BFC0`: leaves the session.
    pub fn close(self) {
        self.session.close();
    }

    /// The receive thread (`FUN_0046C570`): system messages update the slot
    /// table and flags at once, session records apply at once, game
    /// messages wait in the ring for the frame's drain.
    pub fn poll(&mut self, now: Instant) {
        while let Some(received) = self.session.receive(now) {
            match received {
                Received::System(SystemMessage::PlayerDestroyed { id, .. }) => {
                    self.slots.remove(id)
                }
                Received::System(SystemMessage::PlayerCreated { .. }) => {}
                Received::System(SystemMessage::SessionLost) => self.session_lost = true,
                Received::System(SystemMessage::BecameHost) => {
                    self.slots.authority = true;
                    self.became_host = true;
                    self.slots.changed = true;
                }
                Received::Game { data, .. } => match framing::header(&data) {
                    Some((0, _)) => {
                        let slots = &mut self.slots;
                        framing::walk_records(&data, 0, 1, |_, record| slots.apply(record));
                    }
                    Some((1, _)) => {
                        if self.inbound.len() < INBOUND_MESSAGES {
                            self.inbound.push_back(data);
                        }
                    }
                    _ => {}
                },
            }
        }
    }

    /// `FUN_0046BD80`, on its one-second timer.
    pub fn advance_timer(&mut self, elapsed_us: u32) {
        if self.timer_us > elapsed_us {
            self.timer_us -= elapsed_us;
            return;
        }
        self.timer_us = TIMER_US;
        if self.slots.timer() {
            self.push(Channel::Session, &SlotRecord(&self.slots.clone()));
        }
        self.flush(Channel::Session);
    }

    /// The frontend's network rows (`FUN_0043D210`, `FUN_0043C7F0`): the
    /// local name, then the queued records, which can only teach the host's
    /// world here.
    pub fn frontend_frame(&mut self, profile: (&str, &str)) {
        self.record_names(None, profile);
        while let Some(message) = self.inbound.pop_front() {
            self.read_message(&message, None);
        }
    }

    /// `FUN_00450CC0`, after the gameplay frame.
    pub fn world_frame(&mut self, frame: &mut WorldFrame) {
        self.events.clear();
        let world = frame.world;
        if self.slots.changed || self.stamp_world != Some(world) {
            self.slots.changed = false;
            self.flush(Channel::Game);
            self.stamp_world = Some(world);
            self.refresh_stamp(world);
            self.begin_game_message();
        }
        self.assign_handles(frame.entities);
        if self.record_names(Some(frame.entities), frame.profile) {
            // FUN_00414990: the departed bodies leave at once.
        }
        self.session_tick(frame);
        while let Some(message) = self.inbound.pop_front() {
            self.read_message(&message, Some(frame));
        }
        let lapsed = frame.entities.advance_network_bodies(frame.elapsed_us);
        for id in lapsed {
            self.forget_peer_body(frame.entities, id);
        }
        if self.became_host || self.session_lost {
            self.events.push(NetworkEvent::Abort);
        }
    }

    /// `FUN_00438B60`: the stamp names this machine's session, slot and
    /// world, and the host's slot from the host.
    fn refresh_stamp(&mut self, world: u16) {
        self.stamp.counter = self.stamp.counter.wrapping_add(1);
        self.stamp.world = world;
        self.stamp.slot = self.slots.local;
        self.stamp.host_slot = if self.is_host() { self.slots.local } else { 0 };
    }

    fn begin_game_message(&mut self) {
        self.stamp.counter = self.stamp.counter.wrapping_add(1);
        let header = self.stamp.pack();
        if let Some(message) = self.game_out.begin(self.slots.local, Some(&header)) {
            self.session.send_all(&message);
        }
    }

    /// `FUN_00469200`: a record into the channel's message, sending the
    /// message first when it is full.
    fn push(&mut self, channel: Channel, record: &dyn Record) {
        let fits = match channel {
            Channel::Session => self.session_out.append(record),
            Channel::Game => self.game_out.append(record),
        };
        if fits {
            return;
        }
        match channel {
            Channel::Session => {
                if let Some(message) = self.session_out.begin(self.slots.local, None) {
                    self.session.send_all(&message);
                }
                self.session_out.append(record);
            }
            Channel::Game => {
                self.begin_game_message();
                self.game_out.append(record);
            }
        }
    }

    /// `FUN_004691D0`: sends the channel's message when it holds a record.
    fn flush(&mut self, channel: Channel) {
        let message = match channel {
            Channel::Session => self.session_out.take(),
            Channel::Game => self.game_out.take(),
        };
        if let Some(message) = message {
            self.session.send_all(&message);
        }
    }

    /// A record to every peer from outside the session tick.
    pub fn send(&mut self, record: GameRecord) {
        self.push(Channel::Game, &record);
    }

    /// `FUN_00439010`: the local name and, on the host, the frag limit; a
    /// slot whose player changed loses its score and name, and its bodies
    /// leave this machine. Returns whether bodies left.
    fn record_names(
        &mut self,
        entities: Option<&mut EntityManager>,
        profile: (&str, &str),
    ) -> bool {
        let local = self.slots.local;
        if local == 0 {
            return false;
        }
        let tables = self.tables.clone();
        self.scoring
            .record_local_name(local, profile, &tables, self.is_host());
        let mut departed = false;
        let mut entities = entities;
        for slot in 1..SLOTS {
            if slot as u8 == local || self.slots.ids[slot] == self.expected[slot] {
                continue;
            }
            let had_player = self.expected[slot] != 0;
            self.scoring.forget_slot(slot as u8, had_player);
            if had_player {
                self.ready[slot] = true;
                self.counters[slot] = 0;
                if let Some(entities) = entities.as_deref_mut() {
                    for (handle, id) in self.handles.slot_handles(slot as u8) {
                        self.handles.free(handle);
                        self.peers.remove(&id);
                        departed |= entities.remove_network_body(id);
                    }
                }
            }
            self.expected[slot] = self.slots.ids[slot];
        }
        departed
    }

    /// `FUN_00410BB0` for bodies born since: every local allocation without a
    /// handle takes one under the local slot and sends its whole record
    /// first; one gone sends its removal (`FUN_004149F0`, kind 8).
    fn assign_handles(&mut self, entities: &EntityManager) {
        let slot = self.slots.local;
        if slot == 0 {
            return;
        }
        let live = entities.network_local_ids();
        let gone: Vec<u32> = self
            .local
            .keys()
            .copied()
            .filter(|id| !live.contains(id))
            .collect();
        for id in gone {
            let body = self.local.remove(&id).unwrap();
            self.handles.free(body.handle);
            self.send(GameRecord::Removed(body.handle));
        }
        // A peer's body destroyed here (a power-up picked up) frees its
        // handle without a send (FUN_004149F0 -> FUN_0043A600).
        let gone_peers: Vec<u32> = self
            .peers
            .keys()
            .copied()
            .filter(|id| entities.network_entity(*id).is_none())
            .collect();
        for id in gone_peers {
            if let Some(handle) = self.peers.remove(&id) {
                self.handles.free(handle);
            }
        }
        for id in live {
            if self.local.contains_key(&id) {
                continue;
            }
            if let Some(handle) = self.handles.allocate(slot, id) {
                self.local.insert(
                    id,
                    LocalBody {
                        handle,
                        full: true,
                        ..LocalBody::default()
                    },
                );
            }
        }
    }

    /// The handle the network knows a local or peer body by.
    pub fn handle_of(&self, id: u32) -> Option<u32> {
        self.local
            .get(&id)
            .map(|body| body.handle)
            .or_else(|| self.peers.get(&id).copied())
    }

    /// A body's local id from its handle.
    pub fn body_of(&self, handle: u32) -> Option<u32> {
        self.handles.lookup(handle)
    }

    /// `FUN_00438CB0`: every 100 ms the live-body list, the host's winner
    /// line and status, a client's score word and the name requests; then
    /// each local body's sync record when its timer runs out.
    fn session_tick(&mut self, frame: &mut WorldFrame) {
        let elapsed = frame.elapsed_us;
        self.scoring.age_killer(elapsed);
        self.period_us = self.period_us.wrapping_add(elapsed);
        if self.period_us > PERIOD_US {
            self.period_us = 0;
            let detailed = self.detailed_peer_bodies(frame.entities);
            self.send(GameRecord::LiveList(detailed));
            if self.is_host() {
                let tables = self.tables.clone();
                let random = (frame.random)();
                if let Some(line) = self.scoring.host_announcement(frame.ticks, &tables, random) {
                    self.send(GameRecord::Line(line.into_bytes()));
                }
                // Kind 3, the sea level and terrain runs, is not sent.
                self.events.push(NetworkEvent::Held { kind: 3 });
                let (limit_winner, table) = self.scoring.scores_record();
                self.send(GameRecord::Scores {
                    limit_winner,
                    table,
                });
            } else {
                self.send(GameRecord::ScoreWord(self.scoring.score_word()));
            }
            for slot in 1..SLOTS {
                if std::mem::take(&mut self.ready[slot]) && !self.scoring.known[slot] {
                    self.send(GameRecord::NameRequest(slot as u32));
                }
            }
        }
        let player_id = frame.entities.player().map(|player| player.id);
        let mut ids: Vec<u32> = self.local.keys().copied().collect();
        ids.sort_by_key(|id| self.local[id].handle >> 16 & 0x3FF);
        for id in ids {
            let body = self.local.get_mut(&id).unwrap();
            body.detail_wait_us = body.detail_wait_us.saturating_sub(elapsed);
            if body.sync_timer_us >= elapsed {
                body.sync_timer_us -= elapsed;
                continue;
            }
            let Some(entity) = frame.entities.network_entity(id) else {
                continue;
            };
            // At rest: state 0x08000000 without 0x80.
            let resting = match entity.collision.state_flags_at_0x08.masked(0x0800_0080) {
                RetailRuntimeValue::Known(bits) => bits == 0x0800_0000,
                _ => {
                    self.events.push(NetworkEvent::UnknownRestState);
                    false
                }
            };
            body.sync_timer_us = if resting {
                ((frame.random)() & 0xFFFF) + 1_000_000
            } else if entity.capability_flags & 1 == 0 {
                ((frame.random)() & 0xFFF) + 70_000
            } else {
                35_000
            };
            body.resend = body.resend.wrapping_add(1);
            let player = (Some(id) == player_id).then_some(&frame.player);
            let record = self.sync_record(frame.entities, id, player);
            if let Some(record) = record {
                if let Some(body) = self.local.get_mut(&id) {
                    body.full = false;
                }
                self.send(GameRecord::Body(record));
            }
        }
        self.flush(Channel::Game);
    }

    /// `FUN_00417240` for a local body: its handle, send count and fields.
    fn sync_record(
        &self,
        entities: &EntityManager,
        id: u32,
        player: Option<&LocalBodyState>,
    ) -> Option<BodySync> {
        let body = self.local.get(&id)?;
        let entity = entities.network_entity(id)?;
        let mut sync = BodySync {
            handle: body.handle,
            resend: body.resend,
            position: entity.position_raw(),
            velocity: entity.velocity_raw(),
            ..BodySync::default()
        };
        // The record's flags are the body's state word's low half; the bits
        // naming fields are set by what this record carries.
        const FIELD_BITS: u16 =
            0x1 | 0x2 | 0x4 | 0x8 | 0x10 | 0x20 | 0x80 | 0x100 | 0x200 | 0x400 | 0x1000;
        let state = entity.collision.state_flags_at_0x08;
        let mut flags = (state.known_value_bits() as u16) & !FIELD_BITS;
        if state.masked(0x8000) == RetailRuntimeValue::Unresolved
            && Some(id) == entities.player().map(|p| p.id)
        {
            flags |= 0x8000;
        }
        flags |= 0x20 | 0x10;
        let euler = state.masked(BODY_BASIS_REBUILT_STATE_BIT)
            == RetailRuntimeValue::Known(BODY_BASIS_REBUILT_STATE_BIT);
        match entity.physical_body_basis_q31() {
            RetailRuntimeValue::Known(basis) if !euler => {
                flags |= 8;
                sync.basis = basis_rows(&basis);
            }
            _ => {
                flags |= 4;
                sync.euler = entity.rotation_heading_pitch_roll_raw();
            }
        }
        if body.full {
            flags |= 1 | 0x100;
            sync.entity_type = if entity.entity_type == PLAYER_CRAFT_TYPE {
                REMOTE_CRAFT_TYPE
            } else {
                entity.entity_type
            };
            sync.capability = entity.capability_flags;
            if let Some(payload) = entity.power_up_payload_packed {
                if payload != 0 {
                    flags |= 0x400;
                    sync.payload = payload;
                }
            }
        }
        if let Some(parent) = entity.attached_to.and_then(|parent| self.handle_of(parent)) {
            flags |= 0x1000;
            sync.parent = parent;
        }
        if let Some(player) = player {
            if !player.model_variables.is_empty() {
                flags |= 0x80;
                sync.model_variables = player.model_variables.clone();
            }
            if player.shield != 0 {
                flags |= 0x200;
                sync.shield = player.shield;
            }
        }
        sync.flags = flags;
        Some(sync)
    }

    /// `FUN_00417FC0`: the peer bodies this machine draws in detail.
    fn detailed_peer_bodies(&self, entities: &EntityManager) -> Vec<u32> {
        let mut handles: Vec<u32> = self
            .peers
            .iter()
            .filter(|(id, _)| {
                entities.network_entity(**id).is_some_and(|entity| {
                    entity.collision.state_flags_at_0x08.masked(0x0200_0000)
                        == RetailRuntimeValue::Known(0x0200_0000)
                })
            })
            .map(|(_, handle)| *handle)
            .collect();
        handles.sort_unstable();
        handles
    }

    fn forget_peer_body(&mut self, entities: &mut EntityManager, id: u32) {
        if let Some(handle) = self.peers.remove(&id) {
            self.handles.free(handle);
        }
        entities.remove_network_body(id);
    }

    /// `FUN_004693A0` over one game message: the stamp decides whether its
    /// records apply (`FUN_00438BE0`), then each record by kind.
    fn read_message(&mut self, message: &[u8], mut frame: Option<&mut WorldFrame>) {
        let Some(stamp) = message.get(4..16).and_then(Stamp::parse) else {
            return;
        };
        let slot = usize::from(stamp.slot);
        if slot == 0 || slot >= SLOTS || stamp.session != self.expected[slot] {
            return;
        }
        let local_world = frame
            .as_ref()
            .map(|frame| (frame.world, frame.world_active));
        let accept = match local_world {
            Some((world, true)) if world == stamp.world => {
                self.ready[slot] = true;
                true
            }
            _ => {
                if stamp.host_slot != 0 && self.counters[slot] < stamp.counter {
                    self.host_world = stamp.world;
                    self.counters[slot] = stamp.counter;
                }
                false
            }
        };
        if !accept {
            return;
        }
        let mut records = Vec::new();
        framing::walk_records(message, 12, KINDS, |kind, data| {
            match GameRecord::parse(kind, data) {
                Some((record, size)) => {
                    records.push(record);
                    size
                }
                None => 0,
            }
        });
        for record in records {
            if let Some(frame) = frame.as_deref_mut() {
                self.apply(record, stamp.slot, frame);
            }
        }
    }

    /// Each kind's apply routine.
    fn apply(&mut self, record: GameRecord, sender: u8, frame: &mut WorldFrame) {
        match record {
            GameRecord::Body(sync) => self.apply_body(sync, frame),
            GameRecord::Removed(handle) => {
                // FUN_00417D90: FUN_00410B70 on whichever body the handle
                // names, a peer's or one of this machine's.
                if let Some(id) = self.handles.lookup(handle) {
                    if self.peers.contains_key(&id) {
                        self.forget_peer_body(frame.entities, id);
                    } else if self.local.contains_key(&id) {
                        self.events.push(NetworkEvent::Destroy { entity_id: id });
                    }
                }
            }
            GameRecord::Resend(handle) => {
                // FUN_00417F30: state bit 1 asks for the whole record.
                if let Some(body) = self
                    .handles
                    .lookup(handle)
                    .and_then(|id| self.local.get_mut(&id))
                {
                    body.full = true;
                }
            }
            GameRecord::LiveList(list) => {
                // FUN_00417F60: a peer sees these bodies in detail.
                for handle in list {
                    if let Some(body) = self
                        .handles
                        .lookup(handle)
                        .and_then(|id| self.local.get_mut(&id))
                    {
                        body.detail_wait_us = 750_000;
                    }
                }
            }
            GameRecord::Damage {
                handle,
                amount,
                kind_slot,
            } => {
                if let Some(id) = self.handles.lookup(handle) {
                    if self.local.contains_key(&id) {
                        self.events.push(NetworkEvent::Damage {
                            entity_id: id,
                            amount,
                            attacker_type: (kind_slot & 0xFF).wrapping_sub(6) & 0xFF,
                            attacker_slot: (kind_slot >> 8) as u8,
                        });
                    }
                }
            }
            GameRecord::Impact { handle, deltas } => {
                if let Some(id) = self.handles.lookup(handle) {
                    if self.local.contains_key(&id) {
                        self.events.push(NetworkEvent::Impact {
                            entity_id: id,
                            deltas,
                        });
                    }
                }
            }
            GameRecord::Particle(request) => self.events.push(NetworkEvent::Particle(request)),
            GameRecord::Burst(request) => self.events.push(NetworkEvent::Burst(request)),
            GameRecord::Sound { word, rate, gain } => {
                // FUN_00439E90: the position's high bytes, each times 256.
                let position_raw = [
                    (word & 0xFF00) as i16,
                    (((word >> 16) & 0xFF) << 8) as i16,
                    (((word >> 24) & 0xFF) << 8) as i16,
                ];
                self.events.push(NetworkEvent::Sound {
                    sound_id: (word & 0xFF) as u16,
                    position_raw,
                    gain: gain.unwrap_or(0x10000),
                    rate: rate.unwrap_or(0x10000),
                });
            }
            GameRecord::ScoreWord(word) => {
                if self.is_host() {
                    let effects = self.scoring.apply_score_word(sender, word);
                    self.run_effects(effects);
                }
            }
            GameRecord::KillReport { victim, attacker } => {
                if self.is_host() {
                    let effects = self.scoring.score_hit(victim, attacker);
                    self.run_effects(effects);
                }
            }
            GameRecord::Scores {
                limit_winner,
                table,
            } => {
                let effects = self.scoring.apply_scores(limit_winner, &table);
                self.run_effects(effects);
            }
            GameRecord::NameRequest(slot) => {
                // FUN_00439720: a peer asks for this machine's name.
                let local = self.slots.local;
                if slot == u32::from(local) && self.scoring.known[usize::from(local)] {
                    let pad = |text: &str, n: usize| {
                        let mut bytes = vec![0u8; n];
                        for (i, b) in text.bytes().take(n - 1).enumerate() {
                            bytes[i] = b;
                        }
                        bytes
                    };
                    let initials = pad(&self.scoring.initials[usize::from(local)], 6);
                    let name = pad(&self.scoring.names[usize::from(local)], 16);
                    self.send(GameRecord::Name {
                        initials: initials.try_into().unwrap(),
                        name: name.try_into().unwrap(),
                    });
                }
            }
            GameRecord::Name { initials, name } => {
                // FUN_00439820: the sender's name.
                let text = |bytes: &[u8]| {
                    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
                    String::from_utf8_lossy(&bytes[..end]).into_owned()
                };
                let slot = usize::from(sender);
                self.scoring.initials[slot] = text(&initials);
                self.scoring.names[slot] = text(&name);
                self.scoring.known[slot] = true;
            }
            GameRecord::Line(text) => {
                // FUN_00439880: at most 79 characters.
                let line = String::from_utf8_lossy(&text[..text.len().min(79)]).into_owned();
                self.scoring.add_line(&line);
            }
            other => self.events.push(NetworkEvent::Held {
                kind: other.kind_of(),
            }),
        }
    }

    /// `FUN_00417680`: a known peer body takes the record's fields; an
    /// unknown one is born when the record carries its type, otherwise the
    /// owner is asked for the whole record (kind 11).
    fn apply_body(&mut self, sync: BodySync, frame: &mut WorldFrame) {
        let id = match self.handles.lookup(sync.handle) {
            Some(id) if self.peers.contains_key(&id) => id,
            Some(_) => return,
            None => {
                if handles::slot_of(sync.handle) == self.slots.local {
                    return;
                }
                if sync.flags & 1 == 0 {
                    self.send(GameRecord::Resend(sync.handle));
                    return;
                }
                let entity_type = if sync.entity_type == PLAYER_CRAFT_TYPE {
                    REMOTE_CRAFT_TYPE
                } else {
                    sync.entity_type
                };
                let Some(id) = frame.entities.append_network_body(entity_type) else {
                    return;
                };
                if !self.handles.register(sync.handle, id) {
                    frame.entities.remove_network_body(id);
                    return;
                }
                self.peers.insert(id, sync.handle);
                id
            }
        };
        let parent = (sync.flags & 0x1000 != 0)
            .then(|| self.handles.lookup(sync.parent))
            .flatten();
        frame.entities.apply_network_sync(id, &sync, parent);
    }

    fn run_effects(&mut self, effects: Vec<ScoringEffect>) {
        for effect in effects {
            match effect {
                ScoringEffect::SendKillReport { victim, attacker } => {
                    self.send(GameRecord::KillReport { victim, attacker })
                }
                ScoringEffect::SendScoreWord(word) => self.send(GameRecord::ScoreWord(word)),
                ScoringEffect::SendLine(line) => self.send(GameRecord::Line(line.into_bytes())),
                ScoringEffect::WinnerFlash => self.events.push(NetworkEvent::WinnerFlash),
            }
        }
    }

    /// `FUN_00414E90`'s branch for a peer's body: the damage goes to its
    /// owner (kind 9) with the attacker's type and this machine's slot.
    pub fn forward_damage(&mut self, entity_id: u32, amount: u16, attacker_type: u16) {
        let Some(handle) = self.peers.get(&entity_id).copied() else {
            return;
        };
        self.send(GameRecord::Damage {
            handle,
            amount,
            kind_slot: (attacker_type.wrapping_add(6) & 0xFF) | u16::from(self.slots.local) << 8,
        });
    }

    /// `FUN_00414E90`'s player branch: the local player was hit by another
    /// player (`FUN_00439DB0`), or killed (`FUN_00439C90`).
    pub fn player_damaged(&mut self, attacker_slot: u8, lethal: bool, random: u32) {
        let local = self.slots.local;
        let host = self.is_host();
        let effects = if lethal {
            let tables = self.tables.clone();
            self.scoring
                .announce_kill(local, attacker_slot, &tables, random, host)
        } else {
            self.scoring.report_hit(local, attacker_slot, host)
        };
        self.run_effects(effects);
    }

    /// `FUN_00414E90`'s branch for the local player's damage: a lethal one
    /// announces the kill, naming the attacker's slot when a player's craft
    /// (type 46) dealt it; a player's nonlethal hit credits its attacker.
    pub fn report_player_hits(
        &mut self,
        hits: Vec<crate::entity::network_body::PlayerHitReport>,
        random: &mut dyn FnMut() -> u32,
    ) {
        for hit in hits {
            let slot = hit.attacker_slot.unwrap_or(self.slots.local);
            if hit.lethal {
                let killer = if hit.attacker_type == PLAYER_CRAFT_TYPE {
                    slot
                } else {
                    0
                };
                self.player_damaged(killer, true, random());
            } else if hit.attacker_type == PLAYER_CRAFT_TYPE {
                self.player_damaged(slot, false, 0);
            }
        }
    }

    /// Kinds 1, 2 and 22..=25 a local birth or sound sends.
    pub fn send_effect(&mut self, record: GameRecord) {
        self.send(record);
    }

    /// Whether a body belongs to a peer.
    pub fn is_peer_body(&self, id: u32) -> bool {
        self.peers.contains_key(&id)
    }

    /// The live message lines, oldest first (`FUN_00438A20`).
    pub fn lines(&mut self, elapsed_us: u32) -> Vec<String> {
        self.scoring.age_lines(elapsed_us)
    }

    /// The slot a handle belongs to.
    pub fn slot_of_body(&self, id: u32) -> Option<u8> {
        self.handle_of(id).map(handles::slot_of)
    }
}

#[cfg(test)]
#[path = "game_tests.rs"]
mod tests;
