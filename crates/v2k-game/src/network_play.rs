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
    NetworkParticleBirth, ParticleEnvironment, TerrainCollisionContext, WorldFx,
};

pub struct NetworkWorldFrame<'a> {
    pub em: &'a mut EntityManager,
    pub world_fx: &'a mut WorldFx,
    pub cache: &'a v2k_game::resource_cache::ResourceCache,
    pub hull: &'a mut PlayerHull,
    pub notifications: &'a mut v2k_game::gameplay_notifications::GameplayNotifications,
    pub extra_lives: u8,
    pub craft: &'a PlayerCraft,
    /// The logical world number the stamp names.
    pub world: u16,
    pub elapsed_micros: u32,
    pub retail_tick: u32,
    /// Peers' bodies the particle pass hit: id, amount, attacker type.
    pub damage: &'a mut Vec<(u32, u16, u16)>,
}

#[derive(Debug, Default)]
pub struct NetworkWorldOutcome {
    pub winner_flash: bool,
    pub abort: bool,
}

fn verbose(message: std::fmt::Arguments) {
    if crate::VERBOSE.load(std::sync::atomic::Ordering::Relaxed) {
        eprintln!("{message}");
    }
}

/// One stop line per kind of record the port holds.
pub(crate) fn report_held(what: String) {
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
        hull,
        notifications,
        extra_lives,
        craft,
        world,
        elapsed_micros,
        retail_tick,
        damage,
    } = frame;
    // Sends the port holds: FUN_00440950's bursts (kind 2) and every
    // FUN_0044F480 sound cue (kinds 22..=25).
    if world_fx.take_network_burst_unsent() {
        report_held("sending FUN_00440950's bursts (kind 2)".into());
    }
    report_held("sending FUN_0044F480's sound cues (kinds 22 to 25)".into());
    // The world owners' own sends: cells (4, 5), static objects (6), impacts
    // on a peer's body (10) and attachments (13 to 15).
    report_held("sending record kinds 4, 5, 6, 10, 13, 14 and 15".into());
    if net.is_host() {
        // FUN_00413500 runs FUN_004132D0 each second while fewer than 32
        // Type61s live: a power-up from the table at 0x004C92D0.
        report_held("the host's power-up spawner (FUN_004132D0)".into());
    }
    // FUN_00440A60's kind-1 sends for this frame's replicated births.
    for birth in world_fx.take_network_particles() {
        let owner = birth.owner_id.and_then(|id| net.handle_of(id)).unwrap_or(0);
        net.send(GameRecord::Particle(particle_request(birth, owner)));
    }
    // FUN_00414E90's kind-9 sends for peers' bodies hit this frame.
    for (id, amount, attacker_type) in damage.drain(..) {
        verbose(format_args!(
            "Network: {amount} damage from a Type{attacker_type} to peer body {id}"
        ));
        net.forward_damage(id, amount, attacker_type);
    }
    let lines_before = net.scoring.lines.clone();
    let scores_before = (net.scoring.table, net.scoring.winner);
    // FUN_00414E90's kill and hit reports for the local player.
    let hits = em.take_network_player_hits();
    for hit in &hits {
        verbose(format_args!("Network: the player took a hit {hit:?}"));
    }
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
        let mut random = || u32::from(world_fx.next_shared_retail_random_u16());
        let mut world_frame = WorldFrame {
            entities: em,
            world,
            world_active: true,
            elapsed_us: elapsed_micros,
            ticks: retail_tick,
            random: &mut random,
            profile: (&name, &initials),
            player,
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
    let mut outcome = NetworkWorldOutcome::default();
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
                    report_held(format!("a peer's damage to an owned Type{entity_type}"));
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
                // FUN_00417EB0: velocity and attitude deltas.
                if let Some(entity) = em.entity_mut(entity_id) {
                    let mut velocity = entity.velocity_raw();
                    for (axis, delta) in velocity.iter_mut().zip(&deltas[..3]) {
                        *axis = axis.wrapping_add(*delta);
                    }
                    entity.velocity = velocity.map(|word| f32::from(word) / 256.0);
                    if deltas[3..] != [0, 0, 0] {
                        report_held("a peer's attitude delta (kind 10)".into());
                    }
                }
            }
            NetworkEvent::Burst(_) => report_held("a peer's burst (kind 2)".into()),
            NetworkEvent::Held { kind } => report_held(format!("record kind {kind}")),
            NetworkEvent::UnknownRestState => {
                report_held("the rest period of a body with unknown state bits".into())
            }
            NetworkEvent::WinnerFlash => {
                verbose(format_args!("Network: the winner flash (FUN_00456750)"));
                outcome.winner_flash = true;
            }
            NetworkEvent::Abort => outcome.abort = true,
        }
    }
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

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
/// view origin's by more than 256.
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
