//! The Playing frame's side of network play: what it hands the network game
//! after the gameplay frame (`FUN_00450CC0`), and what it applies of the
//! records peers sent.

use std::collections::HashSet;
use std::sync::Mutex;

use v2k_game::damage::DamageDeliveryRecord;
use v2k_game::entity::{
    EntityManager, PlayerCheckedDamageFrame, PlayerCheckedDamageOutcome,
    PlayerCheckedDamageRequest, PlayerDamageEntry,
};
use v2k_game::network::game::{LocalBodyState, NetworkEvent, NetworkGame, WorldFrame};
use v2k_game::network::protocol::GameRecord;
use v2k_game::player::PlayerCraft;
use v2k_game::player_hull::PlayerHull;
use v2k_game::world_fx::{
    NetworkParticleBirth, NetworkSend, ParticleEnvironment, TerrainCollisionContext, WorldFx,
};

pub struct NetworkWorldFrame<'a> {
    pub em: &'a mut EntityManager,
    pub world_fx: &'a mut WorldFx,
    pub cache: &'a mut v2k_game::resource_cache::ResourceCache,
    pub static_damage: &'a mut v2k_game::static_damage::StaticDamageScheduler,
    pub scheduler:
        &'a mut v2k_game::specialized_actor_task_production::SpecializedActorTaskScheduler,
    pub hull: &'a mut PlayerHull,
    pub notifications: &'a mut v2k_game::gameplay_notifications::GameplayNotifications,
    pub extra_lives: u8,
    pub craft: &'a PlayerCraft,
    /// The logical world number the stamp names.
    pub world: u16,
    pub elapsed_micros: u32,
    pub retail_tick: u32,
    /// Power-ups the player picked up this frame.
    pub removals: &'a mut Vec<u32>,
}

#[derive(Debug, Default)]
pub struct NetworkWorldOutcome {
    pub winner_flash: bool,
    /// The local player died this frame (`FUN_00447280` ran).
    pub player_died: bool,
    pub abort: bool,
}

fn verbose(message: std::fmt::Arguments) {
    if crate::VERBOSE.load(std::sync::atomic::Ordering::Relaxed) {
        eprintln!("{message}");
    }
}

/// One stop line per kind of record the port holds.
pub fn report_held(what: String) {
    static REPORTED: Mutex<Option<HashSet<String>>> = Mutex::new(None);
    let mut reported = REPORTED
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if reported
        .get_or_insert_with(HashSet::new)
        .insert(what.clone())
    {
        eprintln!("Network: {what} is held");
    }
}

/// `FUN_00439F80`'s 20 bytes from a birth and its owner's handle.
fn particle_request(birth: NetworkParticleBirth, owner_handle: u32) -> [u8; 20] {
    let mut out = [0u8; 20];
    let words = birth.position_raw.iter().chain(&birth.velocity_raw);
    for (index, word) in words.enumerate() {
        out[index * 2..index * 2 + 2].copy_from_slice(&word.to_le_bytes());
    }
    out[12..16].copy_from_slice(&owner_handle.to_le_bytes());
    out[16] = birth.source_class;
    out[18] = birth.owner_entity_type;
    out
}

/// `FUN_00413500` on the host: while fewer than 32 Type61s live (peers'
/// too), each second `FUN_004132D0` tries a power-up from the table at
/// `0x4C92D0` (`DAT_004DB0A4` is the clock).
pub fn advance_power_up_spawner(
    em: &mut EntityManager,
    world_fx: &mut WorldFx,
    terrain: &v2k_formats::terrain::TerrainGrid,
    clock_us: &mut u32,
    elapsed_micros: u32,
    retail_tick: u32,
) {
    use v2k_game::network::power_ups::{spawn, SPAWN_LIMIT, SPAWN_PERIOD_US};
    let live = em
        .iter_all()
        .filter(|entity| entity.entity_type == 61)
        .count();
    if live >= SPAWN_LIMIT {
        return;
    }
    if *clock_us >= elapsed_micros {
        *clock_us -= elapsed_micros;
        return;
    }
    *clock_us = SPAWN_PERIOD_US;
    let bodies: Vec<[i16; 3]> = em.iter_all().map(|entity| entity.position_raw()).collect();
    if let Some((position_raw, payload)) = spawn(
        &mut || world_fx.next_shared_retail_random_u16(),
        bodies,
        |x, z| terrain.bilinear_height_raw(x, z),
    ) {
        match em.append_network_power_up(position_raw, payload, terrain, retail_tick, world_fx) {
            Ok(id) => verbose(format_args!(
                "Network: a power-up {payload:#x} at {position_raw:?} (body {id})"
            )),
            Err(reason) => report_held(format!("a host power-up ({reason})")),
        }
    }
}

/// `FUN_00439DF0`'s word: the cue's low byte, then the high bytes of the
/// position's X, Y and Z.
fn sound_word(sound_id: u16, position_raw: [i16; 3]) -> u32 {
    let high = |word: i16| u32::from(word as u16 >> 8);
    u32::from(sound_id & 0xFF)
        | high(position_raw[0]) << 8
        | high(position_raw[1]) << 16
        | high(position_raw[2]) << 24
}

fn particle_birth(request: &[u8; 20], owner_id: Option<u32>) -> NetworkParticleBirth {
    let word = |index: usize| i16::from_le_bytes([request[index * 2], request[index * 2 + 1]]);
    NetworkParticleBirth {
        source_class: request[16],
        position_raw: [word(0), word(1), word(2)],
        velocity_raw: [word(3), word(4), word(5)],
        owner_id,
        owner_entity_type: request[18],
    }
}

pub fn advance_network_world(
    net: &mut NetworkGame,
    frame: NetworkWorldFrame<'_>,
) -> NetworkWorldOutcome {
    let NetworkWorldFrame {
        em,
        world_fx,
        cache,
        static_damage,
        scheduler,
        hull,
        notifications,
        extra_lives,
        craft,
        world,
        elapsed_micros,
        retail_tick,
        removals,
    } = frame;
    // This frame's sends from the effects, in call order: FUN_00440A60's
    // kind 1, FUN_00440950's kind 2 and FUN_0044F480's kinds 22 and 23.
    for send in world_fx.take_network_sends() {
        match send {
            NetworkSend::Particle(birth) => {
                let owner = birth.owner_id.and_then(|id| net.handle_of(id)).unwrap_or(0);
                net.send(GameRecord::Particle(particle_request(birth, owner)));
            }
            NetworkSend::Burst(request) => {
                let mut record = [0u8; 12];
                for (bytes, word) in record.chunks_exact_mut(2).zip(request) {
                    bytes.copy_from_slice(&word.to_le_bytes());
                }
                net.send(GameRecord::Burst(record));
            }
            NetworkSend::Sound {
                sound_id,
                position_raw,
                rate_q16,
            } => net.send(GameRecord::Sound {
                word: sound_word(sound_id, position_raw),
                rate: (rate_q16 != 0x1_0000).then_some(rate_q16),
                gain: None,
            }),
            NetworkSend::Impact { entity_id, deltas } => {
                if let Some(handle) = net.handle_of(entity_id) {
                    verbose(format_args!(
                        "Network: peer body {entity_id} jolted by {deltas:?} (kind 10)"
                    ));
                    net.send(GameRecord::Impact { handle, deltas });
                }
            }
            NetworkSend::BlastImpulse { .. } => {
                report_held("FUN_00414AE0's kind 10 (the previous target's handle)".into())
            }
            NetworkSend::Damage {
                entity_id,
                amount,
                attacker_type,
            } => {
                verbose(format_args!(
                    "Network: {amount} damage from a Type{attacker_type} to peer body {entity_id}"
                ));
                net.forward_damage(entity_id, amount, attacker_type);
            }
            NetworkSend::CargoDropped {
                parent_id,
                cargo_id,
                x_raw,
                z_raw,
            } => {
                if let (Some(parent), Some(cargo)) =
                    (net.handle_of(parent_id), net.handle_of(cargo_id))
                {
                    verbose(format_args!(
                        "Network: peer body {cargo_id} dropped at ({x_raw}, {z_raw}) (kind 15)"
                    ));
                    net.send(GameRecord::Dropped([
                        parent,
                        cargo,
                        u32::from(x_raw as u16) | u32::from(z_raw as u16) << 16,
                    ]));
                }
            }
        }
    }
    // FUN_00418640 on the player: kind 13 asks the owner of each peer's body
    // in its cargo to attach it.
    let player_handle = em.player().and_then(|player| net.handle_of(player.id));
    for (cargo_id, flags) in em.network_player_cargo_rows() {
        if let (Some(parent), Some(cargo)) = (player_handle, net.handle_of(cargo_id)) {
            verbose(format_args!(
                "Network: peer body {cargo_id} in the cargo, flags {flags} (kind 13)"
            ));
            net.send(GameRecord::Attach([parent, cargo, flags]));
        }
    }
    // FUN_00425AF0 sends kind 8 for each power-up the player picked up,
    // a peer's or this machine's, before it is destroyed.
    for id in removals.drain(..) {
        if let Some(handle) = net.handle_of(id) {
            net.send(GameRecord::Removed(handle));
        }
    }
    send_cell_changes(net, cache, static_damage);
    for reason in em.take_refused_network_drops() {
        report_held(format!("a dropped power-up ({reason})"));
    }
    let lines_before = net.scoring.lines.clone();
    let scores_before = (net.scoring.table, net.scoring.winner);
    // FUN_00414E90's kill and hit reports for the local player.
    let hits = em.take_network_player_hits();
    for hit in &hits {
        verbose(format_args!("Network: the player took a hit {hit:?}"));
    }
    let mut outcome = NetworkWorldOutcome {
        player_died: hits.iter().any(|hit| hit.lethal),
        ..NetworkWorldOutcome::default()
    };
    net.report_player_hits(hits, &mut || {
        u32::from(world_fx.next_shared_retail_random_u16())
    });
    let (name, initials) = v2k_game::network::arena::profile_names(net.local_slot());
    let vars = craft.anim_vars();
    let player = LocalBodyState {
        // The craft's model selectors 1..=7 travel in order.
        model_variables: (1..8)
            .map(|selector| vars.dynamic[selector] as u16)
            .collect(),
        shield: hull.pre_health_damage_buffer_raw.max(0) as u32,
    };
    {
        let model_extent = |id: usize| cache.global_model(id).map(|model| model.radius);
        let mut world_frame = WorldFrame {
            entities: em,
            world,
            world_active: true,
            elapsed_us: elapsed_micros,
            ticks: retail_tick,
            fx: &mut *world_fx,
            model_extent_raw: Some(&model_extent),
            profile: (&name, &initials),
            player,
            terrain: cache.level_terrain(),
        };
        net.world_frame(&mut world_frame);
    }
    for (index, line) in net.scoring.lines.iter().enumerate() {
        if line.0 != 0 && *line != lines_before[index] {
            verbose(format_args!("Network: line \"{}\"", line.1));
        }
    }
    if (net.scoring.table, net.scoring.winner) != scores_before {
        let rows: Vec<String> = (1..9)
            .filter(|&slot| net.scoring.known[slot] || net.scoring.table[slot].score != 0)
            .map(|slot| {
                let row = net.scoring.table[slot];
                format!(
                    "{} {}/{}/{}",
                    net.scoring.names[slot], row.score, row.kills, row.deaths
                )
            })
            .collect();
        verbose(format_args!(
            "Network: scores {} (limit {}, winner {})",
            rows.join(", "),
            net.scoring.frag_limit,
            net.scoring.winner
        ));
    }
    for event in std::mem::take(&mut net.events) {
        match event {
            NetworkEvent::Sound {
                sound_id,
                position_raw,
                gain,
                rate,
            } => {
                if gain != 0x1_0000 {
                    report_held(format!("a sound's gain word ({gain:#x})"));
                }
                verbose(format_args!(
                    "Network: a peer's cue {sound_id} at {position_raw:?}"
                ));
                world_fx.queue_fixed_positional_sound_raw_at_rate(sound_id, position_raw, rate);
            }
            NetworkEvent::Particle(request) => {
                let handle =
                    u32::from_le_bytes([request[12], request[13], request[14], request[15]]);
                let birth = particle_birth(&request, net.body_of(handle));
                let environment = TerrainCollisionContext::from_current_level_cache(cache)
                    .map_or(ParticleEnvironment::Dry, ParticleEnvironment::Terrain);
                if world_fx
                    .materialize_network_particle(birth, environment, retail_tick)
                    .is_none()
                {
                    report_held(format!("a peer's class-{} particle", birth.source_class));
                }
            }
            NetworkEvent::Damage {
                entity_id,
                amount,
                attacker_type,
                attacker_slot,
            } => {
                if em.player().map(|player| player.id) != Some(entity_id) {
                    let entity_type = em.network_entity(entity_id).map_or(0, |e| e.entity_type);
                    let result = v2k_game::attached_particle_damage::apply_network_damage(
                        v2k_game::attached_particle_damage::AttachedParticleDamageFrame {
                            resources: &mut *cache,
                            entities: &mut *em,
                            world_fx: &mut *world_fx,
                            static_damage: &mut *static_damage,
                            scheduler: &mut *scheduler,
                            notifications: &mut *notifications,
                            retail_tick,
                            world: v2k_game::attached_particle_damage::AttachedParticleDamageWorld::Playing {
                                player_hull: &mut *hull,
                                extra_lives: v2k_game::entity_collision_state::RetailRuntimeValue::Known(extra_lives),
                            },
                        },
                        entity_id,
                        amount,
                        attacker_type,
                    );
                    verbose(format_args!(
                        "Network: a peer's {amount} damage to owned Type{entity_type} {entity_id}: {result:?}"
                    ));
                    if result.is_err() {
                        report_held(format!("a peer's damage to an owned Type{entity_type}"));
                    }
                    continue;
                }
                let outcome = em.apply_player_checked_damage(
                    PlayerCheckedDamageRequest {
                        entry: PlayerDamageEntry::Network {
                            amount,
                            attacker_slot,
                        },
                        target_id: entity_id,
                        delivery: DamageDeliveryRecord {
                            source_entity_type_raw: u32::from(attacker_type),
                            ..DamageDeliveryRecord::default()
                        },
                        ratio_numerator: 0,
                        ratio_denominator: 0,
                    },
                    PlayerCheckedDamageFrame {
                        hull: &mut *hull,
                        resources: cache,
                        world_fx: &mut *world_fx,
                        retail_tick,
                        notifications: &mut *notifications,
                        extra_lives,
                    },
                );
                if let PlayerCheckedDamageOutcome::Blocked(block) = outcome {
                    report_held(format!("a peer's damage to the player ({block:?})"));
                }
            }
            NetworkEvent::Impact { entity_id, deltas } => {
                // FUN_00417EB0: velocity (+9C..+A0) and attitude (+A2..+A6)
                // deltas, wrapping words, as FUN_00411030 writes them.
                verbose(format_args!(
                    "Network: a peer's kind 10 on owned body {entity_id}: {deltas:?}"
                ));
                if let Some(entity) = em.entity_mut(entity_id) {
                    let velocity = entity.velocity_raw();
                    entity.velocity = std::array::from_fn(|axis| {
                        f32::from(velocity[axis].wrapping_add(deltas[axis])) / 256.0
                    });
                    let attitude = entity.rotation_heading_pitch_roll_raw();
                    entity.set_rotation_heading_pitch_roll_raw(std::array::from_fn(|axis| {
                        attitude[axis].wrapping_add(deltas[3 + axis])
                    }));
                }
            }
            NetworkEvent::Burst(record) => {
                // FUN_0043A020: the request's six words, network byte 2.
                let word = |i: usize| u16::from_le_bytes([record[i * 2], record[i * 2 + 1]]);
                let sea_level_raw = cache
                    .terrain()
                    .and_then(|terrain| terrain.water_enabled().then(|| terrain.sea_level_raw()));
                let request: [u16; 6] = std::array::from_fn(word);
                verbose(format_args!("Network: a peer's burst {request:?}"));
                world_fx.materialize_network_burst(request, sea_level_raw);
            }
            NetworkEvent::Terrain { record, from_host } => CellWorld {
                cache: &mut *cache,
                em: &mut *em,
                world_fx: &mut *world_fx,
                scheduler: &mut *static_damage,
                host_record: from_host,
            }
            .apply_terrain(&record),
            NetworkEvent::Cell { record, from_host } => CellWorld {
                cache: &mut *cache,
                em: &mut *em,
                world_fx: &mut *world_fx,
                scheduler: &mut *static_damage,
                host_record: from_host,
            }
            .apply_cell(record),
            NetworkEvent::CellRequest(record) => CellWorld {
                cache: &mut *cache,
                em: &mut *em,
                world_fx: &mut *world_fx,
                scheduler: &mut *static_damage,
                host_record: false,
            }
            .apply_cell_request(record),
            NetworkEvent::StaticDestroyed(cell) => {
                use v2k_game::static_damage::StaticNetworkDestruction as Outcome;
                let Some(now) = cache
                    .level_terrain()
                    .and_then(|terrain| terrain.cell(usize::from(cell[0]), usize::from(cell[1])))
                    .copied()
                else {
                    continue;
                };
                let kind_index = cache
                    .terrain_objects()
                    .and_then(|objects| objects.records.get(usize::from(now.attribute)))
                    .map(|object| object.kind_index);
                let outcome = static_damage.submit_network_destruction(
                    cell,
                    now.attribute,
                    now.terrain_type,
                    kind_index,
                );
                verbose(format_args!(
                    "Network: a peer's static program at {cell:?}: {outcome:?}"
                ));
                match outcome {
                    Outcome::UnsupportedKind { kind_index } => report_held(format!(
                        "a peer's kind 6 for a static of kind {kind_index}, outside the admitted kinds"
                    )),
                    // FUN_004281A0 would read the null program: a retail crash.
                    Outcome::NullProgram { kind_index } => report_held(format!(
                        "a peer's kind 6 for a static of kind {kind_index}, whose program is null"
                    )),
                    _ => {}
                }
            }
            NetworkEvent::Held { kind } => report_held(format!("record kind {kind}")),
            NetworkEvent::HollowPeerBody {
                entity_type,
                reason,
            } => report_held(format!("a peer's Type{entity_type} copy ({reason})")),
            NetworkEvent::UnknownAttachmentList => {
                report_held("a peer's body named by an unresolved attachment list".into())
            }
            NetworkEvent::UnknownRestState => {
                report_held("the rest period of a body with unknown state bits".into())
            }
            NetworkEvent::WinnerFlash => {
                verbose(format_args!("Network: the winner flash (FUN_00456750)"));
                outcome.winner_flash = true;
            }
            NetworkEvent::Abort => outcome.abort = true,
            NetworkEvent::Destroy { entity_id } => {
                // Only FUN_00425AF0 sends kind 8 for another machine's body.
                match em
                    .network_entity(entity_id)
                    .map(|entity| entity.entity_type)
                {
                    Some(61) => em.queue_power_up_destroys(&[entity_id]),
                    Some(entity_type) => {
                        report_held(format!("a peer's removal of an owned Type{entity_type}"))
                    }
                    None => {}
                }
            }
        }
    }
    // What the records just applied changed goes out with this frame's
    // message, as retail's drain sends it at once.
    send_cell_changes(net, cache, static_damage);
    outcome
}

/// `FUN_00438F60` for each cell change the frame made: the host's kind 4,
/// a client's kind 5.
fn send_cell_changes(
    net: &mut NetworkGame,
    cache: &mut v2k_game::resource_cache::ResourceCache,
    static_damage: &mut v2k_game::static_damage::StaticDamageScheduler,
) {
    // FUN_00427950 and FUN_00427B20's kind 6: the cell's X, then Z.
    for cell in static_damage.take_network_starts() {
        verbose(format_args!("Network: static program at {cell:?} sent"));
        net.send(GameRecord::StaticDestroyed(
            u32::from(cell[0]) | u32::from(cell[1]) << 8,
        ));
    }
    let host = net.is_host();
    for change in cache.take_network_cell_changes() {
        let record = v2k_game::network::terrain::cell_change_record(change, host);
        verbose(format_args!(
            "Network: cell change {change:?} sent as {record:?}"
        ));
        net.send(record);
    }
}

/// The world a received terrain or cell record writes; `host_record` is
/// retail's `004DC9B4` (no notification, no burn burst).
struct CellWorld<'a> {
    cache: &'a mut v2k_game::resource_cache::ResourceCache,
    em: &'a mut EntityManager,
    world_fx: &'a mut WorldFx,
    scheduler: &'a mut v2k_game::static_damage::StaticDamageScheduler,
    host_record: bool,
}

impl CellWorld<'_> {
    fn suspended(&mut self, apply: impl FnOnce(&mut Self)) {
        let journal = if self.host_record {
            self.cache.take_network_cell_journal()
        } else {
            None
        };
        apply(self);
        if self.host_record {
            self.cache.set_network_cell_journal(journal);
        }
    }

    fn cell(&self, cell: [u8; 2]) -> Option<v2k_formats::terrain::TerrainCell> {
        self.cache
            .level_terrain()?
            .cell(usize::from(cell[0]), usize::from(cell[1]))
            .copied()
    }

    /// `FUN_00433720` and its listeners, at the cell's corner.
    fn infect(&mut self, cell: [u8; 2], infected: bool) {
        if let Err(listener) = v2k_game::infection_listeners::write_infection(
            self.cache,
            self.world_fx,
            v2k_game::infection_evolution::InfectionCellWrite::at_cell(cell, infected),
        ) {
            report_held(format!("a peer's infection at a cell ({listener})"));
        }
    }

    /// `FUN_00433900`'s first listener, the radar's `FUN_0044AA50`.
    fn radar_listener(&mut self, cell: [u8; 2]) {
        if let Err(phase) =
            v2k_game::static_terrain_burn::radar_cell_listener(cell, self.cache, self.world_fx)
        {
            report_held(format!("a peer's attribute at a cell ({phase})"));
        }
    }

    /// `FUN_004337E0` and its `FUN_00427760` callback.
    fn burn(&mut self, cell: [u8; 2], burned: bool) {
        if let Err(failure) = v2k_game::static_terrain_burn::apply_network_terrain_burn(
            cell,
            burned,
            self.host_record,
            self.cache,
            self.em,
            self.world_fx,
            self.scheduler,
        ) {
            report_held(format!("a peer's burn at a cell ({})", failure.phase));
        }
    }

    /// `FUN_00437910` -> `FUN_004379C0`: the host's sea level, then each run.
    fn apply_terrain(&mut self, record: &[u8]) {
        if self.cache.level_terrain().is_none() {
            return;
        }
        let Some((sea, mut cursor, runs)) = v2k_game::network::terrain::parse_runs(record) else {
            return;
        };
        let before = self.cache.level_terrain().map(|terrain| {
            terrain
                .cells
                .iter()
                .filter(|cell| cell.terrain_type & 0x18 != 0)
                .count()
        });
        verbose(format_args!(
            "Network: terrain runs from {cursor:?}, {} runs over {} cells, sea {sea} (host {})",
            runs.len(),
            runs.iter().map(|run| u32::from(run.0)).sum::<u32>(),
            self.host_record
        ));
        self.suspended(|world| {
            if world.host_record {
                if let Some(terrain) = world.cache.level_terrain_mut() {
                    terrain.header[0] = sea.wrapping_shl(8);
                }
            }
            for (count, flags) in runs {
                let infected = flags & 1 != 0;
                let burned = flags & 2 != 0;
                for _ in 0..count {
                    let cell = [cursor[0] as u8, cursor[1] as u8];
                    if let Some(current) = world.cell(cell) {
                        if (current.terrain_type & 0x10 != 0) != infected {
                            world.infect(cell, infected);
                        }
                        // The attribute byte is read once, before the burn.
                        if current.attribute != 0 {
                            let now = world.cell(cell).unwrap_or(current);
                            if (now.terrain_type & 0x08 != 0) != burned {
                                world.burn(cell, burned);
                            }
                            if flags & 4 != 0 {
                                world.cache.take_level_terrain_object_attribute(cell);
                            }
                        }
                    }
                    v2k_game::network::terrain::advance(&mut cursor);
                }
            }
        });
        let after = self.cache.level_terrain().map(|terrain| {
            terrain
                .cells
                .iter()
                .filter(|cell| cell.terrain_type & 0x18 != 0)
                .count()
        });
        if before != after {
            verbose(format_args!(
                "Network: terrain runs changed infected/burned cells {before:?} -> {after:?}"
            ));
        }
    }

    /// `FUN_00437B60`: one cell's attribute byte, infection and burn.
    fn apply_cell(&mut self, [x, z, flags, attribute]: [u8; 4]) {
        let cell = [x, z];
        let Some(old) = self.cell(cell) else {
            return;
        };
        self.suspended(|world| {
            if world
                .cell(cell)
                .is_some_and(|now| now.attribute != attribute)
            {
                world
                    .cache
                    .set_level_terrain_object_attribute(cell, attribute);
                world.radar_listener(cell);
            }
            if world
                .cell(cell)
                .is_some_and(|now| (now.terrain_type ^ flags) & 0x10 != 0)
            {
                world.infect(cell, flags & 0x10 != 0);
            }
            if world
                .cell(cell)
                .is_some_and(|now| (now.terrain_type ^ flags) & 0x08 != 0)
            {
                world.burn(cell, flags & 0x08 != 0);
            }
        });
        verbose(format_args!(
            "Network: cell {cell:?} {:#x}/{} -> {:?} (host {})",
            old.terrain_type,
            old.attribute,
            self.cell(cell).map(|now| (now.terrain_type, now.attribute)),
            self.host_record
        ));
    }

    /// `FUN_00437CB0` on the host: a client's request, applied only when it
    /// changes the cell (an unknown operation does nothing).
    fn apply_cell_request(&mut self, [x, z, operation, attribute]: [u8; 4]) {
        let cell = [x, z];
        let Some(now) = self.cell(cell) else {
            return;
        };
        let infected = now.terrain_type & 0x10 != 0;
        let burned = now.terrain_type & 0x08 != 0;
        verbose(format_args!(
            "Network: a client's request {operation} at {cell:?} ({:#x}/{})",
            now.terrain_type, now.attribute
        ));
        match operation {
            0 if !infected => self.infect(cell, true),
            1 if infected => self.infect(cell, false),
            2 if !burned => self.burn(cell, true),
            3 if burned => self.burn(cell, false),
            4 if now.attribute != attribute => {
                self.cache
                    .set_level_terrain_object_attribute(cell, attribute);
                self.radar_listener(cell);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sound_word_keeps_the_cue_and_the_position_high_bytes() {
        assert_eq!(sound_word(0x13E, [0x1234, -0x0100, 0x7F00]), 0x7F_FF_12_3E);
    }

    #[test]
    fn a_particle_request_keeps_its_bytes() {
        let birth = NetworkParticleBirth {
            source_class: 38,
            position_raw: [1, -2, 3],
            velocity_raw: [-4, 5, -6],
            owner_id: None,
            owner_entity_type: 46,
        };
        let request = particle_request(birth, 0x0C00_FF01);
        assert_eq!(&request[12..19], &[1, 0xFF, 0, 0x0C, 38, 0, 46]);
        assert_eq!(
            particle_birth(&request, Some(7)),
            NetworkParticleBirth {
                owner_id: Some(7),
                ..birth
            }
        );
    }
}

/// `FUN_00452CB0`'s network part and `FUN_00438A20`: the score table along
/// the top and the message lines below it, in the selected tier's canvas.
pub fn draw_network_hud(
    renderer: &mut dyn v2k_render::Renderer,
    fonts: &v2k_game::menu_text::MenuFonts,
    cache: &v2k_game::resource_cache::ResourceCache,
    net: &mut NetworkGame,
    retail_tick: u32,
    elapsed_micros: u32,
) {
    let (width, height) = renderer.viewport_size();
    let mapping = v2k_render::UiMapping::new(v2k_render::UiMappingRequest {
        viewport: [width, height],
        authored_canvas: [fonts.virtual_w as u32, fonts.virtual_h as u32],
        policy: renderer.ui_submission_policy(),
    });
    let canvas = [fonts.virtual_w as i32, fonts.virtual_h as i32];
    let scoring = &net.scoring;
    let limit = i32::from(scoring.frag_limit);
    // Without a limit every row shows; with one, rows at or above the
    // closing threshold blink, and after a winner only the winner holds.
    let all = limit == 0 || (retail_tick / 20) & 1 == 1;
    let threshold = limit - (limit / 2).min(50);
    let leader = (1..9)
        .filter(|&slot| scoring.known[slot])
        .map(|slot| scoring.table[slot].score)
        .max()
        .unwrap_or(0)
        .max(1);
    for slot in 1..9usize {
        let row = scoring.table[slot];
        let shown = all
            || if scoring.winner == 0 {
                i32::from(row.score) < threshold
            } else {
                usize::from(scoring.winner) == slot
            };
        if !shown || !scoring.known[slot] {
            continue;
        }
        let index = slot as i32 - 1;
        let x = canvas[0] / 4 * (index % 4) + canvas[0] / 100;
        let y = canvas[1] * 4 / 100 * (index / 4 + 1);
        let initials: String = scoring.initials[slot].chars().take(5).collect();
        let text = format!("{initials}:{}::{}-{}", row.score, row.kills, row.deaths);
        crate::menu_text_submission::draw_menu_text(
            renderer,
            fonts.font(row.score == leader),
            &text,
            [x as f32, y as f32],
            false,
            usize::MAX,
            mapping,
        );
    }
    // FUN_00438A20: system level 3's local point 30 and level 2's scalar 18
    // place the lines, each word-wrapped to the screen's right edge.
    let lines = net.lines(elapsed_micros);
    let (Some((x, y)), Some(step)) = (
        cache.system_layout_point(3, 30),
        cache.system_data_value(2, 18),
    ) else {
        return;
    };
    let font = fonts.font(true);
    let wrap = (canvas[0] - i32::from(x)) as f32;
    let mut baseline = f32::from(y);
    for line in lines {
        for wrapped in wrap_words(font, &line, wrap) {
            crate::menu_text_submission::draw_menu_text(
                renderer,
                font,
                &wrapped,
                [f32::from(x), baseline],
                false,
                usize::MAX,
                mapping,
            );
            baseline += font.line_step;
        }
        baseline += step as f32 - font.line_step;
    }
}

/// `FUN_00470AE0`'s wrap: at spaces, a whole word to the next line.
fn wrap_words(font: &v2k_game::menu_text::MenuFont, text: &str, width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split(' ').filter(|word| !word.is_empty()) {
        let candidate = if line.is_empty() {
            word.to_string()
        } else {
            format!("{line} {word}")
        };
        if !line.is_empty() && font.measure(&candidate) > width {
            lines.push(std::mem::replace(&mut line, word.to_string()));
        } else {
            line = candidate;
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// `FUN_00421590` in a network game: a peer's craft wears its initials
/// above it (-100, +128 raw from its origin), once its world Z exceeds the
/// view origin's by more than 256. Retail queues each tag's draw through
/// `FUN_00421840` and `FUN_004218A0`.
pub fn draw_name_tags(
    renderer: &mut dyn v2k_render::Renderer,
    fonts: &v2k_game::menu_text::MenuFonts,
    camera: &v2k_render::Camera,
    em: &EntityManager,
    net: &NetworkGame,
) {
    let (width, height) = renderer.viewport_size();
    let mapping = v2k_render::UiMapping::new(v2k_render::UiMappingRequest {
        viewport: [width, height],
        authored_canvas: [fonts.virtual_w as u32, fonts.virtual_h as u32],
        policy: renderer.ui_submission_policy(),
    });
    for entity in em.iter_all().filter(|entity| {
        entity.is_network_remote()
            && entity.entity_type == v2k_game::entity::network_body::REMOTE_CRAFT_TYPE
    }) {
        let Some(slot) = net.slot_of_body(entity.id) else {
            continue;
        };
        let slot = usize::from(slot);
        if slot == usize::from(net.local_slot()) || slot >= 9 || !net.scoring.known[slot] {
            continue;
        }
        let dz = v2k_core::world::delta(entity.position[2], camera.position[2]);
        if dz * 256.0 <= 256.0 {
            continue;
        }
        let point = [
            camera.position[0]
                + v2k_core::world::delta(entity.position[0] - 100.0 / 256.0, camera.position[0]),
            entity.position[1] + 128.0 / 256.0,
            camera.position[2] + dz,
        ];
        let projected = v2k_render::project_particle_center(camera, [width, height], point);
        if projected.clip != 0 {
            continue;
        }
        let virtual_point = [
            (projected.screen[0] as f32 - mapping.offset_x as f32) / mapping.scale,
            (projected.screen[1] as f32 - mapping.offset_y as f32) / mapping.scale,
        ];
        crate::menu_text_submission::draw_menu_text(
            renderer,
            fonts.font(true),
            &net.scoring.initials[slot],
            virtual_point,
            false,
            usize::MAX,
            mapping,
        );
    }
}
